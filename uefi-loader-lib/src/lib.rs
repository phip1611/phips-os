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
use std::arch::asm;
use std::mem::ManuallyDrop;
use std::ops::Deref;
use std::ops::DerefMut;
use elf::ElfBytes;
use elf::endian::{AnyEndian, LittleEndian};
use log::{debug, info};
use std::num::NonZero;
use std::ptr::NonNull;
use elf::abi::PT_LOAD;
use uefi::boot::{AllocateType, MemoryType, PAGE_SIZE};
use uefi::fs::FileSystem;
use uefi::{CStr16, cstr16};
use util::paging;
use util::paging::{Page, PageTable, PageTableEntry, PhysMappingDest, VirtAddress, map_address_step, PAGE_MASK};

/// The path where we expect the kernel ELF to be.
const KERNEL_PATH: &CStr16 = cstr16!("kernel.elf64");

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

/// Reallocates the provided range `bytes` to (at least) two MiB.
///
/// To get the actual data in the end, please access the data like this:
/// ```text
/// let original_bytes: &[u8] = ();
/// let (allocation, idx_begin, idx_end) = realloc_to_2mib(original_bytes);
///
/// let aligned_bytes = &allocation[offset..];
/// let aligned_bytes = &aligned_bytes[..original_bytes.len()];
/// ```
fn realloc_align_up_2mib(
    bytes: Box<[u8]>,
) -> (
    Box<[u8]>,
    usize, /* begin index */
    usize, /* end index */
) {
    const MIB2: usize = 0x200000;

    if bytes.as_ptr().align_offset(MIB2) == 0 {
        let len = bytes.len();
        (bytes, 0, len)
    } else {
        let alloc_size = bytes.len() + (MIB2 - 1);
        // Most likely unaligned allocation, but big enough to push data behind
        // the first matching aligned address.
        let alloc = vec![0u8; alloc_size];
        let mut alloc = alloc.into_boxed_slice();
        let offset = alloc.as_ptr().align_offset(MIB2);
        assert_ne!(offset, usize::MAX);

        let aligned_alloc: &mut [u8] = &mut alloc[offset..];
        let aligned_bytes: &mut [u8] = &mut aligned_alloc[..bytes.len()];
        aligned_bytes.copy_from_slice(&bytes);

        let end = offset + bytes.len();

        (alloc, offset, end)
    }
}

/// Prepares the page-tables for the kernel in ELF format.
///
/// This uses x86_64 4-level page tables.
///
/// Number of page tables:
/// - 1x Level 4 (root/PML4)
/// - 1x Level 3
/// - 1x kernel RX+RW+RO (2 MiB huge pages)
/// - 1x trampoline
fn setup_page_tables(elf_bytes: &[u8], trampoline_addr: u64) -> anyhow::Result<u64 /* addr of pml4 */> {
    let mut pt_l4 = ManuallyDrop::new(Box::new(PageTable::ZERO));
    let mut pt_l3 = ManuallyDrop::new(Box::new(PageTable::ZERO));
    let mut pt_l2 = ManuallyDrop::new(Box::new(PageTable::ZERO));

    let elf: ElfBytes<LittleEndian> = elf::ElfBytes::<LittleEndian>::minimal_parse(elf_bytes)?;
    let segments = elf.segments().expect("should have program headers");
    let first_segment = segments.iter().next().expect("should have program header");
    let vaddr = VirtAddress(first_segment.p_vaddr);

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

    // segment specific setup: huge page mappings for each segment
    {
        for (i, segment) in segments.iter().filter(|segment| segment.p_type == elf::abi::PT_LOAD).enumerate() {
            // check if we can do proper simple 2 MiB mapping
            assert_eq!(segment.p_filesz, segment.p_memsz);
            assert!(segment.p_filesz <= 0x200000 /* 2 MiB */);
            assert_eq!(segment.p_vaddr % 0x200000 /* 2 MiB */, 0);

            let addr = elf_bytes.as_ptr() as u64 + segment.p_offset;
            let write = segment.p_flags & elf::abi::PF_W != 0;
            let execute = segment.p_flags & elf::abi::PF_X != 0;
            debug!("Mapping LOAD segment #{}", i + 1);
            map_address_step(
                VirtAddress(segment.p_vaddr),
                pt_l2.deref_mut(),
                PhysMappingDest::Addr(addr),
                2,
                write,
                true,
                !execute,
            );
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

/// Executes the main logic of the loader.
///
/// ## Steps
/// 1. Find and load kernel from disk into RAM
/// 1. Prepare page-table mappings
/// 1. Prepare trampoline
/// 1. Exit boot services
/// 1. Jump to trampoline; hand-off to kernel
pub fn main(trampoline_addr: u64) -> anyhow::Result<()> {
    let kernel = load_kernel_elf_from_disk()?;
    let (kernel_allocation, idx_begin, idx_end) = realloc_align_up_2mib(kernel);
    let kernel: &[u8] = &kernel_allocation[idx_begin..idx_end];

    info!("Loaded kernel from disk: {KERNEL_PATH}",);
    debug!(
        "  bytes          : {} / {}(KiB) ",
        kernel.len(),
        kernel.len() / 1024,
    );
    debug!(
        "  allocated range: {:#?} -> {:#?}",
        kernel_allocation.as_ptr(),
        unsafe { kernel_allocation.as_ptr().add(kernel_allocation.len()) }
    );
    debug!(
        "  relocated to   : {:#?} (2 MiB aligned)",
        kernel.as_ptr(),
    );

    let pml4_addr: u64 = setup_page_tables(kernel, trampoline_addr)?;

    unsafe {
        asm!(
            "jmp *%rax",
            "ud2",
            in("rax") trampoline_addr,
            in("rcx") pml4_addr,
            // todo get from ELF
            in("rdx") 0xffffffff88200000_u64,
            options(att_syntax, noreturn),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_realloc_to_2mib() {
        const MIB2: usize = 0x200000;

        let foo = vec![1, 2, 3, 4, 5, 6, 7];
        let foo = foo.into_boxed_slice();

        let (bytes, begin, end) = realloc_align_up_2mib(foo);

        assert_eq!(bytes[0..7], [0; 7]);

        let aligned_bytes: &[u8] = &bytes[begin..end];
        assert_eq!(aligned_bytes.as_ptr().align_offset(MIB2), 0);
        assert_eq!(aligned_bytes[..7], [1, 2, 3, 4, 5, 6, 7]);
    }
}
