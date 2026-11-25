use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(target_os = "android")]
use std::sync::{atomic::AtomicBool, Arc, Mutex};
#[cfg(target_os = "android")]
use std::thread::{self, JoinHandle};
#[cfg(target_os = "android")]
use std::time::{Duration, Instant};

use jni::objects::{JClass, JObject};
use jni::sys::{jboolean, jint};
use jni::JNIEnv;

#[cfg(target_os = "android")]
use crate::ENGINE_INSTANCE;

static PRESENTED_FPS: AtomicU32 = AtomicU32::new(0);
static PRESENTER_FAILED: AtomicU32 = AtomicU32::new(0);

#[cfg(target_os = "android")]
mod ffi {
    use std::ffi::c_void;

    pub const WINDOW_FORMAT_RGBA_8888: i32 = 1;

    #[repr(C)]
    pub struct ANativeWindow {
        _private: [u8; 0],
    }

    #[repr(C)]
    pub struct ANativeWindowBuffer {
        pub width: i32,
        pub height: i32,
        pub stride: i32,
        pub format: i32,
        pub bits: *mut c_void,
        pub reserved: [u32; 6],
    }

    #[repr(C)]
    pub struct ARect {
        pub left: i32,
        pub top: i32,
        pub right: i32,
        pub bottom: i32,
    }

    #[link(name = "android")]
    extern "C" {
        pub fn ANativeWindow_fromSurface(
            env: *mut jni::sys::JNIEnv,
            surface: jni::sys::jobject,
        ) -> *mut ANativeWindow;
        pub fn ANativeWindow_setBuffersGeometry(
            window: *mut ANativeWindow,
            width: i32,
            height: i32,
            format: i32,
        ) -> i32;
        pub fn ANativeWindow_lock(
            window: *mut ANativeWindow,
            out_buffer: *mut ANativeWindowBuffer,
            in_out_dirty_bounds: *mut ARect,
        ) -> i32;
        pub fn ANativeWindow_unlockAndPost(window: *mut ANativeWindow) -> i32;
        pub fn ANativeWindow_release(window: *mut ANativeWindow);
    }
}

#[cfg(target_os = "android")]
struct PresenterHandle {
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

#[cfg(target_os = "android")]
impl PresenterHandle {
    fn shutdown(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(handle) = self.join.take() {
            let _ = handle.join();
        }
    }
}

#[cfg(target_os = "android")]
static ACTIVE_PRESENTER: Mutex<Option<PresenterHandle>> = Mutex::new(None);

#[cfg(target_os = "android")]
struct SendWindow(*mut ffi::ANativeWindow);
#[cfg(target_os = "android")]
// fromSurface returns an acquired reference. Only its presenter thread uses it.
unsafe impl Send for SendWindow {}

#[cfg(target_os = "android")]
impl Drop for SendWindow {
    fn drop(&mut self) {
        unsafe { ffi::ANativeWindow_release(self.0) };
    }
}

#[cfg(target_os = "android")]
fn spawn_presenter(window: SendWindow) -> std::io::Result<PresenterHandle> {
    let stop = Arc::new(AtomicBool::new(false));
    let stop_flag = stop.clone();
    let join = thread::Builder::new()
        .name("androidpc-anativewindow".to_string())
        .spawn(move || {
            let win = window;
            let mut last_gen: u64 = 0;
            let mut geom_w: i32 = 0;
            let mut geom_h: i32 = 0;
            let mut frames: u32 = 0;
            let mut fps_window = Instant::now();
            let mut posted = 0u64;
            let mut copied_bytes = 0u64;
            let mut copy_nanos = 0u128;
            let mut submit_nanos = 0u128;

            while !stop_flag.load(Ordering::Acquire) {
                let dirty = ENGINE_INSTANCE.read().ok().and_then(|engine| {
                    let mut rfb = engine.as_ref()?.rfb_client.write().ok()?;
                    let client = rfb.as_mut().filter(|client| client.is_connected)?;
                    let _ = client.poll_updates();
                    let _ = client.request_frame(true);
                    (client.is_connected && client.generation != last_gen && client.width > 0 && client.height > 0)
                        .then_some((client.generation, i32::from(client.width), i32::from(client.height)))
                });

                if let Some((generation, width, height)) = dirty {
                    let submit_start = Instant::now();
                    if (geom_w, geom_h) != (width, height) {
                        if unsafe { ffi::ANativeWindow_setBuffersGeometry(win.0, width, height, ffi::WINDOW_FORMAT_RGBA_8888) } != 0 {
                            PRESENTER_FAILED.store(1, Ordering::Release);
                            break;
                        }
                        (geom_w, geom_h) = (width, height);
                    }

                    let mut buffer = unsafe { std::mem::zeroed::<ffi::ANativeWindowBuffer>() };
                    // BufferQueue may block. Hold neither the engine nor RFB lock across it.
                    if unsafe { ffi::ANativeWindow_lock(win.0, &mut buffer, std::ptr::null_mut()) } != 0 {
                        PRESENTER_FAILED.store(1, Ordering::Release);
                        break;
                    }
                    let valid = !buffer.bits.is_null() && buffer.width >= width && buffer.height >= height
                        && buffer.stride >= width && buffer.format == ffi::WINDOW_FORMAT_RGBA_8888;
                    let copy_start = Instant::now();
                    let copied = valid && ENGINE_INSTANCE.read().ok().is_some_and(|engine| {
                        let Some(core) = engine.as_ref() else { return false };
                        let Ok(rfb) = core.rfb_client.read() else { return false };
                        let Some(client) = rfb.as_ref().filter(|client| client.is_connected
                            && client.generation == generation && i32::from(client.width) == width
                            && i32::from(client.height) == height) else { return false };
                        let row_bytes = width as usize * 4;
                        if client.framebuffer.len() != row_bytes * height as usize {
                            return false;
                        }
                        // The successful lock guarantees writable storage for each validated
                        // stride. The Rust framebuffer and native surface are disjoint.
                        for (row, source) in client.framebuffer.chunks_exact(row_bytes).enumerate() {
                            unsafe {
                                std::ptr::copy_nonoverlapping(source.as_ptr(),
                                    buffer.bits.cast::<u8>().add(row * buffer.stride as usize * 4), row_bytes);
                            }
                        }
                        true
                    });
                    let copy_duration = copy_start.elapsed();
                    // Every successful lock is paired with a post, even on a stale snapshot.
                    let result = unsafe { ffi::ANativeWindow_unlockAndPost(win.0) };
                    if !valid || result != 0 {
                        PRESENTER_FAILED.store(1, Ordering::Release);
                        break;
                    }
                    if copied {
                        last_gen = generation;
                        frames += 1;
                        posted += 1;
                        copied_bytes += width as u64 * height as u64 * 4;
                        copy_nanos += copy_duration.as_nanos();
                        submit_nanos += submit_start.elapsed().as_nanos();
                    }
                }

                if fps_window.elapsed() >= Duration::from_secs(1) {
                    PRESENTED_FPS.store(frames, Ordering::Relaxed);
                    frames = 0;
                    fps_window = Instant::now();
                }

                if dirty.is_none() {
                    thread::sleep(Duration::from_millis(1));
                }
            }

            androidpc_logging::push_log("INFO", "androidpc::display", &format!(
                "Native surface posted {posted} frames, copied {copied_bytes} bytes in {copy_nanos} ns, submit path {submit_nanos} ns"
            ));
            PRESENTED_FPS.store(0, Ordering::Relaxed);
        })
        ?;

    Ok(PresenterHandle { stop, join: Some(join) })
}

pub fn detach_surface() {
    #[cfg(target_os = "android")]
    if let Ok(mut guard) = ACTIVE_PRESENTER.lock() {
        if let Some(mut handle) = guard.take() {
            handle.shutdown();
        }
        PRESENTED_FPS.store(0, Ordering::Relaxed);
        PRESENTER_FAILED.store(0, Ordering::Release);
    }
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeAttachDisplaySurface(
    env: JNIEnv,
    _class: JClass,
    surface: JObject,
) -> jint {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if surface.is_null() {
            detach_surface();
            return 0;
        }
        #[cfg(target_os = "android")]
        {
            let win = unsafe {
                ffi::ANativeWindow_fromSurface(env.get_raw(), surface.as_raw())
            };
            if win.is_null() {
                return -1;
            }
            let window = SendWindow(win);
            let Ok(mut guard) = ACTIVE_PRESENTER.lock() else { return -1 };
            if let Some(mut old) = guard.take() {
                old.shutdown();
            }
            PRESENTER_FAILED.store(0, Ordering::Release);
            match spawn_presenter(window) {
                Ok(handle) => { *guard = Some(handle); 0 }
                Err(_) => -1,
            }
        }
        #[cfg(not(target_os = "android"))]
        {
            let _ = env;
            0
        }
    }));
    result.unwrap_or(-99)
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeDisplayPresenterFailed(
    _env: JNIEnv,
    _class: JClass,
) -> jboolean {
    (PRESENTER_FAILED.load(Ordering::Acquire) != 0) as jboolean
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeDetachDisplaySurface(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    detach_surface();
    0
}

#[no_mangle]
pub extern "system" fn Java_com_androidpc_environment_bridge_NativeBridge_nativeGetPresentedFps(
    _env: JNIEnv,
    _class: JClass,
) -> jint {
    PRESENTED_FPS.load(Ordering::Relaxed) as jint
}
