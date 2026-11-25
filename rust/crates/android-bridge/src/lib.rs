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

pub mod native_display;
pub mod vm_jni;
pub mod windows_jni;

pub(crate) static ENGINE_INSTANCE: RwLock<Option<EngineCore>> = RwLock::new(None);

pub(crate) fn create_jstring<'a>(env: &mut JNIEnv<'a>, value: &str) -> jstring {
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

        let mut guard = match ENGINE_INSTANCE.write() {
            Ok(g) => g,
            Err(poisoned) => poisoned.into_inner(),
        };
        if guard.is_some() {
            return 0;
        }
        match EngineCore::new(config) {
            Ok(core) => {
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
                androidpc_core::diagnostics::collect_system_diagnostics(&default_cfg, &health, 0)
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
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeStartDesktopSession(
    mut env: JNIEnv,
    _class: JClass,
    env_id: JString,
    display_num: jint,
    width: jint,
    height: jint,
    dpi: jint,
    port: jint,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let env_str: String = match env.get_string(&env_id) {
            Ok(s) => s.into(),
            Err(_) => String::new(),
        };
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            match core.start_desktop_session(
                &env_str,
                display_num as u32,
                width as u32,
                height as u32,
                dpi as u32,
                port as u16,
            ) {
                Ok(endpoint) => {
                    let json = serde_json::to_string(&endpoint).unwrap_or_else(|_| "{}".to_string());
                    create_jstring(&mut env, &json)
                }
                Err(e) => {
                    androidpc_logging::push_log(
                        "ERROR",
                        "androidpc::bridge",
                        &format!("Failed to start desktop session: {e}"),
                    );
                    create_jstring(&mut env, &serde_json::json!({ "error": e.to_string() }).to_string())
                }
            }
        } else {
            create_jstring(&mut env, "{\"error\":\"Engine not initialized\"}")
        }
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeStopDesktopSession(
    mut env: JNIEnv,
    _class: JClass,
    session_id: JString,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let session_id: String = match env.get_string(&session_id) {
            Ok(value) => value.into(),
            Err(_) => return -1,
        };
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            match core.stop_desktop_session(Some(&session_id)) {
                Ok(_) => 0,
                Err(e) => {
                    androidpc_logging::push_log(
                        "ERROR",
                        "androidpc::bridge",
                        &format!("Failed to stop desktop session: {e}"),
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
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeHasLiveDisplayConnection(
    _env: JNIEnv,
    _class: JClass,
) -> jni::sys::jboolean {
    ENGINE_INSTANCE.read().ok().and_then(|engine| {
        engine.as_ref().map(|core| core.has_live_display_connection())
    }).unwrap_or(false) as jni::sys::jboolean
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeGetDesktopStatus(
    mut env: JNIEnv,
    _class: JClass,
    env_id: JString,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let env_str: String = match env.get_string(&env_id) {
            Ok(s) => s.into(),
            Err(_) => String::new(),
        };
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            match core.desktop_status(&env_str) {
                Ok(status) => {
                    let json = serde_json::to_string(&status).unwrap_or_else(|_| "{}".to_string());
                    create_jstring(&mut env, &json)
                }
                Err(e) => create_jstring(
                    &mut env,
                    &serde_json::json!({ "error": e.to_string() }).to_string(),
                ),
            }
        } else {
            create_jstring(&mut env, "{\"error\":\"Engine not initialized\"}")
        }
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeResizeDesktopSession(
    _env: JNIEnv,
    _class: JClass,
    width: jint,
    height: jint,
) -> jlong {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            match core.resize_desktop_session(width as u32, height as u32) {
                Ok((w, h)) => ((w as jlong) << 32) | ((h as jlong) & 0xffffffff),
                Err(_) => 0,
            }
        } else {
            0
        }
    }));
    result.unwrap_or(0)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeSendDisplayClipboard(
    mut env: JNIEnv,
    _class: JClass,
    text: JString,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let text_str: String = match env.get_string(&text) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            if core.send_display_clipboard(&text_str).is_ok() { 0 } else { -1 }
        } else {
            -2
        }
    }));
    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeTakeDisplayClipboard(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            if let Ok(Some(text)) = core.take_display_clipboard() {
                return create_jstring(&mut env, &text);
            }
        }
        std::ptr::null_mut()
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeGetDisplayDimensions(
    _env: JNIEnv,
    _class: JClass,
) -> jlong {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            let (w, h) = core.get_display_dimensions();
            ((w as jlong) << 32) | ((h as jlong) & 0xffffffff)
        } else {
            0
        }
    }));
    result.unwrap_or(0)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeInstallDesktopProfile(
    mut env: JNIEnv,
    _class: JClass,
    env_id: JString,
    profile: JString,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let env_str: String = match env.get_string(&env_id) {
            Ok(s) => s.into(),
            Err(_) => String::new(),
        };
        let profile_str: String = match env.get_string(&profile) {
            Ok(s) => s.into(),
            Err(_) => "minimal".to_string(),
        };
        let Some(kind) = androidpc_display::DesktopProfileKind::from_str(&profile_str) else {
            return create_jstring(&mut env, "{\"error\":\"Unknown desktop profile\"}");
        };
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            match core.install_desktop_profile(&env_str, kind) {
                Ok(res) => {
                    let json = serde_json::to_string(&res).unwrap_or_else(|_| "{}".to_string());
                    create_jstring(&mut env, &json)
                }
                Err(e) => create_jstring(
                    &mut env,
                    &serde_json::json!({ "error": e.to_string() }).to_string(),
                ),
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
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeTerminalExitStatus(
    mut env: JNIEnv,
    _class: JClass,
    session_id: JString,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Ok(id) = env.get_string(&session_id) else { return -2 };
        let id: String = id.into();
        let Ok(engine) = ENGINE_INSTANCE.read() else { return -2 };
        match engine.as_ref().map(|core| core.terminal_exit_status(&id)) {
            Some(Ok(status)) => status.unwrap_or(-1),
            _ => -2,
        }
    }));
    result.unwrap_or(-2)
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
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeGetFramebufferDirect(
    env: JNIEnv,
    _class: JClass,
    direct_buf: jni::objects::JByteBuffer,
    last_gen: jlong,
) -> jlong {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let ptr = match env.get_direct_buffer_address(&direct_buf) {
            Ok(p) if !p.is_null() => p,
            _ => return -1,
        };
        let cap = match env.get_direct_buffer_capacity(&direct_buf) {
            Ok(c) if c > 0 => c,
            _ => return -1,
        };

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };

        if let Some(core) = guard.as_ref() {
            let out_slice = unsafe { std::slice::from_raw_parts_mut(ptr, cap) };
            match core.get_framebuffer_if_dirty(out_slice, last_gen as u64) {
                Ok(new_gen) => new_gen as jlong,
                Err(_) => -3,
            }
        } else {
            -4
        }
    }));

    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeGetFramebuffer(
    env: JNIEnv,
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

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeStartEnvironment(
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
            match core.start_environment(&id_str) {
                Ok(_) => 0,
                Err(e) => {
                    androidpc_logging::push_log("ERROR", "androidpc::bridge", &format!("Start env '{id_str}' failed: {e}"));
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
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeDeleteEnvironment(
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
            match core.delete_environment(&id_str) {
                Ok(_) => 0,
                Err(e) => {
                    androidpc_logging::push_log("ERROR", "androidpc::bridge", &format!("Delete env '{id_str}' failed: {e}"));
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
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeVerifyAndInstallArtifact(
    mut env: JNIEnv,
    _class: JClass,
    part_path: JString,
    dest_filename: JString,
    expected_sha256: JString,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let part_str: String = match env.get_string(&part_path) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };
        let dest_str: String = match env.get_string(&dest_filename) {
            Ok(s) => s.into(),
            Err(_) => return -2,
        };
        let sha_str: String = match env.get_string(&expected_sha256) {
            Ok(s) => s.into(),
            Err(_) => "".to_string(),
        };
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            match core.verify_and_install_artifact(std::path::Path::new(&part_str), &dest_str, &sha_str) {
                Ok(_) => 0,
                Err(e) => {
                    androidpc_logging::push_log("ERROR", "androidpc::bridge", &format!("Artifact install failed: {e}"));
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
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeListArtifacts(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            let list = core.list_artifacts_for_host();
            let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
            create_jstring(&mut env, &json)
        } else {
            create_jstring(&mut env, "[]")
        }
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeStartAudioSink(
    _env: JNIEnv,
    _class: JClass,
    port: jint,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            let p = if port <= 0 { 4713 } else { port as u16 };
            match core.start_audio_sink(p) {
                Ok(bound) => bound as jint,
                Err(e) => {
                    androidpc_logging::push_log("ERROR", "androidpc::bridge", &format!("Audio sink start failed: {e}"));
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
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeStopAudioSink(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            match core.stop_audio_sink() {
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
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeReadAudioPcm(
    env: JNIEnv,
    _class: JClass,
    pcm_out: JByteArray,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let len = match env.get_array_length(&pcm_out) {
            Ok(l) => l.max(0) as usize,
            Err(_) => return -1,
        };
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            let mut buf = vec![0u8; len];
            match core.read_audio_pcm(&mut buf) {
                Ok(read) => {
                    if read > 0 {
                        let slice_i8: &[i8] = unsafe {
                            std::slice::from_raw_parts(buf[..read].as_ptr() as *const i8, read)
                        };
                        if env.set_byte_array_region(&pcm_out, 0, slice_i8).is_err() {
                            return -2;
                        }
                    }
                    read as jint
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
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeGetAudioStatus(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            match core.get_audio_status() {
                Ok(status) => {
                    let json = serde_json::to_string(&status).unwrap_or_else(|_| "{}".to_string());
                    create_jstring(&mut env, &json)
                }
                Err(_) => create_jstring(&mut env, "{}"),
            }
        } else {
            create_jstring(&mut env, "{}")
        }
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeLaunchWineApp(
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
            match core.launch_wine_app(&env_id_str, &exe_str, &args) {
                Ok((cmd, pid)) => {
                    let payload = serde_json::json!({
                        "program": cmd.program,
                        "args": cmd.args,
                        "cwd": cmd.cwd,
                        "spawned_pid": pid,
                    });
                    create_jstring(&mut env, &payload.to_string())
                }
                Err(e) => {
                    androidpc_logging::push_log("ERROR", "androidpc::bridge", &format!("Launch wine app failed: {e}"));
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
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeProvisionEnvironmentWithMemory(
    mut env: JNIEnv,
    _class: JClass,
    env_id: JString,
    distro: JString,
    memory_mb: jint,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let env_id_str: String = match env.get_string(&env_id) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };
        let distro_str: String = match env.get_string(&distro) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            let mem = (memory_mb.max(512)) as u32;
            match core.provision_environment_with_memory(&env_id_str, &distro_str, mem) {
                Ok(_) => 0,
                Err(e) => {
                    androidpc_logging::push_log("ERROR", "androidpc::bridge", &format!("Provision failed: {e}"));
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
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeBackupOperation(
    mut env: JNIEnv,
    _class: JClass,
    operation: JString,
    source: JString,
    backup: JString,
    target: JString,
    name: JString,
) -> jstring {
    catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut run = || -> Result<serde_json::Value, String> {
            let operation: String = env.get_string(&operation).map_err(|e| e.to_string())?.into();
            let source: String = env.get_string(&source).map_err(|e| e.to_string())?.into();
            let backup: String = env.get_string(&backup).map_err(|e| e.to_string())?.into();
            let target: String = env.get_string(&target).map_err(|e| e.to_string())?.into();
            let name: String = env.get_string(&name).map_err(|e| e.to_string())?.into();
            let guard = ENGINE_INSTANCE.read().map_err(|_| "Engine lock unavailable")?;
            let core = guard.as_ref().ok_or("Engine not initialized")?;
            match operation.as_str() {
                "list" => core.list_backups(&source).map(|items| serde_json::json!({"backups": items})),
                "create" => core.create_backup(&source, &backup).map(|item| serde_json::json!({"backups": [item]})),
                "restore" => core.restore_backup(&backup, &target, &name).map(|item| serde_json::json!({"environment_id": item.id})),
                "clone" => core.clone_environment(&source, &target, &name, &backup).map(|item| serde_json::json!({"environment_id": item.id})),
                _ => return Err("Unknown backup operation".into()),
            }.map_err(|error| error.to_string())
        };
        let value = run().unwrap_or_else(|error| serde_json::json!({"error": error}));
        create_jstring(&mut env, &value.to_string())
    })).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeRunPackageOperation(
    mut env: JNIEnv,
    _class: JClass,
    id: JString,
    action: JString,
    query: JString,
) -> jstring {
    catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut run = || -> Result<String, String> {
            let id: String = env.get_string(&id).map_err(|e| e.to_string())?.into();
            let action: String = env.get_string(&action).map_err(|e| e.to_string())?.into();
            let query: String = env.get_string(&query).map_err(|e| e.to_string())?.into();
            let guard = ENGINE_INSTANCE.read().map_err(|_| "Engine lock unavailable")?;
            let result = guard.as_ref().ok_or("Engine not initialized")?
                .run_package_operation(&id, &action, &query).map_err(|e| e.to_string())?;
            serde_json::to_string(&result).map_err(|e| e.to_string())
        };
        let json = run().unwrap_or_else(|error| serde_json::json!({"error": error}).to_string());
        create_jstring(&mut env, &json)
    })).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativePackageOperationProgress(
    mut env: JNIEnv,
    _class: JClass,
    id: JString,
) -> jstring {
    catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Ok(id) = env.get_string(&id) else { return std::ptr::null_mut() };
        let id: String = id.into();
        let Ok(guard) = ENGINE_INSTANCE.read() else { return std::ptr::null_mut() };
        let Some(progress) = guard.as_ref().and_then(|engine| engine.package_operation_progress(&id)) else {
            return std::ptr::null_mut();
        };
        create_jstring(&mut env, &serde_json::to_string(&progress).unwrap_or_default())
    })).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeCancelPackageOperation(
    mut env: JNIEnv,
    _class: JClass,
    id: JString,
) -> jboolean {
    catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Ok(id) = env.get_string(&id) else { return false };
        let id: String = id.into();
        let Ok(guard) = ENGINE_INSTANCE.read() else { return false };
        guard.as_ref().is_some_and(|engine| engine.cancel_package_operation(&id))
    })).unwrap_or(false).into()
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeEnvironmentMetrics(
    mut env: JNIEnv,
    _class: JClass,
    id: JString,
) -> jstring {
    catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Ok(id) = env.get_string(&id) else { return std::ptr::null_mut() };
        let id: String = id.into();
        let Ok(engine) = ENGINE_INSTANCE.read() else { return std::ptr::null_mut() };
        let Some(metrics) = engine.as_ref().and_then(|core| core.environment_metrics(&id).ok()) else {
            return std::ptr::null_mut();
        };
        create_jstring(&mut env, &serde_json::to_string(&metrics).unwrap_or_default())
    })).unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeEnvironmentStorageBytes(
    mut env: JNIEnv,
    _class: JClass,
    id: JString,
) -> jlong {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Ok(id) = env.get_string(&id) else { return -1 };
        let id: String = id.into();
        let Ok(engine) = ENGINE_INSTANCE.read() else { return -1 };
        engine.as_ref().and_then(|core| core.environment_storage_bytes(&id).ok())
            .and_then(|size| jlong::try_from(size).ok()).unwrap_or(-1)
    }));
    result.unwrap_or(-1)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeCancelInstallation(
    mut env: JNIEnv,
    _class: JClass,
    id: JString,
) -> jboolean {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let Ok(id) = env.get_string(&id) else { return false };
        let id: String = id.into();
        let Ok(guard) = ENGINE_INSTANCE.read() else { return false };
        guard.as_ref().is_some_and(|core| core.cancel_installation(&id))
    }));
    result.unwrap_or(false).into()
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeRecoverEnvironment(
    mut env: JNIEnv,
    _class: JClass,
    id: JString,
    restart: jboolean,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut run = || -> Result<(), String> {
            let id: String = env.get_string(&id).map_err(|error| error.to_string())?.into();
            let guard = ENGINE_INSTANCE.read().map_err(|error| error.to_string())?;
            guard.as_ref().ok_or("Engine not initialized")?
                .recover_environment(&id, restart != 0)
                .map_err(|error| error.to_string())
        };
        let error = run().err().unwrap_or_default();
        create_jstring(&mut env, &error)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeInstallEnvironment(
    mut env: JNIEnv,
    _class: JClass,
    id: JString,
    distro: JString,
    name: JString,
    memory_mb: jint,
    archive: JString,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let install = || -> Result<(), String> {
            let id: String = env.get_string(&id).map_err(|e| e.to_string())?.into();
            let distro: String = env.get_string(&distro).map_err(|e| e.to_string())?.into();
            let name: String = env.get_string(&name).map_err(|e| e.to_string())?.into();
            let archive: String = env.get_string(&archive).map_err(|e| e.to_string())?.into();
            let guard = ENGINE_INSTANCE.read().map_err(|e| e.to_string())?;
            let core = guard.as_ref().ok_or("Engine not initialized")?;
            core.install_environment(&id, &distro, &name, memory_mb as u32, std::path::Path::new(&archive))
                .map_err(|e| e.to_string())?;
            Ok(())
        };
        let mut install = install;
        let error = install().err().unwrap_or_default();
        create_jstring(&mut env, &error)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeProbeKvmSupport(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let caps = androidpc_vm::KvmCapabilities::probe();
        let json = serde_json::to_string(&caps).unwrap_or_else(|_| "{}".to_string());
        create_jstring(&mut env, &json)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeCreateVm(
    mut env: JNIEnv,
    _class: JClass,
    vm_id: JString,
    arch: JString,
    memory_mb: jint,
    cpus: jint,
    disk_size_mb: jint,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let id_str: String = match env.get_string(&vm_id) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };
        let arch_str: String = match env.get_string(&arch) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };
        let vm_arch = match androidpc_vm::VmArch::parse(&arch_str) {
            Ok(a) => a,
            Err(_) => return -2,
        };
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            match core.create_vm(
                &id_str,
                vm_arch,
                memory_mb.max(256) as u32,
                cpus.clamp(1, 16) as u8,
                disk_size_mb.max(256) as u32,
            ) {
                Ok(_) => 0,
                Err(e) => {
                    androidpc_logging::push_log("ERROR", "androidpc::bridge", &format!("Create VM failed: {e}"));
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
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeStartVm(
    mut env: JNIEnv,
    _class: JClass,
    vm_id: JString,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let id_str: String = match env.get_string(&vm_id) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            match core.start_vm(&id_str) {
                Ok(_) => 0,
                Err(e) => {
                    androidpc_logging::push_log("ERROR", "androidpc::bridge", &format!("Start VM '{id_str}' failed: {e}"));
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
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeStopVm(
    mut env: JNIEnv,
    _class: JClass,
    vm_id: JString,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let id_str: String = match env.get_string(&vm_id) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            match core.stop_vm(&id_str) {
                Ok(_) => 0,
                Err(_) => -2,
            }
        } else {
            -3
        }
    }));
    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeDeleteVm(
    mut env: JNIEnv,
    _class: JClass,
    vm_id: JString,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let id_str: String = match env.get_string(&vm_id) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            match core.delete_vm(&id_str) {
                Ok(_) => 0,
                Err(_) => -2,
            }
        } else {
            -3
        }
    }));
    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeListVms(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            let vms = core.list_vms();
            let json = serde_json::to_string(&vms).unwrap_or_else(|_| "[]".to_string());
            create_jstring(&mut env, &json)
        } else {
            create_jstring(&mut env, "[]")
        }
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeCreateTerminalForEnv(
    mut env: JNIEnv,
    _class: JClass,
    session_id: JString,
    env_id: JString,
    cols: jint,
    rows: jint,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let sid: String = match env.get_string(&session_id) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };
        let eid: String = match env.get_string(&env_id) {
            Ok(s) => s.into(),
            Err(_) => String::new(),
        };
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            let opt_eid = if eid.is_empty() { None } else { Some(eid.as_str()) };
            match core.create_terminal_session_for_env(
                &sid,
                opt_eid,
                cols.clamp(20, 300) as u16,
                rows.clamp(10, 120) as u16,
            ) {
                Ok(fd) => fd,
                Err(_) => -2,
            }
        } else {
            -3
        }
    }));
    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeSetGpuAccelerationMode(
    mut env: JNIEnv,
    _class: JClass,
    mode: JString,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mode_str: String = match env.get_string(&mode) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            core.set_gpu_acceleration_mode(androidpc_display::GpuAccelerationMode::parse(&mode_str));
            0
        } else {
            -2
        }
    }));
    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeGetGpuAccelerationMode(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let mode_str = guard
            .as_ref()
            .map(|c| c.get_gpu_acceleration_mode().as_str())
            .unwrap_or("software");
        create_jstring(&mut env, mode_str)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeCreateTerminalForEnvWithShell(
    mut env: JNIEnv,
    _class: JClass,
    session_id: JString,
    env_id: JString,
    cols: jint,
    rows: jint,
    shell: JString,
) -> jint {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let sid: String = match env.get_string(&session_id) {
            Ok(s) => s.into(),
            Err(_) => return -1,
        };
        let eid: String = match env.get_string(&env_id) {
            Ok(s) => s.into(),
            Err(_) => String::new(),
        };
        let sh: String = match env.get_string(&shell) {
            Ok(s) => s.into(),
            Err(_) => String::new(),
        };
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        if let Some(core) = guard.as_ref() {
            let opt_eid = if eid.is_empty() { None } else { Some(eid.as_str()) };
            let opt_sh = if sh.is_empty() { None } else { Some(sh.as_str()) };
            match core.create_terminal_session_for_env_with_shell(
                &sid,
                opt_eid,
                cols.clamp(20, 300) as u16,
                rows.clamp(10, 120) as u16,
                opt_sh,
            ) {
                Ok(fd) => fd,
                Err(_) => -2,
            }
        } else {
            -3
        }
    }));
    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeGetBox64Status(
    mut env: JNIEnv,
    _class: JClass,
    env_id: JString,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let eid: String = env.get_string(&env_id).map(Into::into).unwrap_or_default();
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let status = match guard.as_ref() {
            Some(core) => core.get_box64_status(if eid.is_empty() { None } else { Some(eid.as_str()) }),
            None => androidpc_box64::Box64CompatibilityLayer::new(EngineConfig::default().runtime_artifacts_dir)
                .probe_installation(None),
        };
        let json = serde_json::to_string(&status).unwrap_or_else(|_| "{}".to_string());
        create_jstring(&mut env, &json)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeGetBox64Config(
    mut env: JNIEnv,
    _class: JClass,
    env_id: JString,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let eid: String = env.get_string(&env_id).map(Into::into).unwrap_or_default();
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let cfg = guard
            .as_ref()
            .and_then(|core| core.get_box64_config(&eid).ok())
            .unwrap_or_default();
        let json = serde_json::to_string(&cfg).unwrap_or_else(|_| "{}".to_string());
        create_jstring(&mut env, &json)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeUpdateBox64Config(
    mut env: JNIEnv,
    _class: JClass,
    env_id: JString,
    config_json: JString,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let eid: String = match env.get_string(&env_id) {
            Ok(s) => s.into(),
            Err(_) => return create_jstring(&mut env, "{\"error\":\"Invalid environment id\"}"),
        };
        let raw_cfg: String = match env.get_string(&config_json) {
            Ok(s) => s.into(),
            Err(_) => return create_jstring(&mut env, "{\"error\":\"Invalid Box64 config JSON\"}"),
        };
        let parsed: androidpc_configuration::Box64EnvironmentConfig = match serde_json::from_str(&raw_cfg) {
            Ok(c) => c,
            Err(e) => return create_jstring(&mut env, &format!("{{\"error\":\"{}\"}}", e.to_string().replace('"', "'"))),
        };
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let json = match guard.as_ref() {
            Some(core) => match core.update_box64_config(&eid, parsed) {
                Ok(saved) => serde_json::to_string(&saved).unwrap_or_else(|_| "{}".to_string()),
                Err(e) => format!("{{\"error\":\"{}\"}}", e.to_string().replace('"', "'")),
            },
            None => "{\"error\":\"Engine not initialized\"}".to_string(),
        };
        create_jstring(&mut env, &json)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeExecuteCompatibleBinary(
    mut env: JNIEnv,
    _class: JClass,
    env_id: JString,
    binary_path: JString,
    args_json: JString,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let eid: String = env.get_string(&env_id).map(Into::into).unwrap_or_default();
        let bin: String = env.get_string(&binary_path).map(Into::into).unwrap_or_default();
        let args_str: String = env.get_string(&args_json).map(Into::into).unwrap_or_else(|_| "[]".to_string());
        let args: Vec<String> = serde_json::from_str(&args_str).unwrap_or_default();
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let json = match guard.as_ref() {
            Some(core) => match core.execute_compatible_binary(&eid, &bin, &args) {
                Ok(outcome) => serde_json::to_string(&outcome).unwrap_or_else(|_| "{}".to_string()),
                Err(e) => format!("{{\"error\":\"{}\"}}", e.to_string().replace('"', "'")),
            },
            None => "{\"error\":\"Engine not initialized\"}".to_string(),
        };
        create_jstring(&mut env, &json)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeRunBox64Suite(
    mut env: JNIEnv,
    _class: JClass,
    env_id: JString,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let eid: String = env.get_string(&env_id).map(Into::into).unwrap_or_default();
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let report = match guard.as_ref() {
            Some(core) => core.run_box64_compatibility_suite(if eid.is_empty() { None } else { Some(eid.as_str()) }),
            None => {
                let layer = androidpc_box64::Box64CompatibilityLayer::new(EngineConfig::default().runtime_artifacts_dir);
                androidpc_box64::build_compatibility_test_matrix(&layer, None, &Default::default())
            }
        };
        let json = serde_json::to_string(&report).unwrap_or_else(|_| "{}".to_string());
        create_jstring(&mut env, &json)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeGetBox64History(
    mut env: JNIEnv,
    _class: JClass,
    limit: jint,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let safe_limit = if limit <= 0 { 25 } else { limit as usize };
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let history = guard
            .as_ref()
            .map(|core| core.list_box64_execution_history(safe_limit))
            .unwrap_or_default();
        let json = serde_json::to_string(&history).unwrap_or_else(|_| "[]".to_string());
        create_jstring(&mut env, &json)
    }));
    result.unwrap_or(std::ptr::null_mut())
}


