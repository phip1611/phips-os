#![allow(static_mut_refs)]

use util::paging::{
    PAGE_SIZE,
    Page,
};

const STACK_SIZE: usize = 0x20000 /* 128 KiB */;

#[unsafe(no_mangle)]
static mut STACK_MEM: [Page; STACK_SIZE / PAGE_SIZE] = [Page::ZERO; STACK_SIZE / PAGE_SIZE];

/// Bottom of the stack (inclusive).
#[unsafe(no_mangle)]
#[used]
static mut STACK_BOTTOM: *const u8 = unsafe { STACK_MEM.as_ptr().cast::<u8>() };

/// Top of the stack (exclusive).
///
/// This is guaranteed to be 16 byte aligned. In fact, it is even [`PAGE_SIZE`]
/// aligned.
#[unsafe(no_mangle)]
#[used]
static mut STACK_TOP: *const u8 = unsafe { STACK_BOTTOM.add(STACK_SIZE) };
