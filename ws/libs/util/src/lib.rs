//! Generic utility library for PhipsOS.

#![no_std]

extern crate alloc;
#[cfg(test)]
extern crate std;

pub mod logging;
pub mod mem;
pub mod paging;

pub mod sizes {
    pub const FOUR_K: usize = 4096;
    pub const TWO_MIB: usize = 0x200000;
    pub const ONE_GIB: usize = 0x40000000;
}

#[cfg(test)]
mod tests {
    // use super::*;
}
