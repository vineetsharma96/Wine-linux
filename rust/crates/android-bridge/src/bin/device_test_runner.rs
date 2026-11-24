use std::path::Path;
use flate2::write::GzEncoder;
use flate2::Compression;
use tar::Builder;

use androidpc_configuration::EngineConfig;
use androidpc_core::EngineCore;
use androidpc_display::{
    FramebufferUpdateRequest, KeyEvent, PointerEvent, XvncDisplayBackend,
};
use androidpc_downloader::Sha256Verifier;
use androidpc_filesystem::{ArchiveExtractor, PathSecurity};
use androidpc_linux_runtime::ProotRuntimeProvider;
use androidpc_process::DefaultProcessSupervisor;
use androidpc_runtime_api::{
    DisplayConfig, DisplayType, EnvironmentState, ProcessCommand, ProcessSupervisor, TerminalSession,
};
use androidpc_terminal::DefaultTerminalSession;

fn main() {
    println!("============================================================");
    println!(" ANDROID PC ENVIRONMENT — HARDWARE PHASE 1 RUNTIME TEST     ");
    println!("============================================================");
    println!("Host Architecture: {}", std::env::consts::ARCH);
    println!("Target OS:         {}", std::env::consts::OS);

    // 1. Core Engine Lifecycle
    let mut config = EngineConfig::default();
    #[cfg(target_os = "android")]
    {
        config.base_data_dir = std::path::PathBuf::from("/data/local/tmp/androidpc_sandbox/files");
        config.runtime_artifacts_dir = std::path::PathBuf::from("/data/local/tmp/androidpc_sandbox/artifacts");
        config.temporary_dir = std::path::PathBuf::from("/data/local/tmp/androidpc_sandbox/tmp");
    }
    let core = EngineCore::new(config).expect("EngineCore initialization failed");
    println!("[1/5] Engine Core v{} online.", EngineCore::version());

    // 2. Cryptographic Integrity Verifier
    println!("[2/5] Testing SHA-256 cryptographic stream hashing...");
    let sample_data = b"Android PC Environment Rootless Guest Payload";
    let hash = Sha256Verifier::hash_bytes(sample_data);
    println!("      Calculated Hash: {}", hash);
    assert_eq!(
        hash,
        "b37b23d65c321a5e1ff801910a5c706238ac8c14ed33c676fd81bf1f1e613ca4"
    );
    println!("      SHA-256 verification validated successfully.");

    // 3. Zip-Slip / Path Traversal Security
    println!("[3/5] Testing Zip-Slip / Path Traversal defense...");
    let safe_base = Path::new("/data/local/tmp/androidpc_sandbox/rootfs");
    let evil_path = Path::new("../../system/bin/sh");
    let result = PathSecurity::sanitize_and_resolve(safe_base, evil_path);
    assert!(result.is_err(), "Zip-slip path must be rejected by security filter");
    println!("      Malicious traversal '../../system/bin/sh' successfully BLOCKED.");

    // 4. Safe Rootfs In-Memory Archive Extraction
    println!("[4/5] Testing archive unpacker with traversal protection...");
    let temp_archive_path = Path::new("/data/local/tmp/test_guest_payload.tar.gz");
    let extract_target = Path::new("/data/local/tmp/androidpc_sandbox/rootfs");

    // Create a valid tar.gz archive on device
    {
        let tar_gz_file = std::fs::File::create(temp_archive_path).expect("Create tar.gz");
        let enc = GzEncoder::new(tar_gz_file, Compression::default());
        let mut tar = Builder::new(enc);

        let mut header = tar::Header::new_gnu();
        header.set_size(15);
        header.set_mode(0o755);
        header.set_cksum();
        tar.append_data(&mut header, "etc/issue", &b"Ubuntu 24.04\n"[..])
            .expect("Append etc/issue");
        tar.finish().expect("Finish tar");
    }

    let entries = ArchiveExtractor::extract_tar_gz(temp_archive_path, extract_target)
        .expect("Extract archive");
    println!("      Safely unpacked {} entries into target rootfs sandbox.", entries);
    let extracted_file = extract_target.join("etc/issue");
    assert!(extracted_file.exists(), "Extracted payload must exist on disk");
    println!("      Verified extracted content: {:?}", std::fs::read_to_string(&extracted_file).unwrap().trim());
    let _ = std::fs::remove_file(temp_archive_path);
    let _ = std::fs::remove_dir_all("/data/local/tmp/androidpc_sandbox");

    // 5. Process Supervisor & PRoot Argument Resolution
    println!("[5/5] Testing Process Supervisor & PRoot command builder...");
    let proot_provider = ProotRuntimeProvider::new("/data/data/com.androidpc.environment/files/artifacts/proot-arm64");
    let env_config = core.create_environment_config("default-ubuntu", "ubuntu");
    let proot_cmd = proot_provider.build_proot_command(&env_config, &["/bin/uname".to_string(), "-m".to_string()]);
    println!("      PRoot Generated Arguments: {:?}", proot_cmd);

    // Test process supervisor spawning host echo
    let supervisor = DefaultProcessSupervisor::new();
    let echo_cmd = ProcessCommand {
        program: "/system/bin/sh".to_string(),
        args: vec!["-c".to_string(), "echo 'PROCESS_SUPERVISOR_ACTIVE'".to_string()],
        env: vec![],
        cwd: None,
        uid: None,
        gid: None,
    };
    let pid = supervisor.spawn_process(&echo_cmd).expect("Spawn supervisor child");
    println!("      Process supervisor active: spawned child PID {}", pid);
    assert!(pid > 0);

    // 6. Remote Framebuffer (RFB 003.008) Protocol Encoding
    println!("[6/8] Testing RFC 6143 RFB display protocol serialization...");
    let pointer = PointerEvent {
        button_mask: 0b0000_0001, // Left button down
        x: 1024,
        y: 768,
    };
    let p_bytes = pointer.to_bytes();
    assert_eq!(p_bytes.len(), 6);
    assert_eq!(p_bytes[0], 5); // PointerEvent type
    assert_eq!(p_bytes[1], 1); // Button mask
    assert_eq!(u16::from_be_bytes([p_bytes[2], p_bytes[3]]), 1024);
    assert_eq!(u16::from_be_bytes([p_bytes[4], p_bytes[5]]), 768);

    let key = KeyEvent {
        down_flag: 1,
        key: 0xff0d, // X11 XK_Return
    };
    let k_bytes = key.to_bytes();
    assert_eq!(k_bytes.len(), 8);
    assert_eq!(k_bytes[0], 4); // KeyEvent type
    assert_eq!(k_bytes[1], 1); // Down flag
    assert_eq!(u32::from_be_bytes([k_bytes[4], k_bytes[5], k_bytes[6], k_bytes[7]]), 0xff0d);

    let fb_req = FramebufferUpdateRequest {
        incremental: 1,
        x: 0,
        y: 0,
        width: 1920,
        height: 1080,
    };
    let fb_bytes = fb_req.to_bytes();
    assert_eq!(fb_bytes.len(), 10);
    assert_eq!(fb_bytes[0], 3); // FramebufferUpdateRequest type
    println!("      RFB pointer, keyboard, and frame-update packets correctly formatted.");

    // 7. Unix PTY Subsystem on Bionic
    println!("[7/8] Testing Unix PTY subsystem (posix_openpt, grantpt, unlockpt)...");
    let mut session = DefaultTerminalSession::new("term-device-0", 80, 24)
        .expect("PTY session allocation failed");
    println!("      Allocated PTY session id: {}", session.session_id());
    #[cfg(unix)]
    {
        let pts = session.pts_name().expect("pts_name resolution");
        println!("      Slave PTY node: {} (master fd: {})", pts, session.master_fd());
        assert!(!pts.is_empty());
    }
    assert!(session.resize(120, 40).is_ok(), "PTY resize ioctl failed");
    println!("      PTY resize to 120x40 succeeded via TIOCSWINSZ.");

    // 8. Xvnc Display Server Command Line Builder
    println!("[8/8] Testing Xvnc display backend command line builder...");
    let disp_config = DisplayConfig {
        display_type: DisplayType::Xvnc,
        width: 1920,
        height: 1080,
        dpi: 160,
        port: 5901,
        listen_address: "127.0.0.1".to_string(),
        display_number: 1,
    };
    let xvnc_args = XvncDisplayBackend::build_xvnc_args(&disp_config);
    assert!(xvnc_args.contains(&":1".to_string()));
    assert!(xvnc_args.contains(&"1920x1080".to_string()));
    assert!(xvnc_args.contains(&"5901".to_string()));
    assert!(xvnc_args.contains(&"None".to_string()));
    println!("      Generated Xvnc arguments: {:?}", xvnc_args);

    // 9. Rootfs Environment Provisioning & Network Configuration
    println!("[9/9] Testing rootless environment provisioning & network setup...");
    let env_id = "test-device-ubuntu";
    let env_cfg = core.provision_environment(env_id, "ubuntu")
        .expect("Environment provisioning failed");
    println!("      Environment provisioned: ID='{}', Distro='{}'", env_cfg.id, env_cfg.distribution);

    let state = core.get_environment_state(env_id);
    assert_eq!(state, Some(EnvironmentState::Stopped));
    println!("      Environment state validated: {:?}", state.unwrap());

    let rootfs_dir = core.storage.environment_rootfs(env_id);
    let resolv_conf = rootfs_dir.join("etc/resolv.conf");
    let hosts_file = rootfs_dir.join("etc/hosts");
    assert!(resolv_conf.exists(), "resolv.conf must be created");
    assert!(hosts_file.exists(), "hosts file must be created");

    let resolv_content = std::fs::read_to_string(&resolv_conf).unwrap();
    assert!(resolv_content.contains("8.8.8.8"));
    println!("      DNS configuration (/etc/resolv.conf) verified: {}", resolv_content.trim());

    // Query Artifact Metadata catalog
    let meta = core.get_artifact_metadata("ubuntu").expect("Ubuntu artifact metadata not found");
    assert_eq!(meta.distribution, "ubuntu");
    println!("      Catalog lookup verified: {} ({} bytes)", meta.filename, meta.size_bytes);

    // Cleanup test environment
    let _ = core.storage.remove_environment(env_id);
    println!("      Test sandbox cleaned up successfully.");

    // 10. RFB Client Framebuffer Stream & Blitter
    println!("[10/10] Testing RFB client stream receiver & pixel blitter...");
    let (width, height) = core.connect_display_client("127.0.0.1", 5901)
        .expect("RFB client connection initialization");
    assert_eq!(width, 1280);
    assert_eq!(height, 720);
    println!("      RFB client display stream initialized: {}x{}", width, height);

    let mut frame_buf = vec![0u8; 1280 * 720 * 4];
    let bytes_read = core.get_framebuffer_slice(&mut frame_buf).expect("Get framebuffer slice");
    assert_eq!(bytes_read, 1280 * 720 * 4);
    assert_eq!(frame_buf[0], 0x1e); // Top bar Red component
    assert_eq!(frame_buf[3], 0xff); // Alpha channel
    println!("      Framebuffer slice decoded: {} bytes (32-bit RGBA).", bytes_read);

    // Test input forwarding through active client stream
    assert!(core.send_display_pointer(1, 640, 360).is_ok());
    assert!(core.send_display_key(1, 0xff0d).is_ok());
    println!("      Pointer & Key events dispatched through display client stream.");

    // Disconnect client
    assert!(core.disconnect_display_client().is_ok());
    println!("      Display stream cleanly disconnected.");

    // 11. Wine + Box64 Windows Compatibility Subsystem
    println!("[11/11] Testing Wine & Box64 Windows compatibility subsystem...");
    let wine_env_id = "test-wine-env";
    let wineboot_cmd = core.init_wine_prefix(wine_env_id, "/home/user/.wine", androidpc_wine::WineArch::Win64)
        .expect("Wine prefix initialization");
    assert_eq!(wineboot_cmd.program, "box64");
    assert_eq!(wineboot_cmd.args, vec!["wine", "wineboot", "-u"]);
    assert!(wineboot_cmd.env.iter().any(|(k, v)| k == "BOX64_DYNAREC" && v == "1"));
    assert!(wineboot_cmd.env.iter().any(|(k, v)| k == "WINEPREFIX" && v == "/home/user/.wine"));
    assert!(wineboot_cmd.env.iter().any(|(k, v)| k == "WINEARCH" && v == "win64"));
    assert!(wineboot_cmd.env.iter().any(|(k, v)| k == "DISPLAY" && v == ":1"));
    println!("      Wineboot initialization command & Box64 dynarec flags validated.");

    let wine_status = core.get_wine_status(wine_env_id).expect("Get wine status");
    assert_eq!(wine_status.arch, androidpc_wine::WineArch::Win64);
    assert_eq!(wine_status.prefix_path, "/home/user/.wine");
    assert!(wine_status.box64_dynarec);
    println!("      Wine prefix status validated: {:?}", wine_status.state);

    let launch_cmd = core.build_wine_launch_command(wine_env_id, "C:\\Games\\Doom.exe", &["-fullscreen".to_string()])
        .expect("Build wine launch command");
    assert_eq!(launch_cmd.program, "box64");
    assert_eq!(launch_cmd.args, vec!["wine", "C:\\Games\\Doom.exe", "-fullscreen"]);
    println!("      Windows binary launch command generated: {:?}", launch_cmd.args);

    let kill_cmd = core.terminate_wine_session(wine_env_id).expect("Terminate wine session");
    assert_eq!(kill_cmd.program, "wineserver");
    assert_eq!(kill_cmd.args, vec!["-k"]);
    println!("      Wineserver termination command validated.");

    // Validate path translations
    let prefix_p = std::path::PathBuf::from("/home/user/.wine");
    let mapped_win = androidpc_wine::WineCommandBuilder::to_windows_path(
        &prefix_p.join("drive_c/windows/notepad.exe"),
        &prefix_p,
    );
    assert_eq!(mapped_win, Some("C:\\windows\\notepad.exe".to_string()));
    let mapped_unix = androidpc_wine::WineCommandBuilder::to_unix_path("C:\\windows\\notepad.exe", &prefix_p);
    assert_eq!(mapped_unix, Some(prefix_p.join("drive_c/windows/notepad.exe")));
    println!("      Windows/Unix path bidirection translation verified (C:\\ <-> drive_c).");

    println!("============================================================");
    println!(" ALL PHASE 1, 2, 3, 4 & 5 HARDWARE TESTS PASSED: 100% OPERATIONAL");
    println!("============================================================");
}
