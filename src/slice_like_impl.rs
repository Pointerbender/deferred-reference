//! This module contains method implementations for slice-like deferred references on [Deferred].

use crate::{Deferred, PointerLength, Reference, SliceLike, SlicePointerIndex};

/// # Methods only available for deferred references to slices and arrays
/// [Deferred] overrides some of the standard methods for arrays and slices, in order to allow
/// for deferred access to disjoint indices. The idea is that if you only need a reference
/// to a specific element or subslice, then it is not necessary to create a reference to the
/// entire array or slice. The overriden methods are listed below. In addition to the overriden
/// methods listed below, [Deferred] also overrides the [Index](core::ops::Index) and
/// [IndexMut](core::ops::IndexMut) traits on slices and arrays. This allows for direct access
/// to disjoint subslices, without triggering undefined behavior, using the syntactic sugar of
/// Rust:
/// ```
/// use deferred_reference::Deferred;
/// let mut buffer = [0u8; 300];
/// let mut a = Deferred::from(&mut buffer); // a mutable deferred reference
/// let b = unsafe { a.clone_unchecked().into_ref() }; // immutable deferred reference
/// let mut c = unsafe { a.clone_unchecked() }; // another mutable deferred reference
/// let mut_ref1 = &mut a[0..100];
/// assert_eq!(&[0u8; 100], &b[100..200]);
/// c[200..].copy_from_slice(&[1u8; 100]);
/// assert_eq!(&mut [1u8; 100], &mut c[200..]);
/// assert_eq!(&mut [0u8; 100], mut_ref1);
/// ```
/// The above example also works on stable Rust, because `buffer` is an array. However, for slices
/// this will not work on stable Rust, at least not until the
/// [`slice_ptr_len`](https://github.com/rust-lang/rust/issues/71146) feature is stabilized.
/// On nightly Rust, this is already possible with slices, too. In order to work with slices
/// on stable Rust (or on nightly Rust without the unstable features disabled), you will need to
/// insert an explicit call to [Deref::deref](core::ops::Deref::deref) or
/// [DerefMut::deref_mut](core::ops::DerefMut::deref_mut) in order to reach the slice,
/// which will create a reference to the entire slice (without this extra step, you will get a panic).
/// This is made explicit like this to avoid ambiguity when a method resolves to a subslice or
/// the entire slice. Here is an example of how to use [Deferred] on stable Rust with slices,
/// under the condition that indexing operations are disjoint in lifetime (instead of disjoint
/// w.r.t. the indices):
/// ```
/// use deferred_reference::Deferred;
/// use core::ops::{Deref, DerefMut};
/// let mut buffer = [0u8; 300];
/// let mut a: Deferred<&mut [u8]> = Deferred::from(&mut buffer).into(); // a slice
/// let b = unsafe { a.clone_unchecked().into_ref() }; // immutable deferred reference
/// let mut c = unsafe { a.clone_unchecked() }; // another mutable deferred reference;
/// let mut_ref1 = &mut a.deref_mut()[0..100]; // accesses `a` for lifetime 'a
/// assert_eq!(&mut [0u8; 100], &mut mut_ref1[0..100]); // lifetime 'a ends after this statement
/// assert_eq!(&[0u8; 100], &b.deref()[100..200]); // accesses `b` for short-lived lifetime 'b
/// c.deref_mut()[200..].copy_from_slice(&[1u8; 100]); // accesses `c` for short-lived lifetime 'c
/// assert_eq!(&mut [1u8; 100], &mut c.deref_mut()[200..]); // accesses `c` for lifetime 'd
/// ```
impl<T> Deferred<T>
where
    T: Reference,
    T::Target: SliceLike,
{
    /// Obtains the length of the array or slice that this `Deferred` points to, without creating
    /// an intermediate reference to the array or slice.
    ///
    /// # Example
    /// ```
    /// use core::cell::UnsafeCell;
    /// use deferred_reference::{Defer, Deferred};
    /// let buffer = UnsafeCell::new([0u8; 1024]);
    /// let deferred: Deferred<_> = buffer.defer();
    /// assert_eq!(1024, deferred.len());
    /// ```
    ///
    /// # Panics
    /// As of yet, the length of slices (which are a dynamically sized type, unlike fixed size arrays)
    /// can only be accessed when the unstable `Cargo.toml` feature `slice_ptr_len` or `unstable` is enabled.
    /// If you call this method on a deferred slice without one of these features enabled, then this method will panic.
    /// This method will become panic-less for slices when the `slice_ptr_len` feature lands in Rust stable,
    /// see <https://github.com/rust-lang/rust/issues/71146>. It is still possible to access the length
    /// of a fixed sized array `[T; N]` without dereferencing the array in stable Rust (meaning, even
    /// without the use of unstable features and without risk of panics).
    pub fn len(&self) -> usize {
        PointerLength::len(self.as_ptr())
    }

    /// Returns a reference to an element or subslice depending on the type of
    /// index, without creating a reference to the other elements in the slice.
    ///
    /// - If given a position, returns a reference to the element at that
    ///   position or `None` if out of bounds.
    /// - If given a range, returns the subslice corresponding to that range,
    ///   or `None` if out of bounds.
    ///
    /// # Examples
    ///
    /// ```
    /// use deferred_reference::Deferred;
    /// let v = Deferred::from(&[10, 40, 30]);
    /// assert_eq!(Some(&40), v.get(1));
    /// assert_eq!(Some(&[10, 40][..]), v.get(0..2));
    /// assert_eq!(None, v.get(3));
    /// assert_eq!(None, v.get(0..4));
    /// ```
    #[inline]
    pub fn get<I>(&self, index: I) -> Option<&I::Output>
    where
        I: SlicePointerIndex<T::Target>,
    {
        index.get(self.as_ptr()).map(|ptr| {
            // SAFETY: `ptr` is checked to be in bounds, so this is safe
            unsafe { &*ptr }
        })
    }

    /// Returns a reference to an element or subslice, without doing bounds checking and without
    /// creating a reference to the other elements in the slice.
    ///
    /// For a safe alternative see [`get`].
    ///
    /// # Safety
    ///
    /// Calling this method with an out-of-bounds index is *[undefined behavior]*
    /// even if the resulting reference is not used.
    ///
    /// [`get`]: #method.get
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    ///
    /// # Examples
    ///
    /// ```
    /// use deferred_reference::Deferred;
    /// let x = Deferred::from(&[1, 2, 4]);
    ///
    /// unsafe {
    ///     assert_eq!(x.get_unchecked(1), &2);
    /// }
    /// ```
    #[inline]
    pub fn get_unchecked<I>(&self, index: I) -> &I::Output
    where
        I: SlicePointerIndex<T::Target>,
    {
        // SAFETY: the caller must uphold most of the safety requirements for `get_unchecked`;
        // SAFETY: the slice is dereferencable because `self.as_ptr()` is a safe pointer.
        // SAFETY: the returned pointer is safe because impls of `SlicePointerIndex` have to guarantee that it is.
        unsafe {
            &*index.get_unchecked(self.as_ptr())
        }
    }

    /// Divides one deferred slice into two deferred slices at an index, without doing bounds checking
    /// and without creating any intermediate references.
    ///
    /// The first will contain all indices from `[0, mid)` (excluding
    /// the index `mid` itself) and the second will contain all
    /// indices from `[mid, len)` (excluding the index `len` itself).
    ///
    /// For a safe alternative see [`split_at`].
    ///
    /// # Safety
    ///
    /// Calling this method with an out-of-bounds index is *[undefined behavior]*
    /// even if the resulting reference is not used. The caller has to ensure that
    /// `0 <= mid <= self.len()`.
    ///
    /// [`split_at`]: #method.split_at
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    ///
    /// # Examples
    ///
    /// ```
    /// use deferred_reference::Deferred;
    /// let v = [1, 2, 3, 4, 5, 6];
    /// let deferred = Deferred::new(&v);
    ///
    /// unsafe {
    ///    let (left, right) = deferred.split_at_unchecked(0);
    ///    assert_eq!(*left, []);
    ///    assert_eq!(*right, [1, 2, 3, 4, 5, 6]);
    /// }
    ///
    /// unsafe {
    ///     let (left, right) = deferred.split_at_unchecked(2);
    ///     assert_eq!(*left, [1, 2]);
    ///     assert_eq!(*right, [3, 4, 5, 6]);
    /// }
    ///
    /// unsafe {
    ///     let (left, right) = deferred.split_at_unchecked(6);
    ///     assert_eq!(*left, [1, 2, 3, 4, 5, 6]);
    ///     assert_eq!(*right, []);
    /// }
    /// ```
    #[inline]
    pub unsafe fn split_at_unchecked(&self, mid: usize) -> (Deferred<&[<T::Target as SliceLike>::Element]>, Deferred<&[<T::Target as SliceLike>::Element]>){
        // SAFETY: Caller has to check that `0 <= mid <= self.len()`.
        // SAFETY: the other invariants are then upheld by SlicePointerIndex and Deferred.
        (
            Deferred::from_raw((..mid).get_unchecked(self.as_ptr())),
            Deferred::from_raw((mid..).get_unchecked(self.as_ptr()))
        )
    }

    /// Divides one deferred slice into two deferred slices at an index,
    /// without creating any intermediate references.
    ///
    /// The first will contain all indices from `[0, mid)` (excluding
    /// the index `mid` itself) and the second will contain all
    /// indices from `[mid, len)` (excluding the index `len` itself).
    ///
    /// # Panics
    ///
    /// Panics if `mid > len`.
    ///
    /// # Examples
    ///
    /// ```
    /// use deferred_reference::Deferred;
    /// let v = [1, 2, 3, 4, 5, 6];
    /// let deferred = Deferred::new(&v);
    /// {
    ///    let (left, right) = deferred.split_at(0);
    ///    assert_eq!(*left, []);
    ///    assert_eq!(*right, [1, 2, 3, 4, 5, 6]);
    /// }
    ///
    /// {
    ///     let (left, right) = deferred.split_at(2);
    ///     assert_eq!(*left, [1, 2]);
    ///     assert_eq!(*right, [3, 4, 5, 6]);
    /// }
    ///
    /// {
    ///     let (left, right) = deferred.split_at(6);
    ///     assert_eq!(*left, [1, 2, 3, 4, 5, 6]);
    ///     assert_eq!(*right, []);
    /// }
    ///
    /// {
    ///     // this method overrides the `<[T]>::split_at` method from the core library
    ///     // if you rather have actual slices than deferred slices, insert a `deref` like so:
    ///     use core::ops::Deref;
    ///     let (left, right) /* : (&[_], &[_]) */ = deferred.deref().split_at(2);
    ///     assert_eq!(left, [1, 2]);
    ///     assert_eq!(right, [3, 4, 5, 6]);
    /// }
    /// ```
    #[inline]
    pub fn split_at(&self, mid: usize) -> (Deferred<&[<T::Target as SliceLike>::Element]>, Deferred<&[<T::Target as SliceLike>::Element]>) {
        assert!(mid <= self.len());
        // SAFETY: `[ptr; mid]` and `[mid; len]` are inside `self`, which
        // SAFETY: fulfills the requirements of `split_at_unchecked`.
        unsafe { self.split_at_unchecked(mid) }
    }
}

/// # Methods only available for deferred _mutable_ references to slices and arrays
impl<T> Deferred<&mut T>
where
    T: SliceLike + ?Sized,
{
    /// Returns a mutable reference to an element or subslice depending on the
    /// type of index (see [`get`]) or `None` if the index is out of bounds.
    /// This method will not create a reference to the other elements in the slice.
    ///
    /// [`get`]: #method.get
    ///
    /// # Examples
    ///
    /// ```
    /// use deferred_reference::Deferred;
    /// use core::ops::Deref;
    /// let mut x = [0, 1, 2];
    /// let mut x = Deferred::from(&mut x);
    ///
    /// if let Some(elem) = x.get_mut(1) {
    ///     *elem = 42;
    /// }
    /// assert_eq!(x.deref(), &[0, 42, 2]);
    /// ```
    #[inline]
    pub fn get_mut<I>(&mut self, index: I) -> Option<&mut I::Output>
    where
        I: SlicePointerIndex<T>,
    {
        index.get_mut(self.as_mut_ptr()).map(|ptr| {
            // SAFETY: `ptr` is checked to be in bounds, so this is safe
            unsafe { &mut *ptr }
        })
    }

    /// Returns a mutable reference to an element or subslice, without doing bounds checking and without
    /// creating a reference to the other elements in the slice.
    ///
    /// For a safe alternative see [`get_mut`].
    ///
    /// # Safety
    ///
    /// Calling this method with an out-of-bounds index is *[undefined behavior]*
    /// even if the resulting reference is not used.
    ///
    /// [`get_mut`]: #method.get_mut
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    ///
    /// # Examples
    ///
    /// ```
    /// use deferred_reference::Deferred;
    /// use core::ops::Deref;
    /// let mut x = [1, 2, 4];
    /// let mut x = Deferred::from(&mut x);
    ///
    /// unsafe {
    ///     let elem = x.get_unchecked_mut(1);
    ///     *elem = 13;
    /// }
    /// assert_eq!(x.deref(), &[1, 13, 4]);
    /// ```
    #[inline]
    pub unsafe fn get_unchecked_mut<I>(&mut self, index: I) -> &mut I::Output
    where
        I: SlicePointerIndex<T>,
    {
        // SAFETY: the caller must uphold the safety requirements for `get_unchecked_mut`;
        // SAFETY: the slice is dereferencable because `self` is a safe pointer.
        // SAFETY: The returned pointer is safe because impls of `SlicePointerIndex` have to guarantee that it is.
        &mut *index.get_unchecked_mut(self.as_mut_ptr())
    }

    /// Divides one deferred mutable slice into two deferred mutable slice at an index, without doing bounds checking
    /// and without creating any intermediate references.
    ///
    /// The first will contain all indices from `[0, mid)` (excluding
    /// the index `mid` itself) and the second will contain all
    /// indices from `[mid, len)` (excluding the index `len` itself).
    ///
    /// For a safe alternative see [`split_at_mut`].
    ///
    /// # Safety
    ///
    /// Calling this method with an out-of-bounds index is *[undefined behavior]*
    /// even if the resulting reference is not used. The caller has to ensure that
    /// `0 <= mid <= self.len()`.
    ///
    /// [`split_at_mut`]: #method.split_at_mut
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    ///
    /// # Examples
    ///
    /// ```
    /// use deferred_reference::Deferred;
    /// let mut v = [1, 0, 3, 0, 5, 6];
    /// let mut deferred = Deferred::new_mut(&mut v);
    /// // scoped to restrict the lifetime of the borrows
    /// unsafe {
    ///     let (mut left, mut right) = deferred.split_at_mut_unchecked(2);
    ///     assert_eq!(*left, [1, 0]);
    ///     assert_eq!(*right, [3, 0, 5, 6]);
    ///     left[1] = 2;
    ///     right[1] = 4;
    /// }
    /// assert_eq!(*deferred, [1, 2, 3, 4, 5, 6]);
    /// ```
    #[inline]
    pub unsafe fn split_at_mut_unchecked(&mut self, mid: usize) -> (Deferred<&mut [T::Element]>, Deferred<&mut [T::Element]>) {
        // SAFETY: Caller has to check that `0 <= mid <= self.len()`.
        // SAFETY: the other invariants are then upheld by SlicePointerIndex and Deferred.
        (
            Deferred::from_raw_mut((..mid).get_unchecked_mut(self.as_mut_ptr())),
            Deferred::from_raw_mut((mid..).get_unchecked_mut(self.as_mut_ptr()))
        )
    }

    /// Divides one mutable slice into two at an index.
    ///
    /// The first will contain all indices from `[0, mid)` (excluding
    /// the index `mid` itself) and the second will contain all
    /// indices from `[mid, len)` (excluding the index `len` itself).
    ///
    /// # Panics
    ///
    /// Panics if `mid > len`.
    ///
    /// # Examples
    ///
    /// ```
    /// use deferred_reference::Deferred;
    /// let mut v = [1, 0, 3, 0, 5, 6];
    /// let mut deferred = Deferred::new_mut(&mut v);
    /// let (mut left, mut right) = deferred.split_at_mut(2);
    /// assert_eq!(*left, [1, 0]);
    /// assert_eq!(*right, [3, 0, 5, 6]);
    /// left[1] = 2;
    /// right[1] = 4;
    /// assert_eq!(*deferred, [1, 2, 3, 4, 5, 6]);
    /// // this method overrides the `<[T]>::split_at_mut` method from the core library
    /// // if you rather have actual slices than deferred slices, insert a `deref_mut` like so:
    /// use core::ops::DerefMut;
    /// let (left, right) /* : (&mut [_], &mut [_]) */ = deferred.deref_mut().split_at(2);
    /// assert_eq!(*left, [1, 2]);
    /// assert_eq!(*right, [3, 4, 5, 6]);
    /// ```
    #[inline]
    pub fn split_at_mut(&mut self, mid: usize) -> (Deferred<&mut [T::Element]>, Deferred<&mut [T::Element]>) {
        assert!(mid <= self.len());
        // SAFETY: `[ptr; mid]` and `[mid; len]` are inside `self`, which
        // SAFETY: fulfills the requirements of `split_at_mut_unchecked`.
        unsafe { self.split_at_mut_unchecked(mid) }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec::Vec;
    use alloc::boxed::Box;
    use core::cell::UnsafeCell;
    use core::ops::{Deref, DerefMut};
    use crate::{DeferMut, Deferred};

    #[test]
    fn doctest1() {
        let mut buffer = [0u8; 300];
        let mut a = Deferred::from(&mut buffer); // a mutable deferred reference
        let b = unsafe { a.clone_unchecked().into_ref() }; // immutable deferred reference
        let mut c = unsafe { a.clone_unchecked() }; // another mutable deferred reference
        let mut_ref1 = &mut a[0..100];
        assert_eq!(&[0u8; 100], &b[100..200]);
        c[200..].copy_from_slice(&[1u8; 100]);
        assert_eq!(&mut [1u8; 100], &mut c[200..]);
        assert_eq!(&mut [0u8; 100], mut_ref1);
    }

    #[test]
    fn doctest2() {
        let mut buffer = [0u8; 300];
        let mut a: Deferred<&mut [u8]> = Deferred::from(&mut buffer).into(); // a mutable deferred reference
        let b = unsafe { a.clone_unchecked().into_ref() }; // immutable deferred reference
        let mut c = unsafe { a.clone_unchecked() }; // another mutable deferred reference;
        let mut_ref1 = &mut a.deref_mut()[0..100]; // accesses `a` for lifetime 'a
        assert_eq!(&mut [0u8; 100], &mut mut_ref1[0..100]); // lifetime 'a ends after this statement
        assert_eq!(&[0u8; 100], &b.deref()[100..200]); // accesses `b` for short-lived lifetime 'b
        c.deref_mut()[200..].copy_from_slice(&[1u8; 100]); // accesses `c` for short-lived lifetime 'c
        assert_eq!(&mut [1u8; 100], &mut c.deref_mut()[200..]); // accesses `c` for short-lived lifetime 'd
    }
    

    #[test]
    fn len_array() {
        let mut buffer = [0u8; 1024];
        let ptr = core::ptr::addr_of!(buffer);
        let deferred = unsafe { Deferred::from_raw(ptr) };
        assert_eq!(1024, deferred.len());

        let ptr = core::ptr::addr_of_mut!(buffer);
        let deferred = unsafe { Deferred::from_raw_mut(ptr) };
        assert_eq!(1024, deferred.len());
    }

    #[test]
    fn len_slice() {
        let mut buffer = Vec::with_capacity(1024);
        buffer.resize(1024, 0u8);
        let ptr = &buffer[..] as *const [u8];
        let deferred = unsafe { Deferred::from_raw(ptr) };
        Deferred::len(&deferred);
        assert_eq!(1024, deferred.len());

        let ptr = core::ptr::slice_from_raw_parts_mut(buffer.as_mut_ptr(), buffer.len());
        let deferred = unsafe { Deferred::from_raw_mut(ptr) };
        assert_eq!(1024, deferred.len());
    }

    /// Tests that length of arrays can be obtained without dereferencing them.
    #[test]
    fn test_array_len_ub() {
        let buffer = UnsafeCell::new([0u8; 1024]);
        // SAFETY: there are no references active whatsoever, so this is safe.
        let mut deferred = unsafe { buffer.defer_mut() };
        // SAFETY: we launder the lifetime of the mutable reference, but we promise not to alias it
        let mut_borrow = unsafe { &mut *(deferred.deref_mut() as *mut [u8; 1024]) };
        assert_eq!(1024, deferred.len()); // should not create any references to pointee
        // ensure that mutable borrow persists until end of this function:
        assert_eq!(0, mut_borrow[0]);
    }

    /// Tests that length of slices can be obtained without dereferencing them.
    #[test]
    fn test_slice_len_ub() {
        let mut vector = Vec::with_capacity(1024);
        vector.resize(1024, 0u8);
        let boxed_slice = vector.into_boxed_slice();
        // SAFETY: UnsafeCell is #[repr(transparent)] so this is safe.
        let buffer: Box<UnsafeCell<[u8]>> = unsafe { core::mem::transmute(boxed_slice) };
        // SAFETY: we won't dereference this deferred reference.
        let mut deferred = unsafe { buffer.defer_mut() };
        // SAFETY: we launder the lifetime of a mutable reference, but we promise not to alias it
        let mut_borrow = unsafe { &mut *(deferred.deref_mut() as *mut [u8]) };
        assert_eq!(1024, deferred.len()); // should not create any references to pointee
        // ensure that mutable borrow persists until end of this function:
        assert_eq!(0, mut_borrow[0]);
    }

    #[test]
    fn core_split_at_mut() {
        let mut buffer = [1, 2, 3];
        let mut_ref = &mut buffer;
        let (left, right) = mut_ref.split_at_mut(1);
        assert_eq!(&mut [1], left);
        assert_eq!(&mut [2, 3], right);
        assert_eq!([1, 2, 3], *mut_ref);
    }
}