package com.androidpc.environment.bridge

import android.content.Context
import android.util.Log
import com.androidpc.environment.BuildConfig
import com.google.gson.Gson
import com.google.gson.JsonParser
import com.google.gson.reflect.TypeToken
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import java.io.File
import java.io.RandomAccessFile
import java.security.MessageDigest

/**
 * Phase 5 QEMU Virtual Machine Engine Kotlin/JNI Facade.
 *
 * All core virtualization orchestration lives in Rust (`rust/crates/vm` + `rust/crates/android-bridge/src/vm_jni.rs`).
 * When running against an older prebuilt `libandroidpc_engine.so` (UnsatisfiedLinkError), this bridge degrades
 * honestly: it inspects actual host filesystem/KVM capabilities and persists profiles/disks/offline snapshots in
 * app-private `filesDir/vms`, while explicitly reporting when QEMU binaries or JNI symbols are not yet staged
 * (never fabricating running VM state or KVM access).
 */
object VmEngineBridge {
    private const val TAG = "VmEngineBridge"
    private val gson = Gson()

    private fun extractError(raw: String): String? = runCatching {
        val obj = JsonParser.parseString(raw).asJsonObject
        if (obj.has("error") && !obj.get("error").isJsonNull) obj.get("error").asString else null
    }.getOrNull()

    suspend fun getCapabilities(context: Context): QemuEngineCapabilitiesData = withContext(Dispatchers.IO) {
        if (NativeBridge.isLoaded) {
            try {
                val raw = NativeBridge.nativeGetVmCapabilities()
                if (extractError(raw) == null) {
                    return@withContext gson.fromJson(raw, QemuEngineCapabilitiesData::class.java)
                }
            } catch (_: UnsatisfiedLinkError) {
                Log.w(TAG, "nativeGetVmCapabilities pending .so rebuild; probing local filesystem honestly")
            } catch (e: Exception) {
                Log.w(TAG, "nativeGetVmCapabilities failed: ${e.message}")
            }
        }
        probeLocalCapabilities(context)
    }

    suspend fun listVms(context: Context): List<VmRuntimeSummaryData> = withContext(Dispatchers.IO) {
        if (NativeBridge.isLoaded) {
            try {
                val raw = NativeBridge.nativeListVmProfiles()
                val type = object : TypeToken<List<VmRuntimeSummaryData>>() {}.type
                return@withContext gson.fromJson(raw, type)
            } catch (_: UnsatisfiedLinkError) {
                // Fallback to reading persisted app-private VM profiles
            } catch (e: Exception) {
                Log.w(TAG, "nativeListVmProfiles failed: ${e.message}")
            }
        }
        loadLocalVmSummaries(context)
    }

    suspend fun createVm(
        context: Context,
        id: String,
        name: String,
        arch: String,
        memoryMb: Int,
        cpus: Int,
        diskSizeMb: Long,
        diskFormat: String = "qcow2",
        isoPath: String? = null,
        networkMode: String = "user_slirp",
        audioBackend: String = "disabled",
        displayBackend: String = "vnc_localhost",
        firmwareKind: String? = null
    ): Result<VmRuntimeSummaryData> = withContext(Dispatchers.IO) {
        val cleanId = id.trim()
        if (cleanId.isEmpty() || !cleanId.all { it.isLetterOrDigit() || it == '-' || it == '_' }) {
            return@withContext Result.failure(IllegalArgumentException("VM ID must use letters, digits, '-' or '_'"))
        }
        val payload = mapOf(
            "id" to cleanId,
            "name" to name.trim().ifEmpty { cleanId },
            "arch" to arch,
            "memory_mb" to memoryMb,
            "cpus" to cpus,
            "disk_size_mb" to diskSizeMb,
            "disk_format" to diskFormat,
            "iso_path" to isoPath?.trim()?.takeIf { it.isNotEmpty() }
        )
        if (NativeBridge.isLoaded) {
            try {
                val raw = NativeBridge.nativeCreateVmProfile(gson.toJson(payload))
                extractError(raw)?.let { return@withContext Result.failure(RuntimeException(it)) }
                var summary = gson.fromJson(raw, VmRuntimeSummaryData::class.java)
                val needsCustomUpdate = networkMode != "user_slirp" ||
                    audioBackend != "disabled" ||
                    displayBackend != "vnc_localhost" ||
                    firmwareKind != null
                if (needsCustomUpdate) {
                    val updatedProfile = summary.profile.copy(
                        network = summary.profile.network.copy(mode = networkMode),
                        audio = summary.profile.audio.copy(backend = audioBackend),
                        display = summary.profile.display.copy(backend = displayBackend),
                        firmware = summary.profile.firmware.copy(kind = firmwareKind ?: summary.profile.firmware.kind)
                    )
                    val updRaw = NativeBridge.nativeUpdateVmProfile(gson.toJson(updatedProfile))
                    if (extractError(updRaw) == null) {
                        summary = gson.fromJson(updRaw, VmRuntimeSummaryData::class.java)
                    }
                }
                return@withContext Result.success(summary)
            } catch (_: UnsatisfiedLinkError) {
                // Fallback below
            } catch (e: Exception) {
                return@withContext Result.failure(e)
            }
        }
        createLocalVmFallback(
            context, cleanId, name.trim().ifEmpty { cleanId }, arch, memoryMb, cpus,
            diskSizeMb, diskFormat, isoPath, networkMode, audioBackend, displayBackend, firmwareKind
        )
    }

    suspend fun lifecycleAction(
        context: Context,
        vmId: String,
        action: String,
        timeoutMs: Long = 3000L,
        newId: String? = null,
        newName: String? = null
    ): Result<String> = withContext(Dispatchers.IO) {
        val req = mutableMapOf<String, Any>(
            "vm_id" to vmId,
            "action" to action,
            "timeout_ms" to timeoutMs
        )
        newId?.let { req["new_id"] = it }
        newName?.let { req["new_name"] = it }

        if (NativeBridge.isLoaded) {
            try {
                val raw = NativeBridge.nativeVmLifecycleAction(gson.toJson(req))
                extractError(raw)?.let { return@withContext Result.failure(RuntimeException(it)) }
                return@withContext Result.success(raw)
            } catch (_: UnsatisfiedLinkError) {
                // Handle local non-execution actions honestly; never fake a running VM
            } catch (e: Exception) {
                return@withContext Result.failure(e)
            }
        }
        when (action) {
            "delete" -> {
                val dir = File(context.filesDir, "vms/$vmId")
                dir.deleteRecursively()
                Result.success("{\"ok\":true}")
            }
            "clone" -> {
                val targetId = newId?.trim().orEmpty().ifEmpty { "${vmId}-clone" }
                val srcDir = File(context.filesDir, "vms/$vmId")
                val dstDir = File(context.filesDir, "vms/$targetId")
                if (!srcDir.isDirectory) return@withContext Result.failure(RuntimeException("Source VM not found"))
                if (dstDir.exists()) return@withContext Result.failure(RuntimeException("VM '$targetId' already exists"))
                srcDir.copyRecursively(dstDir, overwrite = false)
                val profileFile = File(dstDir, "vm_profile.json")
                if (profileFile.isFile) {
                    val existing = gson.fromJson(profileFile.readText(), VmProfileData::class.java)
                    val cloned = existing.copy(
                        id = targetId,
                        name = newName?.trim()?.ifEmpty { "${existing.name} (Clone)" } ?: "${existing.name} (Clone)",
                        state = "Stopped",
                        created_at_epoch_ms = System.currentTimeMillis(),
                        display = existing.display.copy(vnc_display = existing.display.vnc_display + 1),
                        qmp_port = existing.qmp_port + 1
                    )
                    profileFile.writeText(gson.toJson(cloned))
                }
                Result.success("{\"ok\":true}")
            }
            else -> Result.failure(
                UnsupportedOperationException(
                    "QEMU execution action '$action' requires staged QEMU ${BuildConfig.QEMU_VERSION} binary and rebuilt libandroidpc_engine.so."
                )
            )
        }
    }

    suspend fun inspectDisks(context: Context, vmId: String): Result<List<VmDiskInspectionData>> = withContext(Dispatchers.IO) {
        if (NativeBridge.isLoaded) {
            try {
                val raw = NativeBridge.nativeManageVmDisk(gson.toJson(mapOf("vm_id" to vmId, "action" to "inspect")))
                extractError(raw)?.let { return@withContext Result.failure(RuntimeException(it)) }
                val type = object : TypeToken<List<VmDiskInspectionData>>() {}.type
                return@withContext Result.success(gson.fromJson(raw, type))
            } catch (_: UnsatisfiedLinkError) {
                // Local inspection fallback
            } catch (e: Exception) {
                return@withContext Result.failure(e)
            }
        }
        val summary = loadLocalVmSummaries(context).firstOrNull { it.profile.id == vmId }
            ?: return@withContext Result.failure(RuntimeException("VM '$vmId' not found"))
        val reports = summary.profile.disks.map { disk ->
            val file = File(disk.path)
            val exists = file.isFile
            val len = if (exists) file.length() else 0L
            VmDiskInspectionData(
                path = disk.path,
                exists = exists,
                detected_format = if (exists) disk.format else "missing",
                virtual_size_bytes = disk.size_mb * 1024L * 1024L,
                actual_disk_bytes = len,
                qcow2_version = if (disk.format == "qcow2" && exists) 3 else null,
                healthy = exists && len > 0L,
                diagnostics = listOf(if (exists) "File present (${len / 1024} KiB on disk)" else "Disk file missing")
            )
        }
        Result.success(reports)
    }

    suspend fun attachDisk(
        context: Context,
        vmId: String,
        diskId: String,
        format: String,
        sizeMb: Long,
        bus: String = "virtio_blk"
    ): Result<Unit> = withContext(Dispatchers.IO) {
        if (NativeBridge.isLoaded) {
            try {
                val raw = NativeBridge.nativeManageVmDisk(
                    gson.toJson(
                        mapOf(
                            "vm_id" to vmId,
                            "action" to "attach",
                            "disk_id" to diskId,
                            "format" to format,
                            "size_mb" to sizeMb,
                            "bus" to bus
                        )
                    )
                )
                extractError(raw)?.let { return@withContext Result.failure(RuntimeException(it)) }
                return@withContext Result.success(Unit)
            } catch (_: UnsatisfiedLinkError) {
                // Local fallback below
            } catch (e: Exception) {
                return@withContext Result.failure(e)
            }
        }
        runCatching {
            val vmDir = File(context.filesDir, "vms/$vmId")
            val profileFile = File(vmDir, "vm_profile.json")
            val profile = gson.fromJson(profileFile.readText(), VmProfileData::class.java)
            val ext = if (format == "qcow2") "qcow2" else "img"
            val diskFile = File(vmDir, "$diskId.$ext")
            createLocalDiskFile(diskFile, format, sizeMb)
            val updated = profile.copy(
                disks = profile.disks + VmStorageDeviceData(
                    disk_id = diskId,
                    path = diskFile.absolutePath,
                    format = format,
                    bus = bus,
                    size_mb = sizeMb,
                    boot_index = null
                )
            )
            profileFile.writeText(gson.toJson(updated))
        }
    }

    suspend fun listSnapshots(context: Context, vmId: String): List<VmSnapshotMetadataData> = withContext(Dispatchers.IO) {
        if (NativeBridge.isLoaded) {
            try {
                val raw = NativeBridge.nativeManageVmSnapshots(gson.toJson(mapOf("vm_id" to vmId, "action" to "list")))
                if (extractError(raw) == null) {
                    val type = object : TypeToken<List<VmSnapshotMetadataData>>() {}.type
                    return@withContext gson.fromJson(raw, type)
                }
            } catch (_: UnsatisfiedLinkError) {
                // Fallback below
            } catch (e: Exception) {
                Log.w(TAG, "listSnapshots failed: ${e.message}")
            }
        }
        val snapDir = File(context.filesDir, "vms/$vmId/snapshots")
        snapDir.listFiles { f -> f.extension == "json" }
            ?.mapNotNull { runCatching { gson.fromJson(it.readText(), VmSnapshotMetadataData::class.java) }.getOrNull() }
            ?.sortedBy { it.created_at_epoch_ms }
            ?: emptyList()
    }

    suspend fun manageSnapshot(
        context: Context,
        vmId: String,
        action: String,
        snapshotId: String,
        name: String = snapshotId,
        description: String = ""
    ): Result<Unit> = withContext(Dispatchers.IO) {
        if (NativeBridge.isLoaded) {
            try {
                val raw = NativeBridge.nativeManageVmSnapshots(
                    gson.toJson(
                        mapOf(
                            "vm_id" to vmId,
                            "action" to action,
                            "snapshot_id" to snapshotId,
                            "name" to name,
                            "description" to description
                        )
                    )
                )
                extractError(raw)?.let { return@withContext Result.failure(RuntimeException(it)) }
                return@withContext Result.success(Unit)
            } catch (_: UnsatisfiedLinkError) {
                // Local offline verified snapshot fallback below
            } catch (e: Exception) {
                return@withContext Result.failure(e)
            }
        }
        runCatching {
            val vmDir = File(context.filesDir, "vms/$vmId")
            val snapDir = File(vmDir, "snapshots").apply { mkdirs() }
            val metaFile = File(snapDir, "$snapshotId.json")
            val profile = gson.fromJson(File(vmDir, "vm_profile.json").readText(), VmProfileData::class.java)
            val primary = profile.disks.firstOrNull() ?: error("VM has no primary disk")
            when (action) {
                "create" -> {
                    val backup = File(snapDir, "$snapshotId.disk")
                    File(primary.path).copyTo(backup, overwrite = true)
                    val hash = sha256File(backup)
                    val meta = VmSnapshotMetadataData(
                        snapshot_id = snapshotId,
                        vm_id = vmId,
                        name = name.ifBlank { snapshotId },
                        description = description,
                        disk_format = primary.format,
                        mechanism = "offline_verified_copy",
                        created_at_epoch_ms = System.currentTimeMillis(),
                        disk_sha256 = hash,
                        backup_disk_path = backup.absolutePath
                    )
                    metaFile.writeText(gson.toJson(meta))
                }
                "restore" -> {
                    val meta = gson.fromJson(metaFile.readText(), VmSnapshotMetadataData::class.java)
                    val backup = File(meta.backup_disk_path ?: error("Backup path missing"))
                    meta.disk_sha256?.let { expected ->
                        check(sha256File(backup).equals(expected, ignoreCase = true)) { "Snapshot SHA-256 verification failed" }
                    }
                    backup.copyTo(File(primary.path), overwrite = true)
                }
                "delete" -> {
                    if (metaFile.isFile) {
                        val meta = gson.fromJson(metaFile.readText(), VmSnapshotMetadataData::class.java)
                        meta.backup_disk_path?.let { File(it).delete() }
                        metaFile.delete()
                    }
                }
            }
        }
    }

    suspend fun getVmLogs(vmId: String): List<String> = withContext(Dispatchers.IO) {
        if (NativeBridge.isLoaded) {
            try {
                val raw = NativeBridge.nativeGetVmLogs(vmId)
                if (extractError(raw) == null) {
                    val type = object : TypeToken<List<String>>() {}.type
                    return@withContext gson.fromJson(raw, type)
                }
            } catch (_: UnsatisfiedLinkError) {
                return@withContext listOf("Native Phase 5 VM JNI symbols pending libandroidpc_engine.so rebuild.")
            } catch (_: Exception) {
                // Fall through
            }
        }
        listOf("No active QEMU runtime log lines recorded for '$vmId'.")
    }

    suspend fun listOfficialImages(): List<VmGuestImageEntryData> = withContext(Dispatchers.IO) {
        if (NativeBridge.isLoaded) {
            try {
                val raw = NativeBridge.nativeListOfficialVmImages()
                val type = object : TypeToken<List<VmGuestImageEntryData>>() {}.type
                return@withContext gson.fromJson(raw, type)
            } catch (_: UnsatisfiedLinkError) {
                // Fallback catalog mirroring Rust VmImageCatalog
            } catch (_: Exception) {
                // Fallback catalog
            }
        }
        listOf(
            VmGuestImageEntryData(
                id = "alpine-virt-aarch64",
                display_name = "Alpine Linux Virtual (ARM64)",
                distribution = "alpine",
                version = "3.21",
                arch = "aarch64",
                media_type = "iso",
                recommended_firmware = "aavmf_uefi",
                min_ram_mb = 512,
                min_disk_mb = 1024,
                official_download_url = "https://alpinelinux.org/downloads/",
                checksum_url = "https://dl-cdn.alpinelinux.org/alpine/latest-stable/releases/aarch64/latest-releases.yaml",
                license = "MIT / GPL-2.0 (Open Source)",
                redistribution_policy = "Official upstream HTTPS download + SHA-256 verification",
                notes = "Lightweight ARM64 baseline for validating QEMU TCG virt + VirtIO boot."
            ),
            VmGuestImageEntryData(
                id = "debian-13-netinst-arm64",
                display_name = "Debian 13 Trixie Netinst (ARM64)",
                distribution = "debian",
                version = "13",
                arch = "aarch64",
                media_type = "iso",
                recommended_firmware = "aavmf_uefi",
                min_ram_mb = 1024,
                min_disk_mb = 4096,
                official_download_url = "https://cdimage.debian.org/debian-cd/current/arm64/iso-cd/",
                checksum_url = "https://cdimage.debian.org/debian-cd/current/arm64/iso-cd/SHA256SUMS",
                license = "DFSG Open Source",
                redistribution_policy = "Official Debian CD image + signed SHA256SUMS",
                notes = "Full ARM64 Linux guest with VirtIO drivers."
            ),
            VmGuestImageEntryData(
                id = "ubuntu-26.04-server-arm64",
                display_name = "Ubuntu 26.04 LTS Server (ARM64)",
                distribution = "ubuntu",
                version = "26.04 LTS",
                arch = "aarch64",
                media_type = "iso",
                recommended_firmware = "aavmf_uefi",
                min_ram_mb = 2048,
                min_disk_mb = 8192,
                official_download_url = "https://cdimage.ubuntu.com/releases/26.04/release/",
                checksum_url = "https://cdimage.ubuntu.com/releases/26.04/release/SHA256SUMS",
                license = "Ubuntu Open Source",
                redistribution_policy = "Official Canonical release + signed SHA256SUMS",
                notes = "Requires AAVMF UEFI firmware and >= 2 GB RAM."
            ),
            VmGuestImageEntryData(
                id = "alpine-virt-x86_64",
                display_name = "Alpine Linux Virtual (x86-64 TCG Emulated)",
                distribution = "alpine",
                version = "3.21",
                arch = "x86_64",
                media_type = "iso",
                recommended_firmware = "ovmf_uefi",
                min_ram_mb = 512,
                min_disk_mb = 1024,
                official_download_url = "https://alpinelinux.org/downloads/",
                checksum_url = "https://dl-cdn.alpinelinux.org/alpine/latest-stable/releases/x86_64/latest-releases.yaml",
                license = "MIT / GPL-2.0 (Open Source)",
                redistribution_policy = "Official upstream HTTPS download + SHA-256 verification",
                notes = "Cross-architecture x86-64 TCG emulation on ARM64 — NOT native speed."
            )
        )
    }

    private fun probeLocalCapabilities(context: Context): QemuEngineCapabilitiesData {
        val qemuDir = File(context.filesDir, "artifacts/qemu")
        val fwDir = File(context.filesDir, "artifacts/qemu-firmware")
        val kvmFile = File("/dev/kvm")
        val kvmPresent = kvmFile.exists()
        val kvmAccessible = kvmPresent && kvmFile.canRead() && kvmFile.canWrite()
        return QemuEngineCapabilitiesData(
            pinned_qemu_version = BuildConfig.QEMU_VERSION,
            pinned_edk2_version = BuildConfig.EDK2_VERSION,
            pinned_libslirp_version = BuildConfig.LIBSLIRP_VERSION,
            qemu_aarch64_installed = File(qemuDir, "qemu-system-aarch64").isFile,
            qemu_x86_64_installed = File(qemuDir, "qemu-system-x86_64").isFile,
            qemu_img_installed = File(qemuDir, "qemu-img").isFile,
            aavmf_firmware_installed = File(fwDir, "AAVMF_CODE.fd").isFile,
            ovmf_firmware_installed = File(fwDir, "OVMF_CODE.fd").isFile,
            kvm = KvmCapabilitiesData(
                device_present = kvmPresent,
                accessible = kvmAccessible,
                accelerator = if (kvmAccessible) "kvm" else "tcg",
                notes = when {
                    kvmAccessible -> "/dev/kvm accessible; hardware virtualization available for ARM64 guests"
                    kvmPresent -> "/dev/kvm present but restricted by Android SELinux; falling back to TCG"
                    else -> "/dev/kvm not exposed by kernel; using QEMU multi-threaded TCG software translation"
                }
            ),
            avf = AvfCapabilitiesData(
                ordinary_app_accessible = false,
                permission_required = "android.permission.MANAGE_VIRTUAL_MACHINE (@SystemApi)",
                notes = "AVF/pKVM is restricted to privileged system apps; ordinary third-party apps use QEMU TCG."
            ),
            networking_notes = "Rootless Android uses QEMU user-mode SLIRP (libslirp ${BuildConfig.LIBSLIRP_VERSION}); raw TAP/bridge requires CAP_NET_ADMIN.",
            security_boundary_note = "QEMU runs inside the ordinary Android app UID sandbox; it provides emulation isolation rather than an absolute kernel security boundary."
        )
    }

    private fun loadLocalVmSummaries(context: Context): List<VmRuntimeSummaryData> {
        val vmsRoot = File(context.filesDir, "vms")
        if (!vmsRoot.isDirectory) return emptyList()
        val kvm = probeLocalCapabilities(context).kvm
        return vmsRoot.listFiles { f -> f.isDirectory }
            ?.mapNotNull { dir ->
                val pf = File(dir, "vm_profile.json")
                if (!pf.isFile) return@mapNotNull null
                runCatching {
                    val profile = gson.fromJson(pf.readText(), VmProfileData::class.java)
                    val isX86 = profile.arch == "x86_64"
                    val useKvm = !isX86 && profile.accelerator_preference == "kvm" && kvm.accessible
                    val usedBytes = dir.walkTopDown().filter { it.isFile }.sumOf { it.length() }
                    val snapCount = File(dir, "snapshots").listFiles { f -> f.extension == "json" }?.size ?: 0
                    VmRuntimeSummaryData(
                        profile = profile.copy(state = "Stopped"),
                        effective_accelerator = if (useKvm) "kvm" else "tcg",
                        execution_model_note = if (isX86) {
                            "x86-64 guest on ARM64 host ALWAYS uses QEMU TCG binary emulation; native performance is NOT possible."
                        } else if (useKvm) {
                            "ARM64 guest with /dev/kvm hardware virtualization available."
                        } else {
                            "ARM64 guest running under QEMU multi-threaded TCG software translation (no /dev/kvm)."
                        },
                        storage_used_bytes = usedBytes,
                        snapshot_count = snapCount
                    )
                }.getOrNull()
            }
            ?.sortedBy { it.profile.id }
            ?: emptyList()
    }

    private fun createLocalVmFallback(
        context: Context,
        id: String,
        name: String,
        arch: String,
        memoryMb: Int,
        cpus: Int,
        diskSizeMb: Long,
        diskFormat: String,
        isoPath: String?,
        networkMode: String,
        audioBackend: String,
        displayBackend: String,
        firmwareKind: String?
    ): Result<VmRuntimeSummaryData> = runCatching {
        val existing = loadLocalVmSummaries(context)
        if (existing.any { it.profile.id == id }) error("VM '$id' already exists")
        val usedVnc = existing.map { it.profile.display.vnc_display }.toSet()
        val usedQmp = existing.map { it.profile.qmp_port }.toSet()
        val vncDisp = (10..199).first { it !in usedVnc }
        val qmpPort = (4444..4643).first { it !in usedQmp }
        val vmDir = File(context.filesDir, "vms/$id").apply { mkdirs() }
        val ext = if (diskFormat == "qcow2") "qcow2" else "img"
        val diskFile = File(vmDir, "disk0.$ext")
        createLocalDiskFile(diskFile, diskFormat, diskSizeMb)

        val isArm64 = arch == "aarch64"
        val profile = VmProfileData(
            id = id,
            name = name,
            arch = arch,
            machine_type = if (isArm64) "virt,gic-version=3" else "q35",
            cpu = VmCpuConfigData(cores = cpus, model = if (isArm64) "cortex-a72" else "max"),
            memory_mb = memoryMb,
            accelerator_preference = "tcg_multi",
            firmware = VmFirmwareConfigData(
                kind = firmwareKind ?: if (isArm64) "aavmf_uefi" else "ovmf_uefi",
                vars_path = File(vmDir, if (isArm64) "AAVMF_VARS.fd" else "OVMF_VARS.fd").absolutePath
            ),
            disks = listOf(
                VmStorageDeviceData(
                    disk_id = "disk0",
                    path = diskFile.absolutePath,
                    format = diskFormat,
                    bus = "virtio_blk",
                    size_mb = diskSizeMb,
                    boot_index = 1
                )
            ),
            network = VmNetworkConfigData(mode = networkMode),
            display = VmDisplayConfigData(
                backend = displayBackend,
                gpu_device = if (isArm64) "virtio-gpu-pci" else "virtio-vga",
                vnc_display = vncDisp
            ),
            audio = VmAudioConfigData(backend = audioBackend),
            boot = VmBootConfigData(iso_path = isoPath?.trim()?.takeIf { it.isNotEmpty() }),
            qmp_port = qmpPort,
            qemu_version = BuildConfig.QEMU_VERSION,
            created_at_epoch_ms = System.currentTimeMillis(),
            state = "Stopped",
            notes = if (isArm64) "ARM64 VirtIO guest (QEMU TCG)" else "x86-64 TCG emulated guest (non-native speed)"
        )
        File(vmDir, "vm_profile.json").writeText(gson.toJson(profile))
        loadLocalVmSummaries(context).first { it.profile.id == id }
    }

    private fun createLocalDiskFile(file: File, format: String, sizeMb: Long) {
        val virtualBytes = sizeMb * 1024L * 1024L
        if (format == "raw") {
            RandomAccessFile(file, "rw").use { it.setLength(virtualBytes) }
            return
        }
        // Minimal valid QCOW2 v3 header (64 KiB clusters) matching rust/crates/vm/src/storage.rs
        val clusterSize = 65536
        val bytesPerL1 = (clusterSize / 8L) * clusterSize
        val l1Size = ((virtualBytes + bytesPerL1 - 1) / bytesPerL1).coerceAtLeast(1L).toInt()
        val header = ByteArray(clusterSize)
        header[0] = 0x51; header[1] = 0x46; header[2] = 0x49; header[3] = 0xFB.toByte()
        writeBeInt(header, 4, 3) // version 3
        writeBeInt(header, 20, 16) // cluster_bits = 16
        writeBeLong(header, 24, virtualBytes)
        writeBeInt(header, 36, l1Size)
        writeBeLong(header, 40, 3L * clusterSize)
        writeBeLong(header, 48, clusterSize.toLong())
        writeBeInt(header, 56, 1)
        writeBeInt(header, 96, 4)
        writeBeInt(header, 100, 104)
        val refTable = ByteArray(clusterSize)
        writeBeLong(refTable, 0, 2L * clusterSize)
        val refBlock = ByteArray(clusterSize)
        for (i in 0..3) {
            refBlock[i * 2] = 0
            refBlock[i * 2 + 1] = 1
        }
        val l1Table = ByteArray(clusterSize)
        file.outputStream().use { out ->
            out.write(header)
            out.write(refTable)
            out.write(refBlock)
            out.write(l1Table)
        }
    }

    private fun writeBeInt(buf: ByteArray, offset: Int, v: Int) {
        buf[offset] = ((v ushr 24) and 0xFF).toByte()
        buf[offset + 1] = ((v ushr 16) and 0xFF).toByte()
        buf[offset + 2] = ((v ushr 8) and 0xFF).toByte()
        buf[offset + 3] = (v and 0xFF).toByte()
    }

    private fun writeBeLong(buf: ByteArray, offset: Int, v: Long) {
        for (i in 0..7) {
            buf[offset + i] = ((v ushr (56 - i * 8)) and 0xFFL).toByte()
        }
    }

    private fun sha256File(file: File): String {
        val md = MessageDigest.getInstance("SHA-256")
        file.inputStream().use { input ->
            val buf = ByteArray(64 * 1024)
            while (true) {
                val n = input.read(buf)
                if (n <= 0) break
                md.update(buf, 0, n)
            }
        }
        return md.digest().joinToString("") { "%02x".format(it) }
    }
}
