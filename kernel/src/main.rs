//! Kernel of PhipsOS.

#![no_std]
#![no_main]
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

use log::info;

mod panic_handler;

/// Entry into the kernel.
///
/// Set's up the stack before jumping into the Rust code.
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kernel_entry() -> ! {
    core::arch::naked_asm!(
        // todo stack aufsetzen

        // Jump zum Kernel
        "mov $0xdeadbeef, %rax",
        "cli",
        "hlt",
        "jmp main",
        "ud2",
        options(att_syntax)
    )
}

#[unsafe(no_mangle)]
fn main() -> ! {
    let mut data = core::hint::black_box([1, 2, 3, 4]);
    data[3] = 7;
    info!("Hello world from kernel: {:?}", data);
    loop {
        core::hint::spin_loop();
    }
}
