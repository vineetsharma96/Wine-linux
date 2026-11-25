use std::panic::catch_unwind;

use jni::objects::{JClass, JString};
use jni::sys::{jboolean, jstring};
use jni::JNIEnv;

use androidpc_graphics_backend::{GraphicsBackendConfig, GraphicsBackendKind};
use androidpc_windows_apps::WindowsAppProfile;
use androidpc_wine::{WineArch, WinePrefixMetadata};

use crate::{create_jstring, ENGINE_INSTANCE};

fn escape_json_error(msg: &str) -> String {
    format!("{{\"error\":\"{}\"}}", msg.replace('\\', "\\\\").replace('"', "'"))
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeGetWindowsOverview(
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
        let json = match guard.as_ref() {
            Some(core) => {
                let ov = core.get_windows_compatibility_overview(if eid.is_empty() {
                    None
                } else {
                    Some(eid.as_str())
                });
                serde_json::to_string(&ov).unwrap_or_else(|_| "{}".to_string())
            }
            None => escape_json_error("Engine not initialized"),
        };
        create_jstring(&mut env, &json)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeListWinePrefixes(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let list = guard
            .as_ref()
            .map(|c| c.list_wine_prefixes())
            .unwrap_or_default();
        let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".to_string());
        create_jstring(&mut env, &json)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeCreateWinePrefix(
    mut env: JNIEnv,
    _class: JClass,
    id: JString,
    display_name: JString,
    env_id: JString,
    arch: JString,
    windows_version: JString,
    graphics_backend: JString,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let pid: String = env.get_string(&id).map(Into::into).unwrap_or_default();
        let name: String = env.get_string(&display_name).map(Into::into).unwrap_or_default();
        let eid: String = env.get_string(&env_id).map(Into::into).unwrap_or_default();
        let arch_str: String = env.get_string(&arch).map(Into::into).unwrap_or_else(|_| "wow64".into());
        let win_ver: String = env.get_string(&windows_version).map(Into::into).unwrap_or_else(|_| "win10".into());
        let gfx_str: String = env.get_string(&graphics_backend).map(Into::into).unwrap_or_else(|_| "software".into());

        let parsed_arch = match arch_str.to_ascii_lowercase().as_str() {
            "win32" => WineArch::Win32,
            "win64" => WineArch::Win64,
            _ => WineArch::WoW64,
        };
        let parsed_gfx = GraphicsBackendKind::parse(&gfx_str);

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let json = match guard.as_ref() {
            Some(core) => match core.create_isolated_wine_prefix(
                &pid,
                &name,
                &eid,
                parsed_arch,
                &win_ver,
                parsed_gfx,
            ) {
                Ok(meta) => serde_json::to_string(&meta).unwrap_or_else(|_| "{}".to_string()),
                Err(e) => escape_json_error(&e.to_string()),
            },
            None => escape_json_error("Engine not initialized"),
        };
        create_jstring(&mut env, &json)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeManageWinePrefix(
    mut env: JNIEnv,
    _class: JClass,
    action: JString,
    prefix_id: JString,
    payload_json: JString,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let act: String = env.get_string(&action).map(Into::into).unwrap_or_default();
        let pid: String = env.get_string(&prefix_id).map(Into::into).unwrap_or_default();
        let payload: String = env.get_string(&payload_json).map(Into::into).unwrap_or_default();

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let json = match guard.as_ref() {
            Some(core) => match act.as_str() {
                "delete" => match core.delete_wine_prefix(&pid) {
                    Ok(()) => "{\"ok\":true}".to_string(),
                    Err(e) => escape_json_error(&e.to_string()),
                },
                "clone" => {
                    let v: serde_json::Value = serde_json::from_str(&payload).unwrap_or_default();
                    let new_id = v.get("new_id").and_then(|s| s.as_str()).unwrap_or("");
                    let new_name = v.get("new_name").and_then(|s| s.as_str()).unwrap_or("");
                    match core.clone_wine_prefix(&pid, new_id, new_name) {
                        Ok(meta) => serde_json::to_string(&meta).unwrap_or_else(|_| "{}".into()),
                        Err(e) => escape_json_error(&e.to_string()),
                    }
                }
                "backup" => match core.backup_wine_prefix(&pid) {
                    Ok(manifest) => serde_json::to_string(&manifest).unwrap_or_else(|_| "{}".into()),
                    Err(e) => escape_json_error(&e.to_string()),
                },
                "list_backups" => serde_json::to_string(&core.list_wine_prefix_backups())
                    .unwrap_or_else(|_| "[]".into()),
                "restore" => {
                    let v: serde_json::Value = serde_json::from_str(&payload).unwrap_or_default();
                    let target_id = v.get("target_id").and_then(|s| s.as_str()).unwrap_or("");
                    let env_id = v.get("env_id").and_then(|s| s.as_str()).unwrap_or("");
                    match core.restore_wine_prefix(&pid, target_id, env_id) {
                        Ok(meta) => serde_json::to_string(&meta).unwrap_or_else(|_| "{}".into()),
                        Err(e) => escape_json_error(&e.to_string()),
                    }
                }
                "repair" => match core.repair_wine_prefix(&pid) {
                    Ok(items) => serde_json::to_string(&items).unwrap_or_else(|_| "[]".into()),
                    Err(e) => escape_json_error(&e.to_string()),
                },
                "configure" => match serde_json::from_str::<WinePrefixMetadata>(&payload) {
                    Ok(meta) => match core.configure_wine_prefix(meta) {
                        Ok(saved) => serde_json::to_string(&saved).unwrap_or_else(|_| "{}".into()),
                        Err(e) => escape_json_error(&e.to_string()),
                    },
                    Err(e) => escape_json_error(&e.to_string()),
                },
                other => escape_json_error(&format!("Unknown prefix action: {other}")),
            },
            None => escape_json_error("Engine not initialized"),
        };
        create_jstring(&mut env, &json)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeProbeGraphicsBackends(
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
        let json = match guard.as_ref() {
            Some(core) => {
                let ov = core.probe_graphics_backends(if eid.is_empty() {
                    None
                } else {
                    Some(eid.as_str())
                });
                serde_json::to_string(&ov).unwrap_or_else(|_| "{}".into())
            }
            None => escape_json_error("Engine not initialized"),
        };
        create_jstring(&mut env, &json)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeConfigureGraphicsBackend(
    mut env: JNIEnv,
    _class: JClass,
    env_id: JString,
    config_json: JString,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let eid: String = env.get_string(&env_id).map(Into::into).unwrap_or_default();
        let raw_cfg: String = env.get_string(&config_json).map(Into::into).unwrap_or_default();
        let cfg: GraphicsBackendConfig = match serde_json::from_str(&raw_cfg) {
            Ok(c) => c,
            Err(e) => return create_jstring(&mut env, &escape_json_error(&e.to_string())),
        };
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let json = match guard.as_ref() {
            Some(core) => {
                let ov = core.configure_graphics_backend(
                    cfg,
                    if eid.is_empty() { None } else { Some(eid.as_str()) },
                );
                serde_json::to_string(&ov).unwrap_or_else(|_| "{}".into())
            }
            None => escape_json_error("Engine not initialized"),
        };
        create_jstring(&mut env, &json)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeListWindowsApps(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let apps = guard
            .as_ref()
            .map(|c| c.list_windows_apps())
            .unwrap_or_default();
        let json = serde_json::to_string(&apps).unwrap_or_else(|_| "[]".into());
        create_jstring(&mut env, &json)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeRegisterWindowsApp(
    mut env: JNIEnv,
    _class: JClass,
    profile_json: JString,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let raw: String = env.get_string(&profile_json).map(Into::into).unwrap_or_default();
        let profile: WindowsAppProfile = match serde_json::from_str(&raw) {
            Ok(p) => p,
            Err(e) => return create_jstring(&mut env, &escape_json_error(&e.to_string())),
        };
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let json = match guard.as_ref() {
            Some(core) => match core.register_windows_app(profile) {
                Ok(saved) => serde_json::to_string(&saved).unwrap_or_else(|_| "{}".into()),
                Err(e) => escape_json_error(&e.to_string()),
            },
            None => escape_json_error("Engine not initialized"),
        };
        create_jstring(&mut env, &json)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeLaunchWindowsApp(
    mut env: JNIEnv,
    _class: JClass,
    app_id: JString,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let aid: String = env.get_string(&app_id).map(Into::into).unwrap_or_default();
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let json = match guard.as_ref() {
            Some(core) => match core.launch_windows_app(&aid) {
                Ok(res) => serde_json::to_string(&res).unwrap_or_else(|_| "{}".into()),
                Err(e) => escape_json_error(&e.to_string()),
            },
            None => escape_json_error("Engine not initialized"),
        };
        create_jstring(&mut env, &json)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeRemoveWindowsApp(
    mut env: JNIEnv,
    _class: JClass,
    app_id: JString,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let aid: String = env.get_string(&app_id).map(Into::into).unwrap_or_default();
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let json = match guard.as_ref() {
            Some(core) => match core.remove_windows_app(&aid) {
                Ok(()) => "{\"ok\":true}".to_string(),
                Err(e) => escape_json_error(&e.to_string()),
            },
            None => escape_json_error("Engine not initialized"),
        };
        create_jstring(&mut env, &json)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeTerminateWindowsApp(
    mut env: JNIEnv,
    _class: JClass,
    app_id: JString,
    force: jboolean,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let aid: String = env.get_string(&app_id).map(Into::into).unwrap_or_default();
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let json = match guard.as_ref() {
            Some(core) => match core.terminate_windows_app(&aid, force != 0) {
                Ok(proc_opt) => serde_json::to_string(&proc_opt).unwrap_or_else(|_| "null".into()),
                Err(e) => escape_json_error(&e.to_string()),
            },
            None => escape_json_error("Engine not initialized"),
        };
        create_jstring(&mut env, &json)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeListWindowsProcesses(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let list = guard
            .as_ref()
            .map(|c| c.list_windows_processes())
            .unwrap_or_default();
        let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".into());
        create_jstring(&mut env, &json)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeListWindowsDependencies(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let list = guard
            .as_ref()
            .map(|c| c.list_windows_dependency_profiles())
            .unwrap_or_default();
        let json = serde_json::to_string(&list).unwrap_or_else(|_| "[]".into());
        create_jstring(&mut env, &json)
    }));
    result.unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeDiagnosePrefixDependencies(
    mut env: JNIEnv,
    _class: JClass,
    prefix_id: JString,
    recent_logs_json: JString,
) -> jstring {
    let result = catch_unwind(std::panic::AssertUnwindSafe(|| {
        let pid: String = env.get_string(&prefix_id).map(Into::into).unwrap_or_default();
        let logs_raw: String = env
            .get_string(&recent_logs_json)
            .map(Into::into)
            .unwrap_or_else(|_| "[]".into());
        let logs: Vec<String> = serde_json::from_str(&logs_raw).unwrap_or_default();

        let guard = match ENGINE_INSTANCE.read() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        let json = match guard.as_ref() {
            Some(core) => {
                let diag = core.diagnose_prefix_dependencies(&pid, &logs);
                serde_json::to_string(&diag).unwrap_or_else(|_| "{}".into())
            }
            None => escape_json_error("Engine not initialized"),
        };
        create_jstring(&mut env, &json)
    }));
    result.unwrap_or(std::ptr::null_mut())
}
