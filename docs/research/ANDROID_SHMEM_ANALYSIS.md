# `libandroid-shmem` Analysis (`termux/libandroid-shmem`)

**Stage 3.2.** Reviewed the tagged `v0.7` upstream revision `7f0bd7e25dbdd146265aff7c6a890029e374622d`. The packaged ARM64 Termux 0.7 library matched the existing bundled library byte for byte. It stays outside the presenter. The descriptor and mapping primitives provide neither frame readiness nor a compositor contract. A future MIT-SHM or local transport prototype would need a buffer ownership protocol and synchronization. [Upstream source and license](https://github.com/termux/libandroid-shmem/tree/v0.7)

## Source Inspection (`G:\tmp\ref\libandroid-shmem`, BSD-3-Clause)
- **Confirmed**: `libandroid-shmem` (`shmem.c`, `shm.h`) is **BSD-3-Clause** licensed and is **not** a display renderer; it is a System V IPC shared-memory emulation library (`shmget`, `shmat`, `shmdt`, `shmctl`).
- **Confirmed (`shmem.c:1-100`)**:
  - On `__ANDROID_API__ >= 26`, it uses `<android/sharedmem.h>` (`ASharedMemory_create`) rather than legacy `/dev/ashmem`, making it compatible with Android 14/15/16 kernels where `/dev/ashmem` is absent.
  - Passes shared-memory file descriptors between processes over Unix sockets (`/dev/shm/%08x`) using `SCM_RIGHTS` (`ancil_send_fd` / `ancil_recv_fd`).
- **Limitation**: Segments are tied to process lifetime (destroyed when creating process exits) and do not implement POSIX `shm_open`. Useful for MIT-SHM inside PRoot/X11, not a standalone rendering pipeline.
