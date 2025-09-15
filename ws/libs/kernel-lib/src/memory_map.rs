use core::slice;
use util::paging::VirtAddress;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned};

type MemoryMapEntryFlagsRaw = u8;

bitflags::bitflags! {
    #[repr(C)]
    #[derive(Default, Copy, Clone, Debug, PartialOrd, Ord, Eq, PartialEq, Hash)]
    pub struct MemoryMapEntryFlags: u8 {
        const EXECUTABLE = 1 << 0;
        const WRITE = 1 << 1;
        const READ = 1 << 2;
    }
}

type MemoryMapEntryTypeRaw = u8;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialOrd, Ord, Eq, PartialEq, Hash)]
pub enum MemoryMapEntryType {
    AvailableRam = 0,
    /// The kernel itself (all LOAD segments).
    Kernel = 1,
    /// Data from the OS loader, such as the page tables of the kernel and
    /// the boot information.
    LoaderData = 2,
    /// Firmware runtime information, such as UEFI runtime services.
    Firmware = 3,
    /// ACPI tables that are reclaimable memory after parsing.
    AcpiReclaim = 4,
    /// ACPI backing storage.
    AcpiNvs = 5,
    /// MMIO space or otherwise reserved regions.
    MMIO = 6,
}

impl MemoryMapEntryType {
    /// Returns the underlying raw value.
    pub fn val(self) -> u8 {
        self as _
    }

    pub fn from_raw(v: MemoryMapEntryTypeRaw) -> Option<Self> {
        match v {
            0 => Some(Self::AvailableRam),
            1 => Some(Self::Kernel),
            2 => Some(Self::LoaderData),
            3 => Some(Self::Firmware),
            4 => Some(Self::AcpiReclaim),
            5 => Some(Self::AcpiNvs),
            6 => Some(Self::MMIO),
            _ => None,
        }
    }
}

impl TryFrom<MemoryMapEntryTypeRaw> for MemoryMapEntryType {
    type Error = ();

    fn try_from(value: MemoryMapEntryTypeRaw) -> Result<Self, Self::Error> {
        Self::from_raw(value).ok_or(())
    }
}

#[repr(C)]
#[derive(
    Copy,
    Clone,
    Debug,
    PartialOrd,
    Ord,
    Eq,
    PartialEq,
    Hash,
    IntoBytes,
    FromBytes,
    KnownLayout,
    Immutable,
)]
pub struct MemoryMapEntry {
    from: u64, /* phys addr, incl. */
    length: u64,
    typ: MemoryMapEntryTypeRaw,
    prot: MemoryMapEntryFlagsRaw,
    _padding: [u8; 6],
}

impl MemoryMapEntry {
    /// Returns the type of the memory map entry.
    pub fn typ(&self) -> MemoryMapEntryType {
        self.typ.try_into().unwrap()
    }

    /// Returns the includes begin of the memory map entry.
    pub fn from(&self) -> u64 {
        self.from
    }

    /// Returns the lengh
    pub fn length(&self) -> u64 {
        self.length
    }

    /// Returns the inclusive end of the memory map entry.
    pub fn to(&self) -> u64 {
        self.from + self.length
    }

    /// Returns the protection bits of the region.
    pub fn prot(&self) -> MemoryMapEntryFlags {
        MemoryMapEntryFlags::from_bits_retain(self.prot)
    }

    /// Returns a slice to the underlying memory.
    ///
    /// # Safety
    ///
    /// The caller must be sure that the `phys_to_virt` is correct. Also, users
    /// must be very careful with handling of the memory behind the `'static'`
    /// lifetime.
    pub unsafe fn memory(
        &self,
        phys_to_virt: impl Fn(u64) -> Option<VirtAddress>,
    ) -> Option<&'static [u8]> {
        let from = phys_to_virt(self.from())?;
        let to = phys_to_virt(self.to())?;
        let len = to.0 - from.0;
        // SAFETY: we trust the ptr and the length
        let slice = unsafe { slice::from_raw_parts(from.0 as *const u8, len as usize) };
        Some(slice)
    }

    /// Returns a mutable slice to the underlying memory.
    ///
    /// # Safety
    ///
    /// The caller must be sure that the `phys_to_virt` is correct. Also, users
    /// must be very careful with handling of the memory behind the `'static'`
    /// lifetime.
    pub unsafe fn memory_mut(
        &self,
        phys_to_virt: impl Fn(u64) -> Option<VirtAddress>,
    ) -> Option<&'static mut [u8]> {
        let from = phys_to_virt(self.from())?;
        let to = phys_to_virt(self.to())?;
        let len = to.0 - from.0;
        // SAFETY: we trust the ptr and the length
        let slice = unsafe { slice::from_raw_parts_mut(from.0 as *mut u8, len as usize) };
        Some(slice)
    }
}

#[repr(C)]
#[derive(Debug, IntoBytes, FromBytes, KnownLayout, Immutable)]
pub struct MemoryMap([MemoryMapEntry]);

#[cfg(test)]
mod tests {
    use super::*;
    use std::prelude::v1::Box;

    #[test]
    fn test_to_and_from_bytes() {
        let entries = [
            MemoryMapEntry {
                from: 0x1000,
                length: 0x2000,
                typ: MemoryMapEntryType::LoaderData.val(),
                prot: (MemoryMapEntryFlags::READ | MemoryMapEntryFlags::WRITE).bits(),
                _padding: [0; 6],
            },
            MemoryMapEntry {
                from: 0x10000,
                length: 0x20000,
                typ: MemoryMapEntryType::AvailableRam.val(),
                prot: 0,
                _padding: [0; 6],
            },
            MemoryMapEntry {
                from: 0x30000,
                length: 0x40000,
                typ: MemoryMapEntryType::Kernel.val(),
                prot: MemoryMapEntryFlags::WRITE.bits(),
                _padding: [0; 6],
            },
        ];
        let entries_bytes = entries.as_bytes();
        let map = MemoryMap::ref_from_bytes(entries_bytes).unwrap();
        assert_eq!(&map.0, &entries);
        let x = Box::from(*map);
    }
}
