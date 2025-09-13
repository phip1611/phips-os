//! Library for generic and unit-testable functionality of the uefi-loader.

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

mod kernel_file;

pub use kernel_file::KernelFile;

use log::debug;
use std::mem::ManuallyDrop;
use std::ops::DerefMut;
use util::mem::AlignedBuffer;
use util::paging::{PAGE_MASK, PageTable, PhysMappingDest, VirtAddress, map_address_step};
use util::sizes::TWO_MIB;

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
    trampoline_addr: u64,
) -> anyhow::Result<u64 /* addr of pml4 */> {
    let mut pt_l4 = ManuallyDrop::new(Box::new(PageTable::ZERO));
    let mut pt_l3 = ManuallyDrop::new(Box::new(PageTable::ZERO));
    let mut pt_l2 = ManuallyDrop::new(Box::new(PageTable::ZERO));

    let vaddr = kernel.virt_start();

    // generic setup
    {
        // map l4 -> l3
        map_address_step(
            vaddr,
            pt_l4.deref_mut(),
            PhysMappingDest::Page(pt_l3.as_page()),
            4,
            true,
            false,
            false,
        );
        // map l3 -> l2
        map_address_step(
            vaddr,
            pt_l3.deref_mut(),
            PhysMappingDest::Page(pt_l2.as_page()),
            3,
            true,
            false,
            false,
        );
    }

    // Huge page mappings for each segment of the kernel.
    {
        // An aligned buffer sufficient in size.
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
            map_address_step(
                VirtAddress(pr_hdr.p_vaddr),
                pt_l2.deref_mut(),
                PhysMappingDest::Addr(phys_addr),
                2,
                write,
                true,
                !execute,
            );

            dst_buffer_offset += data.len().next_multiple_of(TWO_MIB);
        }
    }

    // trampoline setup
    {
        debug!("Mapping trampoline next: at {trampoline_addr:#x}");
        let trampoline_addr = VirtAddress(trampoline_addr);
        let l4_index = trampoline_addr.index(4);
        if pt_l4.0[l4_index].flags().present {
            panic!("l4 already present; unexpected");
        }

        let mut pt_trampoline_l3 = ManuallyDrop::new(Box::new(PageTable::ZERO));
        map_address_step(
            trampoline_addr,
            pt_l4.deref_mut(),
            PhysMappingDest::Page(pt_trampoline_l3.as_page()),
            4,
            false,
            false,
            false,
        );

        let mut pt_trampoline_l2 = ManuallyDrop::new(Box::new(PageTable::ZERO));
        map_address_step(
            trampoline_addr,
            pt_trampoline_l3.deref_mut(),
            PhysMappingDest::Page(pt_trampoline_l2.as_page()),
            3,
            false,
            false,
            false,
        );

        let mut pt_trampoline_l1 = ManuallyDrop::new(Box::new(PageTable::ZERO));
        map_address_step(
            trampoline_addr,
            pt_trampoline_l2.deref_mut(),
            PhysMappingDest::Page(pt_trampoline_l1.as_page()),
            2,
            false,
            false,
            false,
        );

        let trampoline_addr_page = trampoline_addr.0 & !(PAGE_MASK as u64);
        map_address_step(
            trampoline_addr,
            pt_trampoline_l1.deref_mut(),
            PhysMappingDest::Addr(trampoline_addr_page),
            1,
            false,
            false,
            false,
        );
    }

    Ok(pt_l4.as_page().as_ptr() as u64)
}

#[cfg(test)]
mod tests {}
