//! Library for generic and unit-testable functionality of the uefi-loader.

#![deny(
    clippy::all,
    clippy::cargo,
    clippy::nursery,
    clippy::must_use_candidate,
    // clippy::restriction,
    // clippy::pedantic
)]
// now allow a few rules which are denied by the above statement
// --> they are ridiculous and not necessary
#![allow(
    clippy::suboptimal_flops,
    clippy::redundant_pub_crate,
    clippy::fallible_impl_from
)]
// I can't do anything about this; fault of the dependencies
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::use_self)]
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]
#![deny(rustdoc::all)]

extern crate alloc;

use alloc::vec::Vec;
use log::info;
use uefi::fs::FileSystem;
use uefi::{CStr16, cstr16};

/// The path where we expect the kernel ELF to be.
const KERNEL_PATH: &CStr16 = cstr16!("kernel.elf64");

fn load_kernel_elf_from_disk() -> anyhow::Result<Vec<u8>> {
    let handle = uefi::boot::image_handle();
    let fs = uefi::boot::get_image_file_system(handle)?;
    let mut fs = FileSystem::new(fs);
    fs.read(KERNEL_PATH)
        .map_err(|e: uefi::fs::Error| anyhow::Error::new(e))
}

/// Executes the main logic of the loader.
///
/// ## Steps
/// 1. Find and load kernel from disk into RAM
/// 2. Prepare page-table mappings
/// 3. Prepare trampoline
/// 4. Jump to trampoline; hand-off to kernel
pub fn main() -> anyhow::Result<()> {
    let kernel = load_kernel_elf_from_disk()?;
    info!("loaded {KERNEL_PATH}: {} bytes\r\n", kernel.len());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
}
