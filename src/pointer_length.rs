/// A trait which is only implemented for pointers for which the length of the pointee
/// can be determined without creating a reference to the pointee and without accessing
/// the pointee. This means that the pointer is not dereferenced.
///
/// # Example
/// ```
/// use deferred_reference::PointerLength;
/// let array = [0usize; 1024];
/// assert_eq!(1024, PointerLength::len(core::ptr::addr_of!(array)));
/// ```
///
/// # Safety
/// This trait is unsafe. The implementor must promise never to access or create a reference
/// to the pointee.
pub unsafe trait PointerLength {
    /// Obtains the length of the pointee, without creating an intermediate reference.
    fn len(ptr: *const Self) -> usize;
}

// SAFETY: <*const [T]>::len() extracts the length from the fat pointer without
// SAFETY: dereferencing the pointer, so this is safe.
#[cfg(feature = "slice_ptr_len")]
unsafe impl<T> PointerLength for [T] {
    #[inline]
    fn len(ptr: *const Self) -> usize {
        // requires #![feature(slice_ptr_len)] at crate level
        <*const [T]>::len(ptr)
    }
}

// SAFETY: this impl does not create any references, it merely panics
#[cfg(not(feature = "slice_ptr_len"))]
unsafe impl<T> PointerLength for [T] {
    #[track_caller]
    #[cold]
    fn len(_: *const Self) -> usize {
        // without the `slice_ptr_len` feature, all we can do is panic in order to keep the implementation sound.
        panic!("calling this method on slice pointers requires the `slice_ptr_len` feature to be enabled")
    }
}

// SAFETY: the array length is known at compile time due to the `const N: usize`.
// SAFETY: the pointer is not needed nor dereferenced when returning a constant.
unsafe impl <T, const N: usize> PointerLength for [T; N] {
    #[inline]
    fn len(_: *const Self) -> usize {
        N
    }
}