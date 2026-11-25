package com.androidpc.environment.bridge

import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.withContext
import java.io.File
import java.net.HttpURLConnection
import java.net.URL

suspend fun downloadRootfs(
    meta: ArtifactMetadataData,
    partFile: File,
    filesDir: File,
    onProgress: (Float) -> Unit,
) = withContext(Dispatchers.IO) {
    check(meta.size_bytes > 0 && URL(meta.url).protocol == "https") { "Invalid rootfs metadata" }
    var start = partFile.length()
    if (start > meta.size_bytes) {
        check(partFile.delete()) { "Could not reset incomplete download" }
        start = 0
    }
    // Reserve extra space for the unpacked guest and archive scratch space.
    val reserveFactor = if (meta.filename.endsWith(".xz")) 14 else 8
    val estimatedBytes = meta.size_bytes * reserveFactor
    // Ubuntu 26.04.1 used about 1.25 GiB after extraction on the ARM64 test device.
    val reserveBytes = if (meta.distribution == "ubuntu") maxOf(estimatedBytes, 1_536L * 1024 * 1024) else estimatedBytes
    check(android.os.StatFs(filesDir.absolutePath).availableBytes >= reserveBytes - start) {
        "Free up storage before installing this root filesystem"
    }
    if (start == meta.size_bytes) return@withContext

    val connection = URL(meta.url).openConnection() as HttpURLConnection
    try {
        connection.connectTimeout = 10_000
        connection.readTimeout = 30_000
        connection.setRequestProperty("Accept-Encoding", "identity")
        if (start > 0) connection.setRequestProperty("Range", "bytes=$start-")
        val status = connection.responseCode
        check(connection.url.protocol == "https") { "Rootfs redirect must use HTTPS" }
        when (status) {
            HttpURLConnection.HTTP_OK -> start = 0
            HttpURLConnection.HTTP_PARTIAL -> {
                val expectedRange = "bytes $start-" + (meta.size_bytes - 1) + "/" + meta.size_bytes
                check(connection.getHeaderField("Content-Range") == expectedRange) {
                    "Rootfs server returned an invalid download range"
                }
            }
            else -> error("Rootfs server returned HTTP $status")
        }

        connection.inputStream.use { input ->
            java.io.FileOutputStream(partFile, start > 0).use { output ->
                val buffer = ByteArray(64 * 1024)
                var total = start
                var lastPercent = -1
                while (true) {
                    ensureActive()
                    val count = input.read(buffer)
                    if (count == -1) break
                    total += count
                    check(total <= meta.size_bytes) { "Rootfs download exceeds its expected size" }
                    output.write(buffer, 0, count)
                    val percent = (total * 100 / meta.size_bytes).toInt()
                    if (percent != lastPercent) {
                        lastPercent = percent
                        onProgress(percent / 100f)
                    }
                }
                check(total == meta.size_bytes) { "Rootfs download ended early. Retry to resume." }
            }
        }
    } finally {
        connection.disconnect()
    }
}
