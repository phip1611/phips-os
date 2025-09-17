//! Abstraction over the ELF file of the kernel.

use {
    core::slice,
    elf::{
        ElfBytes,
        abi::{
            PF_R,
            PF_W,
            PF_X,
            PT_LOAD,
        },
        endian::LittleEndian,
        segment::ProgramHeader,
    },
    log::error,
    thiserror::Error,
    util::{
        paging::VirtAddress,
        sizes::TWO_MIB,
    },
};

/// Possible errors when creating a [`KernelFile`] via
/// [`KernelFile::from_bytes`].
#[derive(Debug, Error)]
pub enum KernelFileError {
    /// The file is not a valid ELF.
    #[error("kernel is not a valid ELF")]
    InvalidElf(#[from] elf::ParseError),
    /// The LOAD segments have invalid or unexpected properties (e.g., no 2 MiB alignment).
    #[error("LOAD segments have invalid properties (e.g., no 2 MiB alignment)")]
    InvalidLoadSegments,
}

/// Abstraction over the ELF file of the kernel.
#[derive(Debug)]
pub struct KernelFile<'a> {
    elf_bytes: &'a [u8],
    elf: ElfBytes<'a, LittleEndian>,
}

impl<'a> KernelFile<'a> {
    const EXPECTED_LINK_ADDR: VirtAddress = VirtAddress(0xffffffff88200000);

    /// Performs checks on the ELF.
    ///
    /// For example, this verifies the program header of each LOAD segment.
    fn check_elf(elf: &ElfBytes<'a, LittleEndian>) -> Result<(), KernelFileError> {
        let segments = elf.segments().ok_or(KernelFileError::InvalidLoadSegments)?;
        let load_segments_iter = || {
            segments
                .clone()
                .iter()
                .filter(|pr_hdr| pr_hdr.p_type == PT_LOAD)
        };

        // check: have at least one rx, one rw, one ro segment
        {
            let has_rx = load_segments_iter().any(|pr_hdr| {
                let flags = PF_R | PF_X;
                pr_hdr.p_flags & flags != 0
            });
            let has_rw = load_segments_iter().any(|pr_hdr| {
                let flags = PF_R | PF_W;
                pr_hdr.p_flags & flags != 0
            });
            let has_ro = load_segments_iter().any(|pr_hdr| pr_hdr.p_flags == PF_R);

            if !has_rx {
                error!("didn't find read-execute segment");
            }

            if !has_rw {
                error!("didn't find read-write segment");
            }

            if !has_ro {
                error!("didn't find read-only segment");
            }

            if has_ro && has_rx && has_rw {
                Ok(())
            } else {
                Err(KernelFileError::InvalidLoadSegments)
            }
        }?;

        // check: we have three LOAD segments
        {
            let count = load_segments_iter().count();
            if count != 3 {
                error!("expected exactly three LOAD segments, but has {count}",);
                return Err(KernelFileError::InvalidLoadSegments);
            }
        };

        // check: We have the expected link address.
        {
            let first = load_segments_iter().next().unwrap();
            if first.p_vaddr != Self::EXPECTED_LINK_ADDR.0 {
                error!(
                    "expected virtual address {:#x} but was {}",
                    Self::EXPECTED_LINK_ADDR.0,
                    first.p_vaddr
                );
                return Err(KernelFileError::InvalidLoadSegments);
            }
        }

        // check: all LOAD segments are aligned to 2 MiB (for huge-page mappings)
        if load_segments_iter().any(|pr_hdr| !pr_hdr.p_vaddr.is_multiple_of(TWO_MIB as u64)) {
            error!("not all LOAD segments are properly aligned for huge pages");
            return Err(KernelFileError::InvalidLoadSegments);
        }

        // check: memsize == filesize.
        // This makes the actual size of the kernel more clear and loading a
        // bit easier.
        if load_segments_iter().any(|pr_hdr| pr_hdr.p_filesz != pr_hdr.p_memsz) {
            error!("not all LOAD segments have equal mem size and file size");
            return Err(KernelFileError::InvalidLoadSegments);
        }

        // check virtual address space is contiguous
        for (pr_hdr, pr_hdr_ne) in load_segments_iter().zip(load_segments_iter().skip(1)) {
            let expected_next_vaddr = pr_hdr.p_vaddr + pr_hdr_ne.p_filesz;
            let expected_next_vaddr = expected_next_vaddr.next_multiple_of(TWO_MIB as u64);
            if expected_next_vaddr != pr_hdr_ne.p_vaddr {
                error!("LOAD segments are not contiguous in virtual memory space");
                return Err(KernelFileError::InvalidLoadSegments);
            }
        }

        Ok(())
    }

    /// Creates a new kernel file wrapper and performs checks on the provided
    /// ELF.
    pub fn from_bytes(elf_bytes: &'a [u8]) -> Result<Self, KernelFileError> {
        let elf: ElfBytes<LittleEndian> = ElfBytes::<LittleEndian>::minimal_parse(elf_bytes)?;
        Self::check_elf(&elf)?;
        Ok(Self { elf_bytes, elf })
    }

    /// Returns the segments of the ELF file.
    ///
    /// For all segments, the corresponding content is emitted as well.
    pub fn segments(&self) -> impl Iterator<Item = (ProgramHeader, &[u8])> {
        // SAFETY: Earlier, we already checked that the segments are valid.
        let segments = unsafe { self.elf.segments().unwrap_unchecked() };
        segments.into_iter().map(move |pr_hdr| {
            let data = if pr_hdr.p_offset != 0 {
                // SAFETY: We know the pointer is valid.
                let segment_ptr = unsafe {
                    self.elf_bytes
                        .as_ptr()
                        .cast::<u8>()
                        .add(pr_hdr.p_offset as usize)
                };
                // SAFETY: We will check the bounds in the next step before
                // accessing the data.
                let segment_ptr_end = unsafe { segment_ptr.add(pr_hdr.p_offset as usize) };

                // Safety checks: Is the data in range?
                {
                    let ptr_begin = self.elf_bytes.as_ptr().cast::<u8>();
                    let ptr_end = unsafe { ptr_begin.add(self.elf_bytes.len()) };
                    let data_in_range = (ptr_begin..ptr_end).contains(&segment_ptr_end);
                    assert!(data_in_range);
                }

                // SAFETY: We know the size is valid
                unsafe { slice::from_raw_parts(segment_ptr, pr_hdr.p_filesz as usize) }
            } else {
                &[]
            };

            (pr_hdr, data)
        })
    }

    /// Returns the LOAD segments of the ELF file.
    ///
    /// Filtered version of [`Self::segments`].
    pub fn load_segments(&self) -> impl Iterator<Item = (ProgramHeader, &[u8])> {
        self.segments().filter(|(hdr, _)| hdr.p_type == PT_LOAD)
    }

    /// Returns the virtual start address of the kernel.
    ///
    /// Do not confuse this with [`Self::entry`] which is not guaranteed to be
    /// the same!
    #[must_use]
    pub fn virt_start(&self) -> VirtAddress {
        // SAFETY; We checked in the constructor that we have valid segments.
        let vaddr = unsafe { self.load_segments().next().unwrap_unchecked().0.p_vaddr };
        VirtAddress(vaddr)
    }

    /// Returns the total memsize the kernel will use at runtime when it is
    /// mapped continuously into physical memory.
    #[must_use]
    pub fn total_runtime_memsize(&self) -> usize {
        // we checked in the constructor that all LOAD segments are continuous
        self.load_segments()
            .map(|(pr_hdr, _)| pr_hdr)
            // We map them as huge pages.
            .map(|pr_hdr| pr_hdr.p_memsz.next_multiple_of(TWO_MIB as u64))
            .sum::<u64>() as usize
    }

    /// Returns the address of the entry symbol.
    #[must_use]
    pub fn entry(&self) -> VirtAddress {
        self.elf.ehdr.e_entry.into()
    }
}
