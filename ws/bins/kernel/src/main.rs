//! Kernel of PhipsOS.

#![no_std]
#![no_main]
#![deny(
    clippy::all,
    clippy::cargo,
    clippy::nursery,
    clippy::must_use_candidate
)]
// I can't do anything about this; fault of the dependencies
#![allow(clippy::multiple_crate_versions)]
#![deny(missing_docs)]
#![deny(missing_debug_implementations)]
#![deny(rustdoc::all)]

extern crate alloc;

use log::info;
use kernel_lib::BootInformation;
use util::paging::VirtAddress;

mod heap;
mod logger;
mod panic_handler;
mod stack;

/// Entry into the kernel.
///
/// Set's up the stack before jumping into the Rust code.
#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.entry")]
pub unsafe extern "sysv64" fn kernel_entry() -> ! {
    core::arch::naked_asm!(
        // Set up stack. Symbol comes from Rust.
        "mov (STACK_TOP), %rsp",
        "mov %rdx, %rdi",
        "call main",
        "ud2",
        options(att_syntax)
    )
}

fn main_inner(boot_information: VirtAddress) -> anyhow::Result<()> {
    logger::early_init();
    heap::init();
    let boot_information = unsafe {
        (boot_information.0 as *const VirtAddress as *const BootInformation).as_ref().unwrap()
    };
    assert_eq!(boot_information.magic, BootInformation::MAGIC);

    let mut data = core::hint::black_box([1, 2, 3, 4]);
    data[3] = 7;
    info!("Hello world from kernel: {:?}", data);
    Ok(())
}

#[unsafe(no_mangle)]
fn main(boot_information: VirtAddress) -> ! {
    main_inner(boot_information).unwrap();
    unreachable!("");
}
