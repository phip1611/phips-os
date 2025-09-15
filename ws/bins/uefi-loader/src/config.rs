use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use uefi::fs::FileSystem;
use uefi::{CStr16, cstr16};

pub const DEFAULT_CONFIG_PATH: &CStr16 = cstr16!("phips_os_config.json");

pub fn load() {}

/// The configuration of the UEFI loader config.
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    cmdline: String,
}

impl Config {
    /// Reads the configuration from disk and returns the parsed JSON.
    pub fn read_from_disk() -> anyhow::Result<Self> {
        let handle = uefi::boot::image_handle();
        let fs = uefi::boot::get_image_file_system(handle)?;
        let mut fs = FileSystem::new(fs);
        let bytes: Vec<u8> = fs
            .read(DEFAULT_CONFIG_PATH)
            .map_err(|e: uefi::fs::Error| anyhow::Error::new(e))?;

        serde_json::from_slice(bytes.as_slice())
            .map_err(|e: serde_json::Error| anyhow::Error::new(e))
    }
}
