//! The heap (global Rust allocator) of the kernel.
//!
//! The heap is baked into the ELF and therefore easily usable before any
//! parsing of the memory map.

#![allow(static_mut_refs)]

use {
    alloc::vec,
    core::{
        alloc::{
            GlobalAlloc,
            Layout,
        },
        cell::OnceCell,
        hint::black_box,
        ptr::NonNull,
    },
    spin::Mutex as SpinMutex,
    talc::{
        ErrOnOom,
        Span,
        Talc,
    },
    util::paging::{
        PAGE_SIZE,
        Page,
    },
};

/// Heap size of 32 MiB.
const HEAP_SIZE: usize = 0x2000000;

/// Heap backing memory backed into the kernel ELF.
#[used]
static mut HEAP_MEM: [Page; HEAP_SIZE / PAGE_SIZE] = [Page::ZERO; HEAP_SIZE / PAGE_SIZE];

#[global_allocator]
static HEAP_ALLOCATOR: Allocator = Allocator::new();

struct Allocator {
    inner: SpinMutex<OnceCell<Talc<ErrOnOom>>>,
}

impl Allocator {
    const fn new() -> Self {
        Self {
            inner: SpinMutex::new(OnceCell::new()),
        }
    }
}

unsafe impl GlobalAlloc for Allocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut lock = self.inner.lock();
        let talc = lock.get_mut().expect("should have been initialized");
        // SAFETY: The backing memory is valid.
        let alloc_ptr = unsafe { talc.malloc(layout).expect("should be able to allocate") };
        alloc_ptr.as_ptr()
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let mut lock = self.inner.lock();
        let talc = lock.get_mut().expect("should have been initialized");
        // SAFETY: The backing memory is valid.
        unsafe {
            talc.free(NonNull::new(ptr).unwrap(), layout);
        };
    }
}

/// Initializes the global allocator, i.e., the heap of the loader.
pub fn init() {
    HEAP_ALLOCATOR.inner.lock().get_or_init(|| {
        // SAFETY: We are protected by a lock and only do this once on valid
        // memory.
        let heap = unsafe { HEAP_MEM.as_mut_ptr() };

        let mut talc = Talc::new(ErrOnOom);
        let span = Span::from_base_size(heap.cast(), HEAP_SIZE);
        unsafe {
            talc.claim(span)
                .expect("span {span} should be valid memory")
        };
        talc
    });

    log::debug!(
        "initialized heap: allocations work: vec={:?}",
        black_box(vec![1, 2, 3])
    );
}
