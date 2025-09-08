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

use std::os::uefi as uefi_std;
use uefi::runtime::ResetType;
use uefi::{Handle, Status};

/// Performs the necessary setup code for the `uefi` crate.
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

fn main() {
    println!("Hello World from uefi_std");
    setup_uefi_crate();
    println!("UEFI-Version is {}", uefi::system::uefi_revision());
    uefi::runtime::reset(ResetType::SHUTDOWN, Status::SUCCESS, None);
}
