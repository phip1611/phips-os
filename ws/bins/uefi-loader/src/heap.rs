//! The heap (global Rust allocator) of the loader.
//!
//! We get a big chunk of memory from UEFI boot services initially, and then
//! we use [`talc`] on that backing memory. This way, we can use allocations and
//! deallocations even after exiting the UEFI boot services.

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
    uefi::boot::{
        AllocateType,
        MemoryType,
        allocate_pages,
    },
    util::paging::PAGE_SIZE,
};

/// Heap size of 64 MiB.
///
/// This is enough to cover:
/// - the kernel's LOAD segments as 2 MiB mappings
/// - the boot information
/// - the page tables for loading the kernel
const HEAP_SIZE: usize = 0x2000000;

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
    let pages = HEAP_SIZE / PAGE_SIZE;
    let heap_start = allocate_pages(AllocateType::AnyPages, MemoryType::LOADER_DATA, pages)
        .expect("should have {pages} available pages");

    HEAP_ALLOCATOR.inner.lock().get_or_init(|| {
        let mut talc = Talc::new(ErrOnOom);
        let span = Span::from_base_size(heap_start.as_ptr(), HEAP_SIZE);
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
