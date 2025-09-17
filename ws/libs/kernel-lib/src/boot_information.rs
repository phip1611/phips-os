// TODO remove
#![allow(unused)]

use {
    crate::memory_map::MemoryMapEntry,
    core::num::NonZeroU64,
    zerocopy::{
        FromBytes,
        Immutable,
        IntoBytes,
    },
};

/// The boot information of PhipsBoot kernel.
///
/// It is guaranteed to be no larger than two MiB, i.e., one huge page.
///
/// # Structure (raw bytes)
/// ```text
/// - u64: Magic
/// - u64: Total Length
/// - u64: Length of command line
/// - [u8]: UTF-8 command line (without terminating NUL)
/// - [u8]: Optional alignment to 8-byte boundary
/// - u64: Physical Address of UEFI System Table
/// - u64: Physical Address of UEFI Image Handle
/// - u64: Number of memory map entries
/// - [mmap entry]:
/// - [u8]: Optional alignment to 8-byte boundary
/// ```
#[repr(C)]
#[derive(Debug, FromBytes, IntoBytes, Immutable)]
pub struct BootInformation {
    pub magic: u64,
    version: u32,
    length: u32,
    cmdline_len: u32,
    /// UTF-8 without terminating NUL.
    cmdline: [u8; 2048],
    _pad1: [u8; 4],
    rsdp_addr: Option<NonZeroU64>,
    efi_system_table: Option<NonZeroU64>,
    efi_image_handle: Option<NonZeroU64>,
    _pad2: [u8; 4],
    mmap_n: u32,
    // TODO make the mmap dynamically sized?!
    mmap: [MemoryMapEntry; 1024],
}

impl BootInformation {
    pub const MAGIC: u64 = 0xdead_beef_1337_1337;
    pub const MAX_CMDLINE_LEN: usize = 4096;

    pub fn new(cmdline: &str) -> Self {
        let mut cmdline_buffer = [0; 2048];
        cmdline_buffer[0..cmdline.len()].copy_from_slice(cmdline.as_bytes());
        let mmap = [MemoryMapEntry::default(); 1024];

        Self {
            magic: Self::MAGIC,
            version: 1,
            length: size_of::<Self>() as u32,
            cmdline_len: cmdline_buffer.len() as u32,
            cmdline: cmdline_buffer,
            _pad1: [0; 4],
            rsdp_addr: None,
            efi_system_table: None,
            efi_image_handle: None,
            _pad2: [0; 4],
            mmap_n: 0,
            mmap,
        }
    }
    /*
    /// Allocates a buffer
    pub fn allocate_buffer(cmdline_len: usize, memory_map_entries: usize) -> Box<[u8]> {}

    pub fn new_in_buffer(buffer: &mut u8, cmdline: &str, memory_map: &[MemoryMapEntry]) {
        let total_size =
            size_of::<u64>() /* magic */ + (size_of::<u64>() + cmdline.len() /* cmdline */);
        let mut buf = vec![0u8; total_size];

        let writer = Writer::new(&mut buf);

        buf.into_boxed_slice()
    }*/
}

struct Writer<'a> {
    buffer: &'a mut [u8],
    write_ptr: *mut u8,
    offset: usize,
}

impl<'a> Writer<'a> {
    /*fn new(buffer: &'a mut [u8]) -> Writer<'a> {
        Self {
            write_ptr: buffer.as_mut_ptr(),
            buffer,
        }
    }*/

    /// Writes the data to the slice. Ensure that before writing starts, the
    /// write pointer is advanced to the needed alignment of the type.
    fn write<T: IntoBytes + Immutable>(&mut self, element: &T) {
        let bytes = element.as_bytes();

        // Advance write pointer to next alignment boundary.
        {
            let align_offset = self.write_ptr.align_offset(align_of::<T>());
            self.offset += align_offset;
            assert!(self.offset < bytes.len());
            // SAFETY: We ensure that the ptr stays in the allocated range.
            self.write_ptr = unsafe { self.write_ptr.add(align_offset) };
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        util::sizes::TWO_MIB,
    };

    #[test]
    fn test_abi() {
        assert!(size_of::<BootInformation>() <= TWO_MIB)
    }
}
