use crate::memory_map::MemoryMapEntry;
use alloc::boxed::Box;
use alloc::vec;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// Raw boot information.
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
#[derive(Debug)]
pub struct BootInformationRaw([u8]);

impl BootInformationRaw {
    pub const MAGIC: u64 = 0xdead_beef_1337_1337;
    pub const MAX_CMDLINE_LEN: usize = 4096;
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
