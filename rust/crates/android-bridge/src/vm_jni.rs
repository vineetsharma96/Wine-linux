use std::panic::catch_unwind;
use std::path::PathBuf;

use jni::objects::{JClass, JString};
use jni::sys::jstring;
use jni::JNIEnv;
use serde::Deserialize;

use androidpc_vm::{DiskBus, DiskFormat, VmArch, VmProfile};

use crate::{create_jstring, ENGINE_INSTANCE};

fn jstr(env: &mut JNIEnv, value: &JString) -> String {
    env.get_string(value).map(Into::into).unwrap_or_default()
}

fn json_err(env: &mut JNIEnv, msg: impl std::fmt::Display) -> jstring {
    create_jstring(env, &serde_json::json!({ "error": msg.to_string() }).to_string())
}

fn json_ok<T: serde::Serialize>(env: &mut JNIEnv, value: &T) -> jstring {
    let payload = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
    create_jstring(env, &payload)
}

#[derive(Deserialize)]
struct CreateVmRequest {
    id: String,
    name: String,
    arch: String,
    memory_mb: u32,
    cpus: u8,
    disk_size_mb: u64,
    #[serde(default = "default_qcow2")]
    disk_format: String,
    #[serde(default)]
    iso_path: Option<String>,
}

fn default_qcow2() -> String {
    "qcow2".to_string()
}

#[derive(Deserialize)]
struct VmActionRequest {
    vm_id: String,
    action: String,
    #[serde(default)]
    timeout_ms: Option<u64>,
    #[serde(default)]
    new_id: Option<String>,
    #[serde(default)]
    new_name: Option<String>,
}

#[derive(Deserialize)]
struct VmDiskRequest {
    vm_id: String,
    action: String,
    #[serde(default)]
    disk_id: Option<String>,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    size_mb: Option<u64>,
    #[serde(default)]
    bus: Option<String>,
    #[serde(default)]
    delete_file: Option<bool>,
    #[serde(default)]
    source_path: Option<String>,
    #[serde(default)]
    expected_sha256: Option<String>,
}

#[derive(Deserialize)]
struct VmSnapshotRequest {
    vm_id: String,
    action: String,
    #[serde(default)]
    snapshot_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    description: Option<String>,
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeGetVmCapabilities(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = ENGINE_INSTANCE.read().unwrap_or_else(|p| p.into_inner());
        let Some(core) = guard.as_ref() else {
            return json_err(&mut env, "Engine not initialized");
        };
        json_ok(&mut env, &core.qemu_capabilities())
    }))
    .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeListVmProfiles(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = ENGINE_INSTANCE.read().unwrap_or_else(|p| p.into_inner());
        let Some(core) = guard.as_ref() else {
            return create_jstring(&mut env, "[]");
        };
        json_ok(&mut env, &core.list_vm_summaries())
    }))
    .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeCreateVmProfile(
    mut env: JNIEnv,
    _class: JClass,
    request_json: JString,
) -> jstring {
    catch_unwind(std::panic::AssertUnwindSafe(|| {
        let raw = jstr(&mut env, &request_json);
        let req: CreateVmRequest = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => return json_err(&mut env, format!("Invalid VM create payload: {e}")),
        };
        let arch = match VmArch::parse(&req.arch) {
            Ok(a) => a,
            Err(e) => return json_err(&mut env, e),
        };
        let format = match DiskFormat::parse(&req.disk_format) {
            Ok(f) => f,
            Err(e) => return json_err(&mut env, e),
        };
        let iso = req
            .iso_path
            .filter(|s| !s.trim().is_empty())
            .map(PathBuf::from);
        let guard = ENGINE_INSTANCE.read().unwrap_or_else(|p| p.into_inner());
        let Some(core) = guard.as_ref() else {
            return json_err(&mut env, "Engine not initialized");
        };
        match core.create_vm_profile(
            &req.id,
            &req.name,
            arch,
            req.memory_mb,
            req.cpus,
            req.disk_size_mb,
            format,
            iso,
        ) {
            Ok(summary) => json_ok(&mut env, &summary),
            Err(e) => json_err(&mut env, e),
        }
    }))
    .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeUpdateVmProfile(
    mut env: JNIEnv,
    _class: JClass,
    profile_json: JString,
) -> jstring {
    catch_unwind(std::panic::AssertUnwindSafe(|| {
        let raw = jstr(&mut env, &profile_json);
        let profile: VmProfile = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => return json_err(&mut env, format!("Invalid VmProfile JSON: {e}")),
        };
        let guard = ENGINE_INSTANCE.read().unwrap_or_else(|p| p.into_inner());
        let Some(core) = guard.as_ref() else {
            return json_err(&mut env, "Engine not initialized");
        };
        match core.update_vm_profile(profile) {
            Ok(summary) => json_ok(&mut env, &summary),
            Err(e) => json_err(&mut env, e),
        }
    }))
    .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeVmLifecycleAction(
    mut env: JNIEnv,
    _class: JClass,
    action_json: JString,
) -> jstring {
    catch_unwind(std::panic::AssertUnwindSafe(|| {
        let raw = jstr(&mut env, &action_json);
        let req: VmActionRequest = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => return json_err(&mut env, format!("Invalid VM action JSON: {e}")),
        };
        let guard = ENGINE_INSTANCE.read().unwrap_or_else(|p| p.into_inner());
        let Some(core) = guard.as_ref() else {
            return json_err(&mut env, "Engine not initialized");
        };
        match req.action.as_str() {
            "start" => match core.start_vm_detailed(&req.vm_id) {
                Ok(s) => json_ok(&mut env, &s),
                Err(e) => json_err(&mut env, e),
            },
            "pause" => match core.pause_vm(&req.vm_id) {
                Ok(s) => json_ok(&mut env, &s),
                Err(e) => json_err(&mut env, e),
            },
            "resume" => match core.resume_vm(&req.vm_id) {
                Ok(s) => json_ok(&mut env, &s),
                Err(e) => json_err(&mut env, e),
            },
            "restart" => match core.restart_vm(&req.vm_id) {
                Ok(s) => json_ok(&mut env, &s),
                Err(e) => json_err(&mut env, e),
            },
            "graceful_shutdown" => {
                let timeout = req.timeout_ms.unwrap_or(3000);
                match core.graceful_shutdown_vm(&req.vm_id, timeout) {
                    Ok(()) => create_jstring(&mut env, "{\"ok\":true}"),
                    Err(e) => json_err(&mut env, e),
                }
            }
            "force_stop" => match core.force_stop_vm(&req.vm_id) {
                Ok(()) => create_jstring(&mut env, "{\"ok\":true}"),
                Err(e) => json_err(&mut env, e),
            },
            "clone" => {
                let new_id = req.new_id.unwrap_or_else(|| format!("{}-clone", req.vm_id));
                let new_name = req.new_name.unwrap_or_else(|| format!("{} Clone", req.vm_id));
                match core.clone_vm(&req.vm_id, &new_id, &new_name) {
                    Ok(s) => json_ok(&mut env, &s),
                    Err(e) => json_err(&mut env, e),
                }
            }
            "delete" => match core.delete_vm(&req.vm_id) {
                Ok(()) => create_jstring(&mut env, "{\"ok\":true}"),
                Err(e) => json_err(&mut env, e),
            },
            other => json_err(&mut env, format!("Unsupported VM action '{other}'")),
        }
    }))
    .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeManageVmDisk(
    mut env: JNIEnv,
    _class: JClass,
    disk_json: JString,
) -> jstring {
    catch_unwind(std::panic::AssertUnwindSafe(|| {
        let raw = jstr(&mut env, &disk_json);
        let req: VmDiskRequest = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => return json_err(&mut env, format!("Invalid VM disk request: {e}")),
        };
        let guard = ENGINE_INSTANCE.read().unwrap_or_else(|p| p.into_inner());
        let Some(core) = guard.as_ref() else {
            return json_err(&mut env, "Engine not initialized");
        };
        match req.action.as_str() {
            "inspect" => match core.inspect_vm_disks(&req.vm_id) {
                Ok(reports) => json_ok(&mut env, &reports),
                Err(e) => json_err(&mut env, e),
            },
            "attach" => {
                let disk_id = req.disk_id.unwrap_or_else(|| "data1".to_string());
                let format = DiskFormat::parse(req.format.as_deref().unwrap_or("qcow2"))
                    .unwrap_or(DiskFormat::Qcow2);
                let size_mb = req.size_mb.unwrap_or(1024);
                let bus = match req.bus.as_deref().unwrap_or("virtio_blk") {
                    "virtio_scsi" => DiskBus::VirtioScsi,
                    "sata" => DiskBus::Sata,
                    "ide" => DiskBus::Ide,
                    _ => DiskBus::VirtioBlk,
                };
                match core.attach_vm_disk(&req.vm_id, &disk_id, format, size_mb, bus) {
                    Ok(summary) => json_ok(&mut env, &summary),
                    Err(e) => json_err(&mut env, e),
                }
            }
            "detach" => {
                let disk_id = req.disk_id.unwrap_or_default();
                let delete_file = req.delete_file.unwrap_or(false);
                match core.detach_vm_disk(&req.vm_id, &disk_id, delete_file) {
                    Ok(summary) => json_ok(&mut env, &summary),
                    Err(e) => json_err(&mut env, e),
                }
            }
            "resize" => {
                let disk_id = req.disk_id.unwrap_or_else(|| "disk0".to_string());
                let new_size_mb = req.size_mb.unwrap_or(4096);
                match core.resize_vm_disk(&req.vm_id, &disk_id, new_size_mb) {
                    Ok(report) => json_ok(&mut env, &report),
                    Err(e) => json_err(&mut env, e),
                }
            }
            "import" => {
                let disk_id = req.disk_id.unwrap_or_else(|| "imported".to_string());
                let Some(src) = req.source_path else {
                    return json_err(&mut env, "source_path is required for import");
                };
                match core.import_vm_disk(
                    &req.vm_id,
                    std::path::Path::new(&src),
                    &disk_id,
                    req.expected_sha256.as_deref(),
                ) {
                    Ok(summary) => json_ok(&mut env, &summary),
                    Err(e) => json_err(&mut env, e),
                }
            }
            other => json_err(&mut env, format!("Unsupported disk action '{other}'")),
        }
    }))
    .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeManageVmSnapshots(
    mut env: JNIEnv,
    _class: JClass,
    snapshot_json: JString,
) -> jstring {
    catch_unwind(std::panic::AssertUnwindSafe(|| {
        let raw = jstr(&mut env, &snapshot_json);
        let req: VmSnapshotRequest = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(e) => return json_err(&mut env, format!("Invalid VM snapshot request: {e}")),
        };
        let guard = ENGINE_INSTANCE.read().unwrap_or_else(|p| p.into_inner());
        let Some(core) = guard.as_ref() else {
            return json_err(&mut env, "Engine not initialized");
        };
        match req.action.as_str() {
            "list" => match core.list_vm_snapshots(&req.vm_id) {
                Ok(list) => json_ok(&mut env, &list),
                Err(e) => json_err(&mut env, e),
            },
            "create" => {
                let snap_id = req.snapshot_id.unwrap_or_else(|| "snap-1".to_string());
                let name = req.name.unwrap_or_else(|| snap_id.clone());
                let desc = req.description.unwrap_or_default();
                match core.create_vm_snapshot(&req.vm_id, &snap_id, &name, &desc) {
                    Ok(meta) => json_ok(&mut env, &meta),
                    Err(e) => json_err(&mut env, e),
                }
            }
            "restore" => {
                let snap_id = req.snapshot_id.unwrap_or_default();
                match core.restore_vm_snapshot(&req.vm_id, &snap_id) {
                    Ok(()) => create_jstring(&mut env, "{\"ok\":true}"),
                    Err(e) => json_err(&mut env, e),
                }
            }
            "delete" => {
                let snap_id = req.snapshot_id.unwrap_or_default();
                match core.delete_vm_snapshot(&req.vm_id, &snap_id) {
                    Ok(()) => create_jstring(&mut env, "{\"ok\":true}"),
                    Err(e) => json_err(&mut env, e),
                }
            }
            other => json_err(&mut env, format!("Unsupported snapshot action '{other}'")),
        }
    }))
    .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeGetVmLogs(
    mut env: JNIEnv,
    _class: JClass,
    vm_id: JString,
) -> jstring {
    catch_unwind(std::panic::AssertUnwindSafe(|| {
        let id = jstr(&mut env, &vm_id);
        let guard = ENGINE_INSTANCE.read().unwrap_or_else(|p| p.into_inner());
        let Some(core) = guard.as_ref() else {
            return create_jstring(&mut env, "[]");
        };
        match core.get_vm_logs(&id) {
            Ok(lines) => json_ok(&mut env, &lines),
            Err(e) => json_err(&mut env, e),
        }
    }))
    .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeListOfficialVmImages(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    catch_unwind(std::panic::AssertUnwindSafe(|| {
        let guard = ENGINE_INSTANCE.read().unwrap_or_else(|p| p.into_inner());
        let images = match guard.as_ref() {
            Some(core) => core.list_official_vm_images(),
            None => androidpc_vm::VmImageCatalog::list_official_images(),
        };
        json_ok(&mut env, &images)
    }))
    .unwrap_or(std::ptr::null_mut())
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeQueryVmQmpStatus(
    mut env: JNIEnv,
    _class: JClass,
    qmp_port: jni::sys::jint,
) -> jstring {
    catch_unwind(std::panic::AssertUnwindSafe(|| {
        match androidpc_vm::QmpClient::query_status(qmp_port as u16) {
            Ok(resp) => create_jstring(&mut env, &resp),
            Err(e) => json_err(&mut env, e),
        }
    }))
    .unwrap_or(std::ptr::null_mut())
}
