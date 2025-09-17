//! Library for generic and unit-testable functionality of the kernel.

#![no_std]
#![deny(
    clippy::all,
    clippy::cargo,
    clippy::nursery,
    clippy::must_use_candidate
)]
// I can't do anything about this; fault of the dependencies
#![allow(clippy::multiple_crate_versions)]
#![allow(missing_docs)]
#![deny(missing_debug_implementations)]
#![deny(rustdoc::all)]

mod boot_information;
mod memory_map;

pub use boot_information::BootInformation;
pub use memory_map::{MemoryMapEntry, MemoryMap, MemoryMapEntryType, MemoryMapEntryFlags};

extern crate alloc;
#[cfg(test)]
extern crate std;

#[cfg(test)]
mod tests {
    // use super::*;
}
