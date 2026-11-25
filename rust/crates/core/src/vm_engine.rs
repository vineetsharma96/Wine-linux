use std::path::{Path, PathBuf};

use androidpc_vm::{
    DiskBus, DiskFormat, DiskInspectionReport, QemuEngineCapabilities, VmArch, VmGuestImageEntry,
    VmImageCatalog, VmProfile, VmRuntimeSummary, VmSnapshotMetadata,
};

use crate::{CoreError, EngineCore};

impl EngineCore {
    pub fn qemu_capabilities(&self) -> QemuEngineCapabilities {
        self.vm_supervisor.capabilities()
    }

    pub fn list_vm_summaries(&self) -> Vec<VmRuntimeSummary> {
        self.vm_supervisor.list_vm_summaries()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_vm_profile(
        &self,
        id: &str,
        name: &str,
        arch: VmArch,
        memory_mb: u32,
        cpus: u8,
        disk_size_mb: u64,
        disk_format: DiskFormat,
        iso_path: Option<PathBuf>,
    ) -> Result<VmRuntimeSummary, CoreError> {
        Ok(self.vm_supervisor.create_vm_with_options(
            id,
            name,
            arch,
            memory_mb,
            cpus,
            disk_size_mb,
            disk_format,
            iso_path,
        )?)
    }

    pub fn update_vm_profile(&self, profile: VmProfile) -> Result<VmRuntimeSummary, CoreError> {
        Ok(self.vm_supervisor.update_vm_profile(profile)?)
    }

    pub fn clone_vm(
        &self,
        source_id: &str,
        new_id: &str,
        new_name: &str,
    ) -> Result<VmRuntimeSummary, CoreError> {
        Ok(self.vm_supervisor.clone_vm(source_id, new_id, new_name)?)
    }

    pub fn start_vm_detailed(&self, id: &str) -> Result<VmRuntimeSummary, CoreError> {
        Ok(self.vm_supervisor.start_vm_detailed(id)?)
    }

    pub fn pause_vm(&self, id: &str) -> Result<VmRuntimeSummary, CoreError> {
        Ok(self.vm_supervisor.pause_vm(id)?)
    }

    pub fn resume_vm(&self, id: &str) -> Result<VmRuntimeSummary, CoreError> {
        Ok(self.vm_supervisor.resume_vm(id)?)
    }

    pub fn restart_vm(&self, id: &str) -> Result<VmRuntimeSummary, CoreError> {
        Ok(self.vm_supervisor.restart_vm(id)?)
    }

    pub fn graceful_shutdown_vm(&self, id: &str, timeout_ms: u64) -> Result<(), CoreError> {
        Ok(self.vm_supervisor.graceful_shutdown_vm(id, timeout_ms)?)
    }

    pub fn force_stop_vm(&self, id: &str) -> Result<(), CoreError> {
        Ok(self.vm_supervisor.force_stop_vm(id)?)
    }

    pub fn attach_vm_disk(
        &self,
        vm_id: &str,
        disk_id: &str,
        format: DiskFormat,
        size_mb: u64,
        bus: DiskBus,
    ) -> Result<VmRuntimeSummary, CoreError> {
        Ok(self
            .vm_supervisor
            .attach_disk(vm_id, disk_id, format, size_mb, bus)?)
    }

    pub fn detach_vm_disk(
        &self,
        vm_id: &str,
        disk_id: &str,
        delete_file: bool,
    ) -> Result<VmRuntimeSummary, CoreError> {
        Ok(self
            .vm_supervisor
            .detach_disk(vm_id, disk_id, delete_file)?)
    }

    pub fn resize_vm_disk(
        &self,
        vm_id: &str,
        disk_id: &str,
        new_size_mb: u64,
    ) -> Result<DiskInspectionReport, CoreError> {
        Ok(self
            .vm_supervisor
            .resize_disk(vm_id, disk_id, new_size_mb)?)
    }

    pub fn import_vm_disk(
        &self,
        vm_id: &str,
        source_path: &Path,
        disk_id: &str,
        expected_sha256: Option<&str>,
    ) -> Result<VmRuntimeSummary, CoreError> {
        Ok(self
            .vm_supervisor
            .import_disk_image(vm_id, source_path, disk_id, expected_sha256)?)
    }

    pub fn inspect_vm_disks(&self, vm_id: &str) -> Result<Vec<DiskInspectionReport>, CoreError> {
        Ok(self.vm_supervisor.inspect_vm_disks(vm_id)?)
    }

    pub fn list_vm_snapshots(&self, vm_id: &str) -> Result<Vec<VmSnapshotMetadata>, CoreError> {
        Ok(self.vm_supervisor.list_snapshots(vm_id)?)
    }

    pub fn create_vm_snapshot(
        &self,
        vm_id: &str,
        snapshot_id: &str,
        name: &str,
        description: &str,
    ) -> Result<VmSnapshotMetadata, CoreError> {
        Ok(self
            .vm_supervisor
            .create_snapshot(vm_id, snapshot_id, name, description)?)
    }

    pub fn restore_vm_snapshot(&self, vm_id: &str, snapshot_id: &str) -> Result<(), CoreError> {
        Ok(self.vm_supervisor.restore_snapshot(vm_id, snapshot_id)?)
    }

    pub fn delete_vm_snapshot(&self, vm_id: &str, snapshot_id: &str) -> Result<(), CoreError> {
        Ok(self.vm_supervisor.delete_snapshot(vm_id, snapshot_id)?)
    }

    pub fn get_vm_logs(&self, vm_id: &str) -> Result<Vec<String>, CoreError> {
        Ok(self.vm_supervisor.get_vm_logs(vm_id)?)
    }

    pub fn list_official_vm_images(&self) -> Vec<VmGuestImageEntry> {
        VmImageCatalog::list_official_images()
    }
}
