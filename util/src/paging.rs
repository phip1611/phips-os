//! Module for x86_64 4-level paging.

use core::ops::{Range, RangeInclusive};

pub const PAGE_SIZE: usize = 4096;
const PAGE_BITS: usize = 12;
const PAGE_BITS_MASK: usize = bit_ops::bitops_usize::create_mask(PAGE_BITS);
const LEVEL_BITS: usize = 9;
const LEVEL_BITS_MASK: usize = bit_ops::bitops_usize::create_mask(LEVEL_BITS);
/// Maximum physical address with 4-level paging.
const LIMIT_MAX_PHYS_BITS: usize = bit_ops::bitops_usize::create_mask(52);

#[derive(Copy, Clone, Debug, PartialOrd, Ord, Eq, PartialEq, Hash)]
pub struct VirtAddress(pub u64);

impl VirtAddress {
    /// Returns the index into the page table for the given level.
    ///
    /// The level must be either `1`, `2`, `3`, or `4`.
    pub fn index(&self, level: usize) -> usize {
        assert!(level > 0);
        assert!(level <= 4);
        let shift = (level - 1) * LEVEL_BITS + PAGE_BITS;
        let shift = shift as u64;
        let index = (self.0 >> shift) & (LEVEL_BITS_MASK as u64);
        index as usize
    }
}

impl From<u64> for VirtAddress {
    fn from(value: u64) -> VirtAddress {
        Self(value)
    }
}

#[derive(Clone, Debug, PartialOrd, Ord, Eq, PartialEq, Hash)]
pub struct PageTableEntryFlags {
    pub present: bool,
    pub write: bool,
    pub superuser: bool,
    pub write_through: bool,
    pub cache_disable: bool,
    pub hugepage: bool,
    pub execute_disable: bool,
}

#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Debug, Default)]
#[repr(C)]
pub struct PageTableEntry(pub u64);

impl PageTableEntry {
    pub const BIT_PRESENT: usize = 1 << 0;
    pub const BIT_WRITE: usize = 1 << 1;
    pub const BIT_SUPERUSER: usize = 1 << 2;
    pub const BIT_WRITE_THROUGH: usize = 1 << 3;
    pub const BIT_CACHE_DISABLE: usize = 1 << 4;
    /// Huge page (page size) bit. Only valid in levels 2 and 3.
    pub const BIT_HUGEPAGE: usize = 1 << 7;
    pub const BITS_PHYS_ADDR: RangeInclusive<usize> = 12..=51;
    pub const BIT_EXECUTE_DISABLE: usize = 1 << 63;

    pub fn new(phys_addr: u64, flags: PageTableEntryFlags) -> Self {
        // Start with zero
        let mut value: u64 = 0;

        if flags.present {
            value |= Self::BIT_PRESENT as u64;
        }
        if flags.write {
            value |= Self::BIT_WRITE as u64;
        }
        if flags.superuser {
            value |= Self::BIT_SUPERUSER as u64;
        }
        if flags.write_through {
            value |= Self::BIT_WRITE_THROUGH as u64;
        }
        if flags.cache_disable {
            value |= Self::BIT_CACHE_DISABLE as u64;
        }
        if flags.hugepage {
            value |= Self::BIT_HUGEPAGE as u64;
        }

        assert_eq!(phys_addr & PAGE_BITS_MASK as u64, 0);
        assert_eq!(phys_addr & (!LIMIT_MAX_PHYS_BITS as u64), 0);

        value |= phys_addr;

        if flags.execute_disable {
            value |= Self::BIT_EXECUTE_DISABLE as u64;
        }

        Self(value)
    }
}

/// Generic page (backing memory).
#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Debug)]
#[repr(C, align(4096))]
pub struct Page(pub [u8; PAGE_SIZE]);

impl Page {
    pub const ZERO: Self = Self([0; PAGE_SIZE]);

    pub fn as_ptr(&self) -> *const u8 {
        let ptr = &raw const *self;
        ptr.cast()
    }

    pub fn as_ptr_mut(&mut self) -> *mut u8 {
        let ptr = &raw mut *self;
        ptr.cast()
    }
}

impl Default for Page {
    fn default() -> Self {
        Self::ZERO
    }
}

/// Generic page table (backing memory).
#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Debug)]
#[repr(C, align(4096))]
pub struct PageTable(pub [PageTableEntry; 512]);

impl PageTable {
    pub const ZERO: Self = Self([PageTableEntry(0); 512]);

    pub fn as_page(&self) -> &Page {
        // SAFETY: same ABI and all bit patterns are valid
        unsafe { core::mem::transmute(self) }
    }
}

impl Default for PageTable {
    fn default() -> Self {
        Self::ZERO
    }
}

#[derive(Clone, PartialOrd, Ord, PartialEq, Eq, Debug)]
pub enum PhysMappingDest<'a> {
    Page(&'a Page),
    Ptr(*const u8),
    PtrMut(*mut u8),
}

impl<'a> From<&'a Page> for PhysMappingDest<'a> {
    fn from(page: &'a Page) -> Self {
        Self::Page(page)
    }
}

impl From<*const u8> for PhysMappingDest<'_> {
    fn from(ptr: *const u8) -> Self {
        Self::Ptr(ptr)
    }
}

impl From<*mut u8> for PhysMappingDest<'_> {
    fn from(ptr: *mut u8) -> Self {
        Self::PtrMut(ptr)
    }
}

impl PhysMappingDest<'_> {
    pub fn to_phys_addr(&self) -> u64 {
        match self {
            PhysMappingDest::Page(page) => page.as_ptr() as u64,
            PhysMappingDest::Ptr(ptr) => *ptr as u64,
            PhysMappingDest::PtrMut(ptr) => *ptr as u64,
        }
    }
}

/// Performs a single mapping step.
pub fn map_address_step(
    addr: VirtAddress,
    phys_src: &mut PageTable,
    phys_dest: &Page,
    level: usize,
    write: bool,
    hugepage: bool,
    execute_disable: bool,
) {
    if hugepage {
        assert!(level == 2 || level == 3);
    }

    let index = addr.index(level);
    let phys_dest = phys_dest.as_ptr() as u64;
    let flags = PageTableEntryFlags {
        present: true,
        write,
        superuser: true,
        write_through: false,
        cache_disable: false,
        hugepage,
        execute_disable,
    };
    let entry = PageTableEntry::new(phys_dest, flags);
    phys_src.0[index] = entry;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_abi() {
        assert_eq!(size_of::<Page>(), PAGE_SIZE);
        assert_eq!(align_of::<Page>(), PAGE_SIZE);
        assert_eq!(size_of::<PageTableEntry>(), 8);
        assert_eq!(align_of::<PageTableEntry>(), 8);
    }

    #[test]
    fn test_virt_address_index() {
        let addr = VirtAddress(0xffff_eeee_dead_beef);
        assert_eq!(addr.index(4), 477);
        assert_eq!(addr.index(3), 443);
        assert_eq!(addr.index(2), 245);
        assert_eq!(addr.index(1), 219);
    }
}
