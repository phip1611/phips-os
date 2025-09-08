//! UEFI-loader for the kernel of PhipsOS.

// Note: As long as this feature is not stable, we need ot to access
// `std::os::uefi::env::*`.
#![feature(uefi_std)]
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

use std::error::Error;
use std::os::uefi as uefi_std;
use uefi::fs::FileSystem;
use uefi::proto::media::file::FileInfo;
use uefi::runtime::ResetType;
use uefi::{CStr16, Handle, Status, cstr16};

/// The path where we expect the kernel ELF to be.
const KERNEL_PATH: &CStr16 = cstr16!("kernel.elf64");

/// Performs the necessary setup code for the [`uefi`] crate.
fn setup_uefi_crate() {
    let st = uefi_std::env::system_table();
    let ih = uefi_std::env::image_handle();

    // Mandatory setup code for `uefi` crate.
    unsafe {
        uefi::table::set_system_table(st.as_ptr().cast());

        let ih = Handle::from_ptr(ih.as_ptr().cast()).unwrap();
        uefi::boot::set_image_handle(ih);
    }
}

fn load_kernel_elf_from_disk() -> anyhow::Result<Vec<u8>> {
    let handle = uefi::boot::image_handle();
    let fs = uefi::boot::get_image_file_system(handle)?;
    let mut fs: FileSystem = FileSystem::new(fs);
    fs.read(KERNEL_PATH)
        .map_err(|e: uefi::fs::Error| anyhow::Error::new(e))
}

fn loader_logic() -> anyhow::Result<()> {
    let kernel = load_kernel_elf_from_disk()?;
    println!("loaded {KERNEL_PATH}: {} bytes\r\n", kernel.len());
    Ok(())
}

fn main() -> ! {
    setup_uefi_crate();
    loader_logic().unwrap();
    loop {
        core::hint::spin_loop();
    }
    uefi::runtime::reset(ResetType::SHUTDOWN, Status::SUCCESS, None);
}
