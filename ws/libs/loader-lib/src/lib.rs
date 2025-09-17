//! Library for generic and unit-testable functionality of the uefi-loader.

#![no_std]
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
#[cfg(test)]
extern crate std;

mod boot_information;
mod kernel_file;

pub use kernel_file::KernelFile;
use {
    alloc::boxed::Box,
    core::mem::ManuallyDrop,
    log::debug,
    util::{
        mem::AlignedBuffer,
        paging::{
            PAGE_MASK,
            PageTable,
            PhysAddress,
            VirtAddress,
            map_address,
        },
        sizes::TWO_MIB,
    },
};

/// Prepares the page-tables for the kernel in ELF format.
///
/// Loads the kernels ELF segments into properly aligned memory and ensures that
/// all LOAD segments are continuous in physical memory.
///
/// The memory behind `KernelFile` can be thread afterward, if not needed for
/// other purposes.
///
/// It uses the default Rust allocator to allocate the pages.
///
/// ## Page Table Format
/// This uses x86_64 4-level page tables.
///
/// Number of page tables:
/// - 1x Level 4 (root/PML4)
/// - 1x Level 3
/// - 1x kernel RX+RW+RO (2 MiB huge pages)
/// - 1x trampoline
pub fn setup_page_tables(
    kernel: &KernelFile<'_>,
    trampoline_addr: VirtAddress,
    _boot_information: VirtAddress,
    _virt_to_phys: impl Fn(VirtAddress) -> PhysAddress,
) -> anyhow::Result<u64 /* addr of pml4 */> {
    let pt_l4 = ManuallyDrop::new(Box::new(PageTable::ZERO));

    debug!("mapping kernel");
    // Huge page mappings for each segment of the kernel.
    {
        // An aligned buffer to hold all LOAD segments.
        let dst_buffer = AlignedBuffer::<u8>::new(kernel.total_runtime_memsize(), TWO_MIB);
        let mut dst_buffer = ManuallyDrop::new(dst_buffer);

        let mut dst_buffer_offset = 0;
        let n = kernel.load_segments().count();
        for (i, (pr_hdr, data)) in kernel.load_segments().enumerate() {
            // Step 1/2: Copy segment data to aligned memory
            let end = dst_buffer_offset + data.len();
            let phys_dst = &mut dst_buffer[dst_buffer_offset..end];
            phys_dst.copy_from_slice(data);

            // Step 2/2: Create mapping to memory

            let phys_addr = phys_dst.as_ptr() as u64;
            assert!(
                phys_addr.is_multiple_of(TWO_MIB as u64),
                "{phys_addr} should be huge-page aligned"
            );

            let write = pr_hdr.p_flags & elf::abi::PF_W != 0;
            let execute = pr_hdr.p_flags & elf::abi::PF_X != 0;
            debug!(
                "Mapping LOAD segment #{}/{n} (execute={}, write={})",
                i + 1,
                execute,
                write
            );

            map_address(
                Some(pt_l4.as_page().as_paddr()),
                VirtAddress(pr_hdr.p_vaddr),
                PhysAddress(phys_addr),
                // UEFI: identity mapping
                |a| VirtAddress(a.0),
                |a| PhysAddress(a.0),
                write,
                execute,
                true,
            );

            dst_buffer_offset += data.len().next_multiple_of(TWO_MIB);
        }
    }

    debug!("Mapping trampoline at {trampoline_addr}");
    // trampoline setup
    {
        let trampoline_addr = VirtAddress(trampoline_addr.0);

        let trampoline_addr_page = trampoline_addr.0 & !(PAGE_MASK as u64);
        map_address(
            Some(pt_l4.as_page().as_paddr()),
            trampoline_addr,
            PhysAddress(trampoline_addr_page),
            // UEFI: identity mapping
            |a| VirtAddress(a.0),
            |a| PhysAddress(a.0),
            false,
            true,
            false,
        );
    }

    Ok(pt_l4.as_page().as_ptr() as u64)
}

#[cfg(test)]
mod tests {}
