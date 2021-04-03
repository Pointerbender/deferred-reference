use core::ptr::NonNull;

use crate::Reference;

/// A smart pointer which holds a "deferred reference" to an instance of type `T: ?Sized`.
/// It has all the properties of a normal reference (`&T` or `&mut T`),
/// except that it does not hold an actual reference. This makes it possible pass
/// around multiple deferred references in unsafe code, without triggering
/// undefined behavior due to existence of aliased mutable references. `Deferred`
/// aims to make it easier to reason about the validity and lifetime of pointers
/// during the act of dereferencing.
/// 
/// # Invariant
/// `Deferred` upholds the same guarantees as its referenceable counter-parts `&T`
/// and `&mut T` (except that it doesn't occupy an actual reference!), it is possible
/// to always dereference it:
/// * The address that `Deferred` points to is guaranteed to be properly aligned.
/// * `Deferred` is guaranteed to be non-dangling.
/// * `Deferred` is guaranteed to be non-null.
/// * `Deferred` is guaranteed to dereference to the same (stack-allocated) object.
/// * The memory that `Deferred` points to is guaranteed to be properly initialized.
/// * `Deferred` is guaranteed to be valid for the duration of its lifetime.
///
/// For mutable pointers, `Deferred<&mut T>` guarantees that no mutable reference(s) existed
/// to (any part of) the `T` instance at the time the `Deferred` was constructed. After
/// a mutable `Deferred<&mut T>` is created, mutable references may be constructed from it
/// (safely or unsafely), but the Rust aliasing rules must always be respected, meaning
/// no two live mutable references may point to overlapping regions in memory, ever.
///
/// # Safety
/// Even though it is possible to work with `Deferred` from purely safe Rust, it also offers
/// additional functionality in `unsafe` code and then the programmer must take special care when
/// dereferencing the `Deferred` or its pointers in unsafe code regarding the usual Rust rules:
/// * Don't create a mutable reference `&mut T` to regions of the memory which already
///   hold an immutable reference `&T` or a mutable reference `&mut T`.
///   The usual Rust aliasing rules still apply, even in unsafe code!
/// * Don't create any reference, `&T` or `&mut T`, to regions of the memory which
///   could be modified from other threads or processes.
/// * Don't create any mutable reference `&mut T` to regions of the memory which
///   could be aliased through a `&T` or `&mut T` from other threads or processes.
/// * Creating immutable aliases `&T` to regions of the memory is fine as long as there
///   are only readers for the same part of the slice, even if it is read from other
///   threads or processes.
#[repr(transparent)] // this is so that it can be casted to and from other pointers
pub struct Deferred<T>
where
    T: Reference,
{
    /// The raw pointer. This pointer may never dangle and must always be valid.
    ptr: NonNull<T::Target>,
}

/// # Constructors for deferred _immutable_ references
/// There exist several ways to construct a deferred immutable reference `Deferred<&T>`, listed here in 
/// order of safety (lower in the list means it's more unsafe).
/// 1. Through the [Deferred::new] method.
/// 2. Through the [From]/[Into] traits implemented for `Deferred<&T>`.
/// 3. Through the [Defer::defer](crate::Defer::defer) method on types that implement the [Defer](crate::Defer) trait.
/// 4. Through the _unsafe_ [Deferred::from_raw] method.
/// 5. Through the _extremely unsafe_ [`defer`](macro@defer) macro (not recommended).
impl<'a, T: ?Sized> Deferred<&'a T> {
    /// Construct a new deferred immutable reference from an existing immutable reference.
    /// ```
    /// use deferred_reference::Deferred;
    /// let x = [1, 2, 3];
    /// let deferred = Deferred::new(&x);
    /// assert_eq!(2, deferred[1]);
    /// ```
    pub fn new(reference: &'a T) -> Self {
        // SAFETY: an actual reference upholds the same guarantees as [Deferred], to this is safe.
        unsafe {
            Self::from_raw(reference)
        }
    }
    /// Construct a new deferred immutable reference to an instance of `T: ?Sized` from a raw pointer.
    /// This method is unsafe. For a safe alternative, use `Deferred::from(&T)` instead. If you don't
    /// have access to a reference or don't want to create one, then this method (`Deferred::from_raw`)
    /// is the method that you could use instead. Alternatively, another safe method is to call the
    /// [Defer::defer](crate::Defer::defer) method on types that implement the [Defer](crate::Defer) trait.
    ///
    /// # Safety
    /// The caller must uphold the invariant of [Deferred], which implies guaranteeing
    /// largely the same safety guarantees as for regular immutable references.
    /// Most importantly this means that the pointer must be derefenceable and may not contain
    /// any part of memory which is left uninitialized. Referencing uninitialized memory of
    /// any type is always instant undefined behavior (see the nomicon at
    /// <https://doc.rust-lang.org/nomicon/uninitialized.html> for more details on this fact).
    /// The caller must also ensure that the pointer remains valid for as long as the returned
    /// `Deferred` exists. If the pointers refers to a shared memory (mapped) region which may
    /// be modified somehow, then it is also the caller's reponsibility never to call safe
    /// methods such as [`<Deferred as Deref>::deref`](core::ops::Deref::deref). This would
    /// alias the entire memory region and is only safe if there are no writers at the same time.
    ///
    /// # Caveat
    /// The lifetime for the returned [Deferred] is inferred from its usage. To prevent accidental misuse,
    /// it's suggested to tie the lifetime to whichever source lifetime is safe in the context, such as
    /// by providing a helper function taking the lifetime of a host value, or by explicit annotation.
    /// ```
    /// use deferred_reference::Deferred;
    /// pub trait MemoryMappedBuffer {
    ///     fn as_ptr(&self) -> *const u8;
    ///     fn len(&self) -> usize;
    /// }
    /// fn defer<'a, T>(buffer: &'a T) -> Deferred<&'a [u8]>
    /// where
    ///     T: MemoryMappedBuffer
    /// {
    ///     let slice_ptr = core::ptr::slice_from_raw_parts(buffer.as_ptr(), buffer.len());
    ///     unsafe { Deferred::from_raw(slice_ptr) }
    /// }
    /// ```
    ///
    /// # Example
    /// ```
    /// use deferred_reference::Deferred;
    /// let buffer = [0u8; 1024];
    /// // SAFETY: `buffer` is not moved or mutably aliased after this.
    /// let deferred = unsafe { Deferred::from_raw(core::ptr::addr_of!(buffer)) };
    /// ```
    pub unsafe fn from_raw(ptr: *const T) -> Self {
        // note: this method must live in the impl for `&'a T`
        // otherwise Rust can't infer the type properly.
        Self {
            // note: the pointer is casted from `*const T` to `*mut T`, but this is safe because
            // this is not undefined behavior. dereferencing the `*mut T` would be UB,
            // but there is no way to do this if it was constructed from a *const T,
            // so this is still sound.
            ptr: NonNull::new_unchecked(ptr as *mut T),
        }
    }
}

/// # Constructors for deferred _mutable_ references
/// There exist several ways to construct a deferred mutable reference `Deferred<&mut T>`, listed here in 
/// order of safety (lower in the list means it's more unsafe).
/// 1. Through the [Deferred::new_mut] method.
/// 2. Through the [From]/[Into] traits implemented for `Deferred<&mut T>`.
/// 3. Through the _unsafe_ [DeferMut::defer_mut](crate::DeferMut::defer_mut) method on types that implement the [DeferMut](crate::DeferMut) trait.
/// 4. Through the _unsafe_ [Deferred::from_raw_mut] method.
/// 5. Through the _extremely unsafe_ [`defer_mut`](macro@defer_mut) macro (not recommended).
impl<'a, T: ?Sized> Deferred<&'a mut T> {
    /// Construct a new deferred mutable reference from an existing mutable reference.
    /// ```
    /// use deferred_reference::Deferred;
    /// let mut x = [1, 2, 3];
    /// let mut deferred = Deferred::new_mut(&mut x);
    /// assert_eq!(&mut 2, &mut deferred[1]);
    /// ```
    pub fn new_mut(reference: &'a mut T) -> Self {
        // SAFETY: an actual reference upholds the same guarantees as [Deferred], to this is safe.
        unsafe {
            Self::from_raw_mut(reference)
        }
    }
    /// Construct a new deferred mutable reference to an instance of `T`.
    ///
    /// # Safety
    /// The caller must uphold the invariant of [Deferred]. Most importantly this means that the pointer must be
    /// derefenceable and may not contain any part of memory which is left uninitialized. Referencing
    /// uninitialized memory of any type is always instant undefined behavior (see the nomicon at
    /// <https://doc.rust-lang.org/nomicon/uninitialized.html> for more details on this fact).
    /// On top of that, the caller must ensure that the pointer remains valid for as long
    /// as the returned `Deferred` exists and that no references to the instance exist when the [Deferred]
    /// is constructed. If the `Deferred` refers to a shared memory (mapped) region which may be modified somehow
    /// (e.g. by other threads or processes), then it is also the caller's reponsibility never to call
    /// the safe methods [`<Deferred as Deref>::deref`](core::ops::Deref::deref) or
    /// [`<Deferred as DerefMut>::deref_mut`](core::ops::DerefMut::deref_mut). This would alias the entire memory
    /// region and is only safe when there are no other writers and readers, respectively.
    ///
    /// # Caveat
    /// The lifetime for the returned `Deferred` is inferred from its usage. To prevent accidental misuse,
    /// it's suggested to tie the lifetime to whichever source lifetime is safe in the context, such as
    /// by providing a helper function taking the lifetime of a host value, or by explicit annotation.
    ///
    /// ```
    /// use deferred_reference::Deferred;
    /// /// SAFETY: `Buffer` may only be implemented on valid smart-pointers which don't own
    /// /// SAFETY: the memory. The smart pointer must also point to fully initialized memory.
    /// pub unsafe trait Buffer {
    ///     fn as_ptr(&self) -> *mut u8;
    ///     fn len(&self) -> usize;
    /// }
    /// /// The lifetime of the returned [Deferred] is bound to the lifetime of the
    /// /// mutable borrow of smart pointer `T` through the explicit lifetime 'a.
    /// fn defer_mut<'a, T>(buffer: &'a mut T) -> Deferred<&'a mut [u8]>
    /// where
    ///     T: Buffer
    /// {
    ///     let slice_ptr = core::ptr::slice_from_raw_parts_mut(buffer.as_ptr(), buffer.len());
    ///     // SAFETY: this is safe because `Deferred` occupies the only mutable reference
    ///     // SAFETY: to the smart pointer `T` for the duration of lifetime 'a, which means
    ///     // SAFETY: no other callers can safely obtain a mutable reference at the same time.
    ///     unsafe { Deferred::from_raw_mut(slice_ptr) }
    /// }
    /// ```
    /// The documentation of [DeferMut](crate::DeferMut) contains some additional examples of how to properly call
    /// [Deferred::from_raw_mut].
    pub unsafe fn from_raw_mut(ptr: *mut T) -> Deferred<&'a mut T> {
        Self {
            ptr: NonNull::new_unchecked(ptr as *mut T),
        }
    }
}

/// # Methods available on all deferred references
impl<T> Deferred<T>
where
    T: Reference,
{
    /// Obtains an immutable pointer to where the deferred reference points.
    /// This pointer can be a thin pointer if `T` is sized or a fat pointer
    /// otherwise.
    pub fn as_ptr(&self) -> *const T::Target {
        self.ptr.as_ptr() as *const _
    }

    /// Unsizes the deferred reference. This method is experimental.
    ///
    /// # Example
    /// ```
    /// use deferred_reference::{Defer, DeferMut, Deferred};
    /// use core::cell::UnsafeCell;
    /// let mut buffer = UnsafeCell::new([0u8; 1024]);
    /// let deferred_array /* : Deferred<&[u8; 1024]> */ = buffer.defer();
    /// let deferred_slice: Deferred<&[u8]> = deferred_array.unsize(); // needs an explicit type
    /// let deferred_array_mut /* : Deferred<&mut [u8; 1024]> */ = unsafe { buffer.defer_mut() };
    /// let deferred_slice_mut: Deferred<&mut [u8]> = deferred_array_mut.unsize();
    /// ```
    ///
    /// # Unstable
    /// This method requires the unstable `Cargo.toml` feature `unstable` or `coerce_unsized` to be enabled.
    /// This method may become stable once the `coerce_unsized` feature lands in stable Rust, see
    /// <https://github.com/rust-lang/rust/issues/27732>. For a stable alternative, you may also use
    /// use the [From]/[Into] traits which are implemented on [Deferred] for converting deferred arrays to
    /// deferred slices.
    #[cfg(feature = "coerce_unsized")]
    pub fn unsize<U>(self) -> Deferred<U>
    where
        U: Reference,
        NonNull<T::Target>: core::ops::CoerceUnsized<NonNull<U::Target>>, // requires #![feature(coerce_unsized)]`
    {
        Deferred {
            ptr: self.ptr,
        }
    }
}

/// # Methods available for all deferred _mutable_ references
impl<'a, T: ?Sized> Deferred<&'a mut T> {
    /// Obtains an mutable pointer to where the deferred reference points.
    /// This pointer can be a thin pointer if `T` is sized or a fat pointer
    /// otherwise.
    pub fn as_mut_ptr(&self) -> *mut T {
        self.ptr.as_ptr()
    }
    /// Make a copy of this mutable `Deferred<&'a mut T>`. The copy will have the same lifetime as `'a`.
    ///
    /// # Safety
    /// This method can be very unsafe. Cloning a mutable deferred reference will let you safely dereference it mutably
    /// afterwards (e.g. through [`<Deferred<&mut T> as DerefMut>::deref_mut`](core::ops::DerefMut::deref_mut)) from several
    /// distinct places simultaneously and this could lead to aliased mutable references which is then instant undefined behavior.
    /// That is why this function is marked as unsafe. Cloning a `Deferred<&mut T>` is not undefined behavior, because this is a
    /// smart pointer and not an actual live reference. Be very careful not to accidentally create mutable aliased references through
    /// dereferencing any `Deferred<&mut T>` after calling [`Deferred::clone_unchecked`](Deferred::clone_unchecked) on it!!!
    /// Calling [`<Deferred as Clone>::clone`](Clone::clone) on immutable deferred references (i.e. `Deferred<&T>`) is entirely safe
    /// (and all `Deferred<&T>` also implement the [Copy] trait).
    pub unsafe fn clone_unchecked(&self) -> Self {
        // SAFETY: calling `from_raw_parts_mut` is safe because the invariant of [Deferred] is respected.
        // SAFETY: still this method is unsafe by itself, see the Safety notes.
        Deferred::from_raw_mut(self.as_mut_ptr())
    }
    /// Convert this deferred mutable reference into a deferred immutable reference.
    ///
    /// # Example
    /// ```
    /// use deferred_reference::{DeferMut, Deferred};
    /// use core::cell::UnsafeCell;
    /// let buffer = UnsafeCell::new([0u8; 1024]);
    /// // SAFETY: this is safe, because this is the only deferred reference we create:
    /// let deferred_mut: Deferred<&mut [u8; 1024]> = unsafe { buffer.defer_mut() };
    /// // which we then convert into a deferred immutable reference:
    /// let deferred: Deferred<&[u8; 1024]> = deferred_mut.into_ref();
    /// ```
    pub fn into_ref(self) -> Deferred<&'a T> {
        self.into()
    }
}

#[cfg(test)]
mod tests {
    use core::cell::UnsafeCell;
    use crate::{Defer, DeferMut, Deferred};

    #[test]
    fn new() {
        let mut buffer = [0u8; 1024];
        let mut deferred = Deferred::from(&mut buffer);
        //assert_eq!(0, buffer[0]); // cannot borrow `buffer[_]` as immutable because it is also borrowed as mutable
        assert_eq!(0, deferred[0]);
        assert_eq!(&mut 0, &mut deferred[0]);
        let mut deferred2 = unsafe { deferred.clone_unchecked() };
        let tmp1: &mut [u8] = &mut deferred[10..20];
        // let tmp2 = &mut deferred2[30..40];
        tmp1[0] = 42; // UB because `tmp2` creates a new &mut reference
        // tmp2[0] = 42;
        // assert_eq!(&mut tmp1[0], &mut tmp2[0]);
        deferred2[0] = 42;
        assert_eq!(42, deferred[0]);
        assert_eq!(42, buffer[0]);
    }

    #[test]
    fn mut_ref_invalidation() {
        let buffer = UnsafeCell::new([0u8; 1024]);
        let mut deferred = unsafe { buffer.defer_mut() };
        let mut deferred2 = unsafe { deferred.clone_unchecked() };
        deferred[10] = 1; // not UB, because mutable reference is temporary
        deferred2[30] = 1; // not UB, because new mutable reference is created
        let _tmp1 = &mut deferred[10..20];
        let tmp2 = &mut deferred2[30..40];
        // tmp1[0] = 42; // UB because `tmp2` creates a new &mut reference
        tmp2[0] = 42;
        // assert_eq!(&mut tmp1[0], &mut tmp2[0]); // UB
    }

    /// Tests whether [Deferred] can be casted to pointers of other types without UB
    #[test]
    fn cast_to_ptr() {
        let mut buffer = UnsafeCell::new([0u8; 1024]);
        buffer.get_mut()[0] = 1;
        {
            let deferred = buffer.defer();
            let ptr = unsafe { *(core::ptr::addr_of!(deferred) as *const *const [u8; 1024]) };
            assert_eq!(*deferred, unsafe { *ptr });
        }
        {
            let deferred = unsafe { buffer.defer_mut() };
            let ptr = unsafe { *(core::ptr::addr_of!(deferred) as *const *mut [u8; 1024]) };
            assert_eq!(*deferred, unsafe { *ptr });
        }
        {
            let mut deferred = unsafe { buffer.defer_mut() };
            let ptr = unsafe { *(core::ptr::addr_of_mut!(deferred) as *mut *mut [u8; 1024]) };
            assert_eq!(*deferred, unsafe { *ptr });
        }
    }

    /// Tests that niche size optimization works.
    #[test]
    fn size_of() {
        assert_eq!(core::mem::size_of::<Deferred<&[u8]>>(), core::mem::size_of::<Option<Deferred<&[u8]>>>())
    }

    /// Tests that pointers point to the same address.
    #[test]
    fn as_ptr() {
        let buffer = UnsafeCell::new([0u8; 1024]);
        let deferred = buffer.defer();
        assert_eq!(deferred.as_ptr() as usize, buffer.get() as usize);
    }

    /// Tests that mutable pointers point to the same address.
    #[test]
    fn as_mut_ptr() {
        let buffer = UnsafeCell::new([0u8; 1024]);
        let deferred = unsafe { buffer.defer_mut() };
        assert_eq!(deferred.as_mut_ptr() as usize, buffer.get() as usize);
    }

    
}