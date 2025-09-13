/// A wrapper type to mark the inner data as [`Send`] and [`Sync`] without
/// proofing Rust aliasing rules at runtime or compile time.
///
/// The main motivation is to enable the use of global statics in environments
/// where only one single unit of execution is used and (spin) locks would
/// create unnecessary overhead.
///
/// # Safety
///
/// Using this is only safe in environments where only one single unit of
/// execution is used at all times:
/// - no multi-threading
/// - no signals
/// - no interrupts
pub struct FakeSafe<T>(T);

impl<T> FakeSafe<T> {
    /// Wraps the provided value in a [`FakeSafe`].
    ///
    /// # Safety
    ///
    /// All usages must follow the safety section of the [type][FakeSafe].
    pub const unsafe fn new(v: T) -> FakeSafe<T> {
        FakeSafe(v)
    }

    /// Returns a reference to the underlying data.
    ///
    /// # Safety
    ///
    /// Accesses must follow the _Safety_ section of the [type][FakeSafe].
    pub const unsafe fn unsafe_deref(&self) -> &T {
        &self.0
    }

    /// Returns a mutable reference to the underlying data.
    ///
    /// # Safety
    ///
    /// Accesses must follow the _Safety_ section of the [type][FakeSafe].
    /// Users MUST NOT create more than one mutable reference at a time.
    /// Further, mutable references are only allowed when no shared reference
    /// is used.
    pub const unsafe fn unsafe_deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

unsafe impl<T> Send for FakeSafe<T> {}
unsafe impl<T> Sync for FakeSafe<T> {}
