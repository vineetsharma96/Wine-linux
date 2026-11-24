use std::panic::catch_unwind;
use std::sync::RwLock;
use jni::objects::{JByteArray, JClass, JString};
use jni::sys::{jboolean, jbyteArray, jint, jlong, jstring};
use jni::JNIEnv;

use androidpc_configuration::EngineConfig;
use androidpc_core::{EngineCore, ENGINE_VERSION};
use androidpc_display::{KeyEvent, PointerEvent};
use androidpc_runtime_api::DisplayConfig;
use androidpc_wine::WineArch;

static ENGINE_INSTANCE: RwLock<Option<EngineCore>> = RwLock::new(None);

fn create_jstring<'a>(env: &mut JNIEnv<'a>, value: &str) -> jstring {
    env.new_string(value)
        .map(|s| s.into_raw())
        .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeInitEngine(
    mut env: JNIEnv,
    _class: JClass,
    config_json: JString,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let json_str: String = match env.get_string(&config_json) {
            Ok(s) => s.into(),
            Err(e) => {
                androidpc_logging::push_log(
                    "ERROR",
                    "androidpc::bridge",
                    &format!("Failed to read JNI config string: {e}"),
                );
                return -1;
            }
        };

        let config = if json_str.trim().is_empty() {
            EngineConfig::default()
        } else {
            match EngineConfig::from_json(&json_str) {
                Ok(c) => c,
                Err(e) => {
                    androidpc_logging::push_log(
                        "ERROR",
                        "androidpc::bridge",
                        &format!("Config parsing failed: {e}"),
                    );
                    return -2;
                }
            }
        };

        match EngineCore::new(config) {
            Ok(core) => {
                let mut guard = match ENGINE_INSTANCE.write() {
                    Ok(g) => g,
                    Err(poisoned) => poisoned.into_inner(),
                };
                *guard = Some(core);
                androidpc_logging::push_log(
                    "INFO",
                    "androidpc::bridge",
                    "EngineCore registered in JNI singleton successfully",
                );
                0 // Success
            }
            Err(e) => {
                androidpc_logging::push_log(
                    "ERROR",
                    "androidpc::bridge",
                    &format!("EngineCore creation failed: {e}"),
                );
                -3
            }
        }
    }));

    result.unwrap_or(-99) // -99 indicates caught panic
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeGetEngineVersion(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        create_jstring(&mut env, ENGINE_VERSION)
    }));

    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeGetEngineStatus(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };

        let status_json = match guard.as_ref() {
            Some(core) => {
                let status = core.status();
                serde_json::to_string(&status).unwrap_or_else(|_| "{}".to_string())
            }
            None => {
                // If not yet explicitly initialized, provide uninitialized status
                let uninit = androidpc_runtime_api::EngineStatusInfo {
                    state: "Uninitialized".to_string(),
                    version: ENGINE_VERSION.to_string(),
                    build_profile: if cfg!(debug_assertions) { "debug" } else { "release" }.to_string(),
                    host_arch: androidpc_runtime_api::Architecture::host(),
                    active_environments: 0,
                    uptime_seconds: 0,
                };
                serde_json::to_string(&uninit).unwrap_or_else(|_| "{}".to_string())
            }
        };

        create_jstring(&mut env, &status_json)
    }));

    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeRunHealthCheck(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };

        let report = match guard.as_ref() {
            Some(core) => core.run_health_check(),
            None => {
                let default_cfg = EngineConfig::default();
                androidpc_core::health::execute_health_check(&default_cfg)
            }
        };

        let json = serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string());
        create_jstring(&mut env, &json)
    }));

    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeGetDiagnostics(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };

        let diagnostics = match guard.as_ref() {
            Some(core) => core.collect_diagnostics(),
            None => {
                let default_cfg = EngineConfig::default();
                let health = androidpc_core::health::execute_health_check(&default_cfg);
                androidpc_core::diagnostics::collect_system_diagnostics(&default_cfg, &health)
            }
        };

        let json = serde_json::to_string_pretty(&diagnostics).unwrap_or_else(|_| "{}".to_string());
        create_jstring(&mut env, &json)
    }));

    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeGetRecentLogs(
    mut env: JNIEnv,
    _class: JClass,
    limit: jint,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let safe_limit = if limit <= 0 { 50 } else { limit as usize };
        let logs = androidpc_logging::get_recent_logs(safe_limit);
        let json = serde_json::to_string(&logs).unwrap_or_else(|_| "[]".to_string());
        create_jstring(&mut env, &json)
    }));

    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeVerifyArtifact(
    mut env: JNIEnv,
    _class: JClass,
    path: JString,
    expected_sha256: JString,
) -> jni::sys::jboolean {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let path_str: String = match env.get_string(&path) {
            Ok(s) => s.into(),
            Err(_) => return 0,
        };
        let expected: String = match env.get_string(&expected_sha256) {
            Ok(s) => s.into(),
            Err(_) => return 0,
        };

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            match core.verify_archive(std::path::Path::new(&path_str), &expected) {
                Ok(true) => 1,
                _ => 0,
            }
        } else {
            0
        }
    }));

    result.unwrap_or(0)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeExtractRootfs(
    mut env: JNIEnv,
    _class: JClass,
    archive_path: JString,
    target_dir: JString,
) -> jni::sys::jlong {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let arc_str: String = match env.get_string(&archive_path) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };
        let target_str: String = match env.get_string(&target_dir) {
            Ok(s) => s.into(),
            Err(_) => return -2,
        };

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            match core.extract_rootfs(std::path::Path::new(&arc_str), std::path::Path::new(&target_str)) {
                Ok(count) => count as jni::sys::jlong,
                Err(e) => {
                    androidpc_logging::push_log("ERROR", "androidpc::bridge", &format!("Extract error: {e}"));
                    -3
                }
            }
        } else {
            -4
        }
    }));

    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeStartDisplayServer(
    mut env: JNIEnv,
    _class: JClass,
    display_num: jint,
    width: jint,
    height: jint,
    dpi: jint,
    port: jint,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            let config = DisplayConfig {
                display_type: androidpc_runtime_api::DisplayType::Xvnc,
                width: width as u32,
                height: height as u32,
                dpi: dpi as u32,
                port: port as u16,
                listen_address: "127.0.0.1".to_string(),
                display_number: display_num as u32,
            };

            match core.start_display_server(&config) {
                Ok(endpoint) => {
                    let json = serde_json::to_string(&endpoint).unwrap_or_else(|_| "{}".to_string());
                    create_jstring(&mut env, &json)
                }
                Err(e) => {
                    androidpc_logging::push_log(
                        "ERROR",
                        "androidpc::bridge",
                        &format!("Failed to start display server: {e}"),
                    );
                    create_jstring(&mut env, &format!("{{\"error\":\"{e}\"}}"))
                }
            }
        } else {
            create_jstring(&mut env, "{\"error\":\"Engine not initialized\"}")
        }
    }));

    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeStopDisplayServer(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            match core.stop_display_server() {
                Ok(_) => 0,
                Err(e) => {
                    androidpc_logging::push_log(
                        "ERROR",
                        "androidpc::bridge",
                        &format!("Failed to stop display server: {e}"),
                    );
                    -1
                }
            }
        } else {
            -2
        }
    }));

    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeIsDisplayRunning(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            if core.is_display_running() { 1 } else { 0 }
        } else {
            0
        }
    }));

    result.unwrap_or(0)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeCreateTerminal(
    mut env: JNIEnv,
    _class: JClass,
    session_id: JString,
    cols: jint,
    rows: jint,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let id_str: String = match env.get_string(&session_id) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            match core.create_terminal_session(&id_str, cols as u16, rows as u16) {
                Ok(fd) => fd,
                Err(e) => {
                    androidpc_logging::push_log(
                        "ERROR",
                        "androidpc::bridge",
                        &format!("Failed to create terminal session '{id_str}': {e}"),
                    );
                    -2
                }
            }
        } else {
            -3
        }
    }));

    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeWriteTerminal(
    mut env: JNIEnv,
    _class: JClass,
    session_id: JString,
    data: JByteArray,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let id_str: String = match env.get_string(&session_id) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };

        let bytes = match env.convert_byte_array(&data) {
            Ok(b) => b,
            Err(_) => return -2,
        };

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            match core.write_terminal(&id_str, &bytes) {
                Ok(written) => written as jint,
                Err(e) => {
                    androidpc_logging::push_log(
                        "ERROR",
                        "androidpc::bridge",
                        &format!("Failed to write to terminal '{id_str}': {e}"),
                    );
                    -3
                }
            }
        } else {
            -4
        }
    }));

    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeReadTerminal(
    mut env: JNIEnv,
    _class: JClass,
    session_id: JString,
    max_len: jint,
) -> jbyteArray {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let id_str: String = match env.get_string(&session_id) {
            Ok(s) => s.into(),
            Err(_) => return std::ptr::null_mut(),
        };

        let mut buf = vec![0u8; (max_len.max(1) as usize).min(65536)];

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            match core.read_terminal(&id_str, &mut buf) {
                Ok(n) => {
                    buf.truncate(n);
                    match env.byte_array_from_slice(&buf) {
                        Ok(arr) => arr.into_raw(),
                        Err(_) => std::ptr::null_mut(),
                    }
                }
                Err(_) => std::ptr::null_mut(),
            }
        } else {
            std::ptr::null_mut()
        }
    }));

    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeResizeTerminal(
    mut env: JNIEnv,
    _class: JClass,
    session_id: JString,
    cols: jint,
    rows: jint,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let id_str: String = match env.get_string(&session_id) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            match core.resize_terminal(&id_str, cols as u16, rows as u16) {
                Ok(_) => 0,
                Err(e) => {
                    androidpc_logging::push_log(
                        "ERROR",
                        "androidpc::bridge",
                        &format!("Failed to resize terminal '{id_str}': {e}"),
                    );
                    -2
                }
            }
        } else {
            -3
        }
    }));

    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeCloseTerminal(
    mut env: JNIEnv,
    _class: JClass,
    session_id: JString,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let id_str: String = match env.get_string(&session_id) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            match core.close_terminal(&id_str) {
                Ok(_) => 0,
                Err(e) => {
                    androidpc_logging::push_log(
                        "ERROR",
                        "androidpc::bridge",
                        &format!("Failed to close terminal '{id_str}': {e}"),
                    );
                    -2
                }
            }
        } else {
            -3
        }
    }));

    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeRfbBuildPointerEvent(
    env: JNIEnv,
    _class: JClass,
    button_mask: jint,
    x: jint,
    y: jint,
) -> jbyteArray {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let event = PointerEvent {
            button_mask: button_mask as u8,
            x: x as u16,
            y: y as u16,
        };
        let bytes = event.to_bytes();
        match env.byte_array_from_slice(&bytes) {
            Ok(arr) => arr.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    }));

    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeRfbBuildKeyEvent(
    env: JNIEnv,
    _class: JClass,
    down_flag: jint,
    key_sym: jlong,
) -> jbyteArray {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let event = KeyEvent {
            down_flag: down_flag as u8,
            key: key_sym as u32,
        };
        let bytes = event.to_bytes();
        match env.byte_array_from_slice(&bytes) {
            Ok(arr) => arr.into_raw(),
            Err(_) => std::ptr::null_mut(),
        }
    }));

    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeProvisionEnvironment(
    mut env: JNIEnv,
    _class: JClass,
    env_id: JString,
    distro: JString,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let id_str: String = match env.get_string(&env_id) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };
        let distro_str: String = match env.get_string(&distro) {
            Ok(s) => s.into(),
            Err(_) => return -2,
        };

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            match core.provision_environment(&id_str, &distro_str) {
                Ok(_) => {
                    androidpc_logging::push_log(
                        "INFO",
                        "androidpc::bridge",
                        &format!("Environment '{id_str}' provisioned successfully"),
                    );
                    0
                }
                Err(e) => {
                    androidpc_logging::push_log(
                        "ERROR",
                        "androidpc::bridge",
                        &format!("Provisioning failed for '{id_str}': {e}"),
                    );
                    -3
                }
            }
        } else {
            -4
        }
    }));

    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeExecuteGuestCommand(
    mut env: JNIEnv,
    _class: JClass,
    env_id: JString,
    command_json: JString,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let id_str: String = match env.get_string(&env_id) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };
        let cmd_str: String = match env.get_string(&command_json) {
            Ok(s) => s.into(),
            Err(_) => return -2,
        };

        let args: Vec<String> = serde_json::from_str(&cmd_str).unwrap_or_else(|_| {
            if cmd_str.trim().is_empty() {
                vec![]
            } else {
                vec![cmd_str]
            }
        });

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            match core.execute_environment_command(&id_str, &args) {
                Ok(pid) => pid as jint,
                Err(e) => {
                    androidpc_logging::push_log(
                        "ERROR",
                        "androidpc::bridge",
                        &format!("Failed to execute command in '{id_str}': {e}"),
                    );
                    -3
                }
            }
        } else {
            -4
        }
    }));

    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeStopEnvironment(
    mut env: JNIEnv,
    _class: JClass,
    env_id: JString,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let id_str: String = match env.get_string(&env_id) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            match core.stop_environment(&id_str) {
                Ok(_) => 0,
                Err(e) => {
                    androidpc_logging::push_log(
                        "ERROR",
                        "androidpc::bridge",
                        &format!("Failed to stop environment '{id_str}': {e}"),
                    );
                    -2
                }
            }
        } else {
            -3
        }
    }));

    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeGetEnvironmentState(
    mut env: JNIEnv,
    _class: JClass,
    env_id: JString,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let id_str: String = match env.get_string(&env_id) {
            Ok(s) => s.into(),
            Err(_) => return create_jstring(&mut env, "Unknown"),
        };

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            match core.get_environment_state(&id_str) {
                Some(state) => create_jstring(&mut env, &format!("{:?}", state)),
                None => create_jstring(&mut env, "NotFound"),
            }
        } else {
            create_jstring(&mut env, "EngineNotInitialized")
        }
    }));

    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeListEnvironments(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            let envs = core.list_environments();
            let json = serde_json::to_string(&envs).unwrap_or_else(|_| "[]".to_string());
            create_jstring(&mut env, &json)
        } else {
            create_jstring(&mut env, "[]")
        }
    }));

    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeGetArtifactInfo(
    mut env: JNIEnv,
    _class: JClass,
    distro: JString,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let distro_str: String = match env.get_string(&distro) {
            Ok(s) => s.into(),
            Err(_) => return create_jstring(&mut env, "{}"),
        };

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            match core.get_artifact_metadata(&distro_str) {
                Some(meta) => {
                    let json = serde_json::to_string(&meta).unwrap_or_else(|_| "{}".to_string());
                    create_jstring(&mut env, &json)
                }
                None => create_jstring(&mut env, "{}"),
            }
        } else {
            create_jstring(&mut env, "{}")
        }
    }));

    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeConnectDisplayClient(
    mut env: JNIEnv,
    _class: JClass,
    host: JString,
    port: jint,
) -> jlong {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let host_str: String = match env.get_string(&host) {
            Ok(s) => s.into(),
            Err(_) => "127.0.0.1".to_string(),
        };

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            match core.connect_display_client(&host_str, port as u16) {
                Ok((w, h)) => ((w as jlong) << 32) | ((h as jlong) & 0xffffffff),
                Err(e) => {
                    androidpc_logging::push_log(
                        "ERROR",
                        "androidpc::bridge",
                        &format!("Failed to connect display client: {e}"),
                    );
                    0
                }
            }
        } else {
            0
        }
    }));

    result.unwrap_or(0)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeGetFramebuffer(
    mut env: JNIEnv,
    _class: JClass,
    pixels_out: JByteArray,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let array_len = match env.get_array_length(&pixels_out) {
            Ok(len) => len as usize,
            Err(_) => return -1,
        };

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            let mut temp_buf = vec![0u8; array_len];
            match core.get_framebuffer_slice(&mut temp_buf) {
                Ok(copied) => {
                    let slice_i8: &[i8] = unsafe {
                        std::slice::from_raw_parts(temp_buf[..copied].as_ptr() as *const i8, copied)
                    };
                    if env.set_byte_array_region(&pixels_out, 0, slice_i8).is_ok() {
                        copied as jint
                    } else {
                        -2
                    }
                }
                Err(_) => -3,
            }
        } else {
            -4
        }
    }));

    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeSendPointer(
    _env: JNIEnv,
    _class: JClass,
    button_mask: jint,
    x: jint,
    y: jint,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            match core.send_display_pointer(button_mask as u8, x as u16, y as u16) {
                Ok(_) => 0,
                Err(_) => -1,
            }
        } else {
            -2
        }
    }));

    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeSendKey(
    _env: JNIEnv,
    _class: JClass,
    down_flag: jint,
    key_sym: jlong,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            match core.send_display_key(down_flag as u8, key_sym as u32) {
                Ok(_) => 0,
                Err(_) => -1,
            }
        } else {
            -2
        }
    }));

    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeDisconnectDisplayClient(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            match core.disconnect_display_client() {
                Ok(_) => 0,
                Err(_) => -1,
            }
        } else {
            -2
        }
    }));

    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeInitWinePrefix(
    mut env: JNIEnv,
    _class: JClass,
    env_id: JString,
    prefix_path: JString,
    arch_str: JString,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let env_id_str: String = match env.get_string(&env_id) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };
        let prefix_str: String = match env.get_string(&prefix_path) {
            Ok(s) => s.into(),
            Err(_) => "".to_string(),
        };
        let arch_val: String = match env.get_string(&arch_str) {
            Ok(s) => s.into(),
            Err(_) => "win64".to_string(),
        };

        let arch = match arch_val.to_lowercase().as_str() {
            "win32" => WineArch::Win32,
            "win64" => WineArch::Win64,
            _ => WineArch::WoW64,
        };

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            match core.init_wine_prefix(&env_id_str, &prefix_str, arch) {
                Ok(_) => 0,
                Err(e) => {
                    androidpc_logging::push_log(
                        "ERROR",
                        "androidpc::bridge",
                        &format!("Failed to init wine prefix: {e}"),
                    );
                    -2
                }
            }
        } else {
            -3
        }
    }));

    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeBuildWineCommand(
    mut env: JNIEnv,
    _class: JClass,
    env_id: JString,
    exe_path: JString,
    args_json: JString,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let env_id_str: String = match env.get_string(&env_id) {
            Ok(s) => s.into(),
            Err(_) => return std::ptr::null_mut(),
        };
        let exe_str: String = match env.get_string(&exe_path) {
            Ok(s) => s.into(),
            Err(_) => return std::ptr::null_mut(),
        };
        let args_raw: String = match env.get_string(&args_json) {
            Ok(s) => s.into(),
            Err(_) => "[]".to_string(),
        };

        let args: Vec<String> = serde_json::from_str(&args_raw).unwrap_or_default();

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            match core.build_wine_launch_command(&env_id_str, &exe_str, &args) {
                Ok(cmd) => {
                    let json = serde_json::to_string(&cmd).unwrap_or_else(|_| "{}".to_string());
                    create_jstring(&mut env, &json)
                }
                Err(e) => {
                    androidpc_logging::push_log(
                        "ERROR",
                        "androidpc::bridge",
                        &format!("Failed to build wine launch command: {e}"),
                    );
                    std::ptr::null_mut()
                }
            }
        } else {
            std::ptr::null_mut()
        }
    }));

    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeKillWineSession(
    mut env: JNIEnv,
    _class: JClass,
    env_id: JString,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let env_id_str: String = match env.get_string(&env_id) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            match core.terminate_wine_session(&env_id_str) {
                Ok(_) => 0,
                Err(e) => {
                    androidpc_logging::push_log(
                        "ERROR",
                        "androidpc::bridge",
                        &format!("Failed to kill wine session: {e}"),
                    );
                    -2
                }
            }
        } else {
            -3
        }
    }));

    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeGetWineStatus(
    mut env: JNIEnv,
    _class: JClass,
    env_id: JString,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let env_id_str: String = match env.get_string(&env_id) {
            Ok(s) => s.into(),
            Err(_) => return std::ptr::null_mut(),
        };

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            match core.get_wine_status(&env_id_str) {
                Ok(status) => {
                    let json = serde_json::to_string(&status).unwrap_or_else(|_| "{}".to_string());
                    create_jstring(&mut env, &json)
                }
                Err(_) => create_jstring(&mut env, "{}"),
            }
        } else {
            std::ptr::null_mut()
        }
    }));

    result.unwrap_or(std::ptr::null_mut())
}

