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

mod logger;

use log::error;
use std::os::uefi as uefi_std;
use uefi::Handle;

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

/// Trampoline in UEFI loader to jump to kernel.
///
/// This is the only part of the loader that will be mapped in the initial page
/// tables of the loader. It is aligned to `8` bytes to prevent its
/// instructions from crossing a page boundary. The `n` (`8`) must be less or
/// equal to the size of the function.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jump_to_kernel_trampoline() -> ! {
    core::arch::naked_asm!(
        ".balign 8",
        "mov %rcx, %cr3",
        "jmp *%rdx",
        "ud2",
        options(att_syntax)
    )
}

fn main() -> ! {
    setup_uefi_crate();
    logger::init();
    std::panic::set_hook(Box::new(|panic_info| {
        error!("PANIC: {panic_info}");
    }));
    let tramponline_addr = jump_to_kernel_trampoline as u64;
    uefi_loader_lib::main(tramponline_addr).unwrap();
    loop {
        core::hint::spin_loop();
    }
    // uefi::runtime::reset(ResetType::SHUTDOWN, Status::SUCCESS, None);
}
