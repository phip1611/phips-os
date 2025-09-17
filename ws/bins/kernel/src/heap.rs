//! The heap (global Rust allocator) of the kernel.

use {
    core::{
        alloc::{
            GlobalAlloc,
            Layout,
        },
        cell::OnceCell,
        ptr::NonNull,
    },
    spin::Mutex as SpinMutex,
    talc::{
        ErrOnOom,
        Talc,
    },
};

/*
/// Heap size of 64 MiB.
///
/// This is enough to cover:
/// - the kernel's LOAD segments as 2 MiB mappings
/// - the boot information
/// - the page tables for loading the kernel
const HEAP_SIZE: usize = 0x2000000;
*/
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
    todo!();
}
