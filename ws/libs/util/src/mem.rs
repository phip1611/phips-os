use core::{
    alloc::Layout,
    ops::{
        Deref,
        DerefMut,
        Index,
        IndexMut,
        Range,
        RangeInclusive,
    },
    slice,
};

/// An aligned buffer. Similar to `Box<[T]>` but with guaranteed alignment.
#[derive(Debug)]
pub struct AlignedBuffer<T> {
    layout: Layout,
    heap_ptr: *mut T,
    capacity: usize,
}

impl<T: Default + Clone> AlignedBuffer<T> {
    /// Creates a new aligned buffer.
    ///
    /// # Arguments
    /// - `size`: Amount of items
    /// - `alignment`: Alignment. Must be power of two.
    pub fn new(capacity: usize, alignment: usize) -> Self {
        let size = capacity * size_of::<T>();
        let layout = Layout::from_size_align(size, alignment).unwrap();
        // SAFETY: We trust the allocator.
        let heap_ptr = unsafe { alloc::alloc::alloc(layout) }.cast::<T>();
        // init data
        {
            // SAFETY: The allocation is big enough and the ptr is valid.
            let slice = unsafe { slice::from_raw_parts_mut(heap_ptr, capacity) };
            slice.fill(T::default());
        }
        Self {
            layout,
            heap_ptr,
            capacity,
        }
    }
}

impl<T> Deref for AlignedBuffer<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        // SAFETY: The allocation is big enough and the ptr is valid.
        unsafe { slice::from_raw_parts(self.heap_ptr, self.capacity) }
    }
}

impl<T> DerefMut for AlignedBuffer<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: The allocation is big enough and the ptr is valid.
        unsafe { slice::from_raw_parts_mut(self.heap_ptr, self.capacity) }
    }
}

impl<T> Index<usize> for AlignedBuffer<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        self.deref().index(index)
    }
}

impl<T> IndexMut<usize> for AlignedBuffer<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        self.deref_mut().index_mut(index)
    }
}

impl<T> Index<Range<usize>> for AlignedBuffer<T> {
    type Output = [T]; // must be a slice type

    fn index(&self, range: Range<usize>) -> &Self::Output {
        self.deref().index(range)
    }
}

impl<T> IndexMut<Range<usize>> for AlignedBuffer<T> {
    fn index_mut(&mut self, range: Range<usize>) -> &mut Self::Output {
        self.deref_mut().index_mut(range)
    }
}

impl<T> Index<RangeInclusive<usize>> for AlignedBuffer<T> {
    type Output = [T]; // must be a slice type

    fn index(&self, range: RangeInclusive<usize>) -> &Self::Output {
        self.deref().index(range)
    }
}

impl<T> IndexMut<RangeInclusive<usize>> for AlignedBuffer<T> {
    fn index_mut(&mut self, range: RangeInclusive<usize>) -> &mut Self::Output {
        self.deref_mut().index_mut(range)
    }
}

impl<T> Drop for AlignedBuffer<T> {
    fn drop(&mut self) {
        // SAFETY: Allocation was done with same properties.
        unsafe { alloc::alloc::dealloc(self.heap_ptr.cast(), self.layout) }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::sizes::TWO_MIB,
    };

    // Main test here is that miri accepts the test.
    #[test]
    fn test_aligned_buffer() {
        let mut buf = AlignedBuffer::<u8>::new(8, TWO_MIB);
        buf[0] = 42;
        buf[1] = 73;
        buf[7] = 7;

        assert_eq!(&buf[0..=2], &[42, 73, 0]);

        assert_eq!(buf.as_ptr().align_offset(TWO_MIB), 0);
    }
}
