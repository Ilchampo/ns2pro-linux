//! Discovering devices and reading hidraw metadata

use std::{fs, io, path::Path};

pub enum BusType {}

#[derive(Debug, Clone)]
pub struct HidDeviceInfo {
    pub devnode: std::path::PathBuf,
    pub sysfs_path: std::path::PathBuf,
    pub bus: BusType,
    pub vendor_id: u16,
    pub product_id: u16,
    pub name: Option<String>,
    pub physical_path: Option<String>,
    pub serial: Option<String>,
    pub interface_number: Option<u8>,
}

// Utility function to resolve hidraw list
fn list_hidraw_class() -> io::Result<Vec<String>> {
    let mut names = Vec::new();
    let sys_path = "/sys/class/hidraw";

    for entry in fs::read_dir(Path::new(sys_path))? {
        let entry = entry?;
        names.push(entry.file_name().to_string_lossy().into_owned());
    }

    names.sort();
    Ok(names)
}
