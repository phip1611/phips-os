//! UEFI-loader for the kernel of PhipsOS.

// Note: As long as this feature is not stable, we need ot to access
// `std::os::uefi::env::*`.
#![feature(uefi_std)]
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

mod logger;

static UEFI_BOOT_SERVICES_EXITED: AtomicBool = AtomicBool::new(false);

use anyhow::Context;
use loader_lib::KernelFile;
use log::{debug, error, info};
use std::mem::ManuallyDrop;
use std::os::uefi as uefi_std;
use std::sync::atomic::{AtomicBool, Ordering};
use uefi::fs::FileSystem;
use uefi::mem::memory_map::MemoryMapOwned;
use uefi::{CStr16, Handle, cstr16};
use util::paging::VirtAddress;

/// The path on the boot volume where we expect the kernel file to be.
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

/// Loads the ELF as raw bytes from disk.
fn load_kernel_elf_from_disk() -> anyhow::Result<Box<[u8]>> {
    let handle = uefi::boot::image_handle();
    let fs = uefi::boot::get_image_file_system(handle)?;
    let mut fs = FileSystem::new(fs);
    let bytes: Vec<u8> = fs
        .read(KERNEL_PATH)
        .map_err(|e: uefi::fs::Error| anyhow::Error::new(e))?;
    Ok(bytes.into_boxed_slice())
}

/// Trampoline in UEFI loader to jump to kernel.
///
/// This is the only part of the loader that will be mapped in the initial page
/// tables of the loader.
///
/// # Alignment
/// The trampoline is aligned to `8` bytes to prevent its instructions from
/// crossing a page boundary. The `n` (`8`) must be less or equal to the size
/// of the function.
///
/// One can check the disassembly with `objdump` to verify this.
///
/// # Arguments
///
/// The arguments passed using the SystemV ABI calling convention.
/// - `new_cr3`: the new root page table
/// - `kernel_addr`: the entry point of the kernel
#[unsafe(naked)]
#[unsafe(no_mangle)]
pub unsafe extern "sysv64" fn jump_to_kernel_trampoline(
    new_cr3: u64,
    kernel_addr: VirtAddress,
) -> ! {
    core::arch::naked_asm!(
        // align:
        ".balign 8",
        "mov %rdi, %cr3",
        "jmp *%rsi",
        "ud2",
        options(att_syntax)
    )
}

fn exit_boot_services() -> ManuallyDrop<MemoryMapOwned> {
    UEFI_BOOT_SERVICES_EXITED.store(true, Ordering::SeqCst);

    // SAFETY: After that, we do not call any boot services again. We also don't
    // use UEFI allocations or deallocations.
    let mmap = unsafe { uefi::boot::exit_boot_services(None) };
    logger::exit_boot_services();
    ManuallyDrop::new(mmap)
}

fn main_inner() -> anyhow::Result<()> {
    // Early init of runtime.
    {
        setup_uefi_crate();
        logger::init();
        std::panic::set_hook(Box::new(|panic_info| {
            error!("PANIC: {panic_info}");
        }));
    }

    let file =
        load_kernel_elf_from_disk().context("should be able to load kernel file from volume")?;
    let kernel = KernelFile::from_bytes(&file).context("should be valid kernel")?;
    let trampoline_addr = jump_to_kernel_trampoline as u64;

    let new_cr3 = loader_lib::setup_page_tables(&kernel, trampoline_addr)?;
    let entry = kernel.entry();
    drop(kernel);
    drop(file);

    // -------------------------------------------------------------------------
    // No allocations etc. beyond this point.

    debug!("Exiting UEFI boot services");
    exit_boot_services();
    info!("Exited UEFI boot services");

    info!("Jumping to kernel");
    debug!("  new cr3     : {:#x}", new_cr3);
    debug!("  kernel entry: {:#x}", entry.0);
    unsafe {
        jump_to_kernel_trampoline(new_cr3, entry);
    }
}

fn main() -> ! {
    main_inner().unwrap();
    loop {
        core::hint::spin_loop();
    }
}
