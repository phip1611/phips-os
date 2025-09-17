//! Module for x86_64 4-level paging.

use zerocopy::{FromBytes, Immutable, IntoBytes};
use {
    crate::sizes::{
        ONE_GIB,
        TWO_MIB,
    },
    alloc::boxed::Box,
    core::{
        fmt::{
            Display,
            Formatter,
        },
        ops::{
            Index,
            IndexMut,
            RangeInclusive,
        },
    },
    log::debug,
    x86::controlregs::cr3,
};

pub const PAGE_SIZE: usize = 4096;
pub const PAGE_MASK: usize = 0xfff;
const PAGE_BITS: usize = 12;
const PAGE_BITS_MASK: usize = bit_ops::bitops_usize::create_mask(PAGE_BITS);
const LEVEL_BITS: usize = 9;
const LEVEL_BITS_MASK: usize = bit_ops::bitops_usize::create_mask(LEVEL_BITS);
/// Maximum physical address with 4-level paging.
const LIMIT_MAX_PHYS_BITS: usize = bit_ops::bitops_usize::create_mask(52);

/// Wrapper around a `u64` marking this data as physical address.
#[derive(Copy, Clone, Debug, PartialOrd, Ord, Eq, PartialEq, Hash, FromBytes, IntoBytes, Immutable)]
#[repr(transparent)]
pub struct PhysAddress(pub u64);

impl PhysAddress {}

impl From<u64> for PhysAddress {
    fn from(value: u64) -> PhysAddress {
        Self(value)
    }
}

impl Display for PhysAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#x} (phys)", self.0)
    }
}

/// Wrapper around a `u64` marking this data as virtual address.
#[derive(Copy, Clone, Debug, PartialOrd, Ord, Eq, PartialEq, Hash)]
#[repr(transparent)]
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

impl Display for VirtAddress {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:#x} (virt)", self.0)
    }
}

/// Companion for [`PageTableEntry`].
#[derive(Clone, Debug, Default, PartialOrd, Ord, Eq, PartialEq, Hash)]
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
    pub const BIT_PRESENT: u64 = 1 << 0;
    pub const BIT_WRITE: u64 = 1 << 1;
    pub const BIT_SUPERUSER: u64 = 1 << 2;
    pub const BIT_WRITE_THROUGH: u64 = 1 << 3;
    pub const BIT_CACHE_DISABLE: u64 = 1 << 4;
    /// Huge page (page size) bit. Only valid in levels 2 and 3.
    pub const BIT_HUGEPAGE: u64 = 1 << 7;
    pub const BITS_PHYS_ADDR: RangeInclusive<u64> = 12..=51;
    pub const BIT_EXECUTE_DISABLE: u64 = 1 << 63;

    pub fn new(addr: PhysAddress, flags: PageTableEntryFlags) -> Self {
        // Start with zero
        let mut value: u64 = 0;

        if flags.present {
            value |= Self::BIT_PRESENT;
        }
        if flags.write {
            value |= Self::BIT_WRITE;
        }
        if flags.superuser {
            value |= Self::BIT_SUPERUSER;
        }
        if flags.write_through {
            value |= Self::BIT_WRITE_THROUGH;
        }
        if flags.cache_disable {
            value |= Self::BIT_CACHE_DISABLE;
        }
        if flags.hugepage {
            value |= Self::BIT_HUGEPAGE;
        }

        assert_eq!(addr.0 & PAGE_BITS_MASK as u64, 0);
        assert_eq!(addr.0 & (!LIMIT_MAX_PHYS_BITS as u64), 0);

        value |= addr.0;

        if flags.execute_disable {
            value |= Self::BIT_EXECUTE_DISABLE;
        }

        Self(value)
    }

    /// Returns the underlying flags.
    pub fn flags(&self) -> PageTableEntryFlags {
        let mut flags = PageTableEntryFlags::default();

        if self.0 & Self::BIT_PRESENT != 0 {
            flags.present = true;
        }
        if self.0 & Self::BIT_WRITE != 0 {
            flags.write = true;
        }
        if self.0 & Self::BIT_SUPERUSER != 0 {
            flags.superuser = true;
        }
        if self.0 & Self::BIT_WRITE_THROUGH != 0 {
            flags.write_through = true;
        }
        if self.0 & Self::BIT_CACHE_DISABLE != 0 {
            flags.cache_disable = true;
        }
        if self.0 & Self::BIT_HUGEPAGE != 0 {
            flags.hugepage = true;
        }
        if self.0 & Self::BIT_EXECUTE_DISABLE != 0 {
            flags.execute_disable = true;
        }

        flags
    }

    /// Returns the physical address this is pointing to.
    pub fn paddr(&self) -> PhysAddress {
        let len = Self::BITS_PHYS_ADDR.end() - Self::BITS_PHYS_ADDR.start();
        let mask = bit_ops::bitops_u64::create_mask(len);
        let mask = mask << Self::BITS_PHYS_ADDR.start();
        PhysAddress(self.0 & mask)
    }
}

/// Generic page (backing memory).
#[derive(Copy, Clone, PartialOrd, Ord, PartialEq, Eq, Debug)]
#[repr(C, align(4096))]
pub struct Page(pub [u8; PAGE_SIZE]);

impl Page {
    pub const ZERO: Self = Self([0; PAGE_SIZE]);

    pub fn as_paddr(&self) -> PhysAddress {
        PhysAddress(self.as_ptr() as u64)
    }

    pub fn as_vaddr(&self) -> VirtAddress {
        VirtAddress(self.as_ptr() as u64)
    }

    pub fn as_ptr(&self) -> *const u8 {
        let ptr = &raw const *self;
        ptr.cast()
    }

    pub fn as_ptr_mut(&mut self) -> *mut u8 {
        let ptr = &raw mut *self;
        ptr.cast()
    }

    pub fn as_page_table(&self) -> &PageTable {
        // SAFETY: same ABI and all bit patterns are valid
        unsafe { core::mem::transmute(self) }
    }

    pub fn as_page_table_mut(&mut self) -> &mut PageTable {
        // SAFETY: same ABI and all bit patterns are valid
        unsafe { core::mem::transmute(self) }
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

    pub fn as_page_mut(&mut self) -> &mut Page {
        // SAFETY: same ABI and all bit patterns are valid
        unsafe { core::mem::transmute(self) }
    }
}

impl Default for PageTable {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Index<usize> for PageTable {
    type Output = PageTableEntry;

    fn index(&self, index: usize) -> &Self::Output {
        self.0.index(index)
    }
}

impl IndexMut<usize> for PageTable {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.0.index_mut(index)
    }
}

/// Performs a single mapping step.
///
/// Maps the virtual address for the given level with the given physical
/// addresses for the page table and the physical destination.
///
/// # Panics
/// If some fundamental assumptions are broken, this function panics.
pub fn map_address_step(
    vaddr: VirtAddress,
    phys_src: &mut PageTable,
    phys_dest: PhysAddress,
    level: usize,
    write: bool,
    hugepage: bool,
    execute_disable: bool,
) {
    if hugepage {
        assert!(level == 2 || level == 3);
        if level == 2 {
            assert!(phys_dest.0.is_multiple_of(TWO_MIB as u64));
        } else if level == 3 {
            assert!(phys_dest.0.is_multiple_of(ONE_GIB as u64));
        }
    }

    let index = vaddr.index(level);
    let flags = PageTableEntryFlags {
        present: true,
        write,
        superuser: true,
        write_through: false,
        cache_disable: false,
        hugepage,
        execute_disable,
    };
    debug!(
        "Mapping step: level={level},rights=r{}{}: table @ {:#x}#{:03} => {}",
        if flags.write { "w" } else { "-" },
        if flags.execute_disable { "-" } else { "x" },
        phys_src.as_page().as_ptr() as u64,
        index,
        phys_dest,
    );
    let entry = PageTableEntry::new(phys_dest, flags);
    phys_src.0[index] = entry;
}

/// Casts a virtual address as page table.
unsafe fn addr_to_page_table(addr: u64) -> &'static mut PageTable {
    let ptr = addr as *mut u8;
    assert_eq!(ptr.align_offset(align_of::<PageTable>()), 0);
    unsafe { ptr.cast::<PageTable>().as_mut().unwrap() }
}

/// Recursively maps the virtual address in the page table structure.
///
/// It either uses the existing page table structure or allocates new page
/// tables from the heap as needed.
///
/// # Arguments
/// - `root_page_table`: The root page table. Of `None`, the value of `cr3` is
///   used.
/// - `vaddr`: The virtual address to map.
/// - `phys_dest`: The physical destination of the data we want to map.
/// - `phys_to_virt`: Translates a [`PhysAddress`] to a [`VirtAddress`] that is
///   reachable.
/// - `virt_to_phys`: Translates a [`VirtAddress`] to the [`PhysAddress`].
/// - `write`: Whether the mapping is writeable.
/// - `execute`: Whether the mapping is executable.
/// - `l2_hugepage`: Whether the mapping is a L2 huge page (2 MiB).
pub fn map_address(
    root_page_table: Option<PhysAddress>,
    vaddr: VirtAddress,
    phys_dest: PhysAddress,
    phys_to_virt: impl Fn(PhysAddress) -> VirtAddress,
    virt_to_phys: impl Fn(VirtAddress) -> PhysAddress,
    write: bool,
    execute: bool,
    l2_hugepage: bool,
) {
    debug!("Recursively mapping vaddr {vaddr} to {phys_dest}");

    let min_level = if l2_hugepage {
        2 /* we map to a 2 MiB huge page */
    } else {
        1 /* we map to a 4k page */
    };

    // SAFETY: trivially safe.
    let root = root_page_table.unwrap_or_else(|| PhysAddress(unsafe { cr3() }));
    let page_table = phys_to_virt(root);

    // Changed on each iteration to point to the current page table.
    let mut current_page_table = unsafe { addr_to_page_table(page_table.0) };

    // First, we just prepare the page table structure without the final entry.
    for level in ((min_level + 1)..=4).rev() {
        let index = vaddr.index(level);
        let entry: PageTableEntry = current_page_table[index];
        if !entry.flags().present {
            let new_page_table = Box::new(PageTable::ZERO);
            let new_page_table = Box::leak(new_page_table);
            let new_page_table_vaddr = (&raw const *new_page_table) as u64;
            let new_page_table_vaddr = VirtAddress(new_page_table_vaddr);
            let new_page_table_paddr = virt_to_phys(new_page_table_vaddr);

            map_address_step(
                vaddr,
                current_page_table,
                new_page_table_paddr,
                level,
                true,
                false,
                false,
            );

            current_page_table = unsafe { addr_to_page_table(new_page_table_paddr.0) };
        } else {
            let entry = current_page_table[index];
            let next_page_table_paddr = entry.paddr();
            let next_page_table_vaddr = phys_to_virt(next_page_table_paddr);
            let next_page_table = unsafe { addr_to_page_table(next_page_table_vaddr.0) };

            current_page_table = next_page_table;
        }
    }

    // Do the final mapping step. Only here, we restrict permissions.
    map_address_step(
        vaddr,
        current_page_table,
        phys_dest,
        min_level,
        write,
        l2_hugepage,
        !execute,
    );
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
