package com.androidpc.environment.bridge

import android.content.Context
import android.util.Log
import java.io.File

/**
 * Unpacks bundled GitHub/Termux runtime binaries and Box64 configuration from APK assets
 * into app-private sandbox storage on first launch or version upgrade.
 */
object AssetInstaller {
    private const val TAG = "AssetInstaller"

    fun installBundledAssets(context: Context): File {
        val artifactsDir = File(context.filesDir, "artifacts").apply { mkdirs() }
        val libDir = File(artifactsDir, "lib").apply { mkdirs() }
        val box64Dir = File(artifactsDir, "box64").apply { mkdirs() }
        val wineDir = File(artifactsDir, "wine").apply { mkdirs() }
        val graphicsDir = File(artifactsDir, "graphics").apply { mkdirs() }
        val dxvkDir = File(artifactsDir, "dxvk").apply { mkdirs() }
        val vkd3dDir = File(artifactsDir, "vkd3d-proton").apply { mkdirs() }
        val qemuDir = File(artifactsDir, "qemu").apply { mkdirs() }
        val qemuFwDir = File(artifactsDir, "qemu-firmware").apply { mkdirs() }
        val tigerVncDir = File(artifactsDir, "tigervnc-arm64").apply { mkdirs() }
        val xfceDir = File(artifactsDir, "xfce").apply { mkdirs() }

        val runtimeMap = mapOf(
            "runtime/arm64-v8a/proot-arm64" to File(artifactsDir, "proot-arm64"),
            "runtime/arm64-v8a/loader" to File(artifactsDir, "loader"),
            "runtime/arm64-v8a/loader32" to File(artifactsDir, "loader32"),
            "runtime/arm64-v8a/libtalloc.so.2" to File(libDir, "libtalloc.so.2"),
            "runtime/arm64-v8a/libandroid-shmem.so" to File(libDir, "libandroid-shmem.so"),
            "box64/default.box64rc" to File(box64Dir, "default.box64rc"),
            "box64/env_vars.json" to File(box64Dir, "env_vars.json"),
            "box64/box64" to File(box64Dir, "box64"),
            "box64/LICENSE" to File(box64Dir, "LICENSE"),
            "box64/manifest.json" to File(box64Dir, "manifest.json"),
            "runtime/wine/manifest.json" to File(wineDir, "manifest.json"),
            "runtime/graphics/manifest.json" to File(graphicsDir, "manifest.json"),
            "runtime/dxvk/manifest.json" to File(dxvkDir, "manifest.json"),
            "runtime/vkd3d/manifest.json" to File(vkd3dDir, "manifest.json"),
            "runtime/qemu/manifest.json" to File(qemuDir, "manifest.json"),
            "runtime/qemu/qemu-system-aarch64" to File(qemuDir, "qemu-system-aarch64"),
            "runtime/qemu/qemu-system-x86_64" to File(qemuDir, "qemu-system-x86_64"),
            "runtime/qemu/qemu-img" to File(qemuDir, "qemu-img"),
            "runtime/qemu-firmware/AAVMF_CODE.fd" to File(qemuFwDir, "AAVMF_CODE.fd"),
            "runtime/qemu-firmware/OVMF_CODE.fd" to File(qemuFwDir, "OVMF_CODE.fd"),
            "xvnc/Xvnc" to File(tigerVncDir, "Xvnc"),
            "xvnc/libvnc.so" to File(tigerVncDir, "libvnc.so"),
            "xvnc/vncserver" to File(tigerVncDir, "vncserver"),
            "xvnc/vncpasswd" to File(tigerVncDir, "vncpasswd"),
            "xvnc/vncconfig" to File(tigerVncDir, "vncconfig"),
            "xvnc/x0vncserver" to File(tigerVncDir, "x0vncserver"),
            "xfce/startxfce.sh" to File(xfceDir, "startxfce.sh")
        )

        val assetManager = context.assets
        for ((assetPath, targetFile) in runtimeMap) {
            try {
                if (!targetFile.exists() || targetFile.length() == 0L) {
                    assetManager.open(assetPath).use { input ->
                        targetFile.outputStream().use { output ->
                            input.copyTo(output)
                        }
                    }
                    targetFile.setExecutable(true, false)
                    targetFile.setReadable(true, false)
                    Log.i(TAG, "Installed bundled asset '$assetPath' -> ${targetFile.absolutePath}")
                }
            } catch (e: Exception) {
                Log.w(TAG, "Optional bundled asset '$assetPath' skipped: ${e.message}")
            }
        }

        // Prefer SELinux-executable nativeLibraryDir/libproot.so if extracted by Android package installer
        val nativeLibDir = context.applicationInfo.nativeLibraryDir
        if (nativeLibDir != null) {
            val nativeProot = File(nativeLibDir, "libproot.so")
            if (nativeProot.exists()) {
                Log.i(TAG, "Native-packaged libproot.so available at ${nativeProot.absolutePath}")
            }
        }

        return artifactsDir
    }
}
