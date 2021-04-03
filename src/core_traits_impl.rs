//! This module contains trait implementations from the core library for [Deferred].

use core::ops::{Deref, DerefMut, Index, IndexMut};

use crate::{Deferred, Reference, SliceLike, SlicePointerIndex};

// if a reference may be copied, then so may the corresponding [Deferred].
impl<T: Copy + Reference> Copy for Deferred<T> {}

// if a reference may be cloned, then so may the corresponding [Deferred].
impl<T: Clone + Copy + Reference> Clone for Deferred<T> {
    fn clone(&self) -> Self {
        *self
    }
}

// SAFETY: this is safe, because we merely inherit the Sync trait bounds from the Rust reference types.
unsafe impl<T: Sync + Reference> Sync for Deferred<T> {}

// SAFETY: this is safe, because we merely inherit the Send trait bounds from the Rust reference types.
unsafe impl<T: Send + Reference> Send for Deferred<T> {}

impl<T: Reference> Deref for Deferred<T> {
    type Target = T::Target;
    
    fn deref(&self) -> &Self::Target {
        // SAFETY: the pointer is valid, non-null and aligned, so this is safe.
        // SAFETY: the caller is still responsible for not giving out any
        // SAFETY: mutable references to the same place before calling deref(),
        // SAFETY: however, creating such mutable references would have to happen
        // SAFETY: through an `unsafe` block where the caller is responsible for
        // SAFETY: the guarantees that no mutable reference can co-exist when
        // SAFETY: deref() is called! hence, this is again safe.
        unsafe {
            &*self.as_ptr()
        }
    }
}

impl<T: ?Sized> DerefMut for Deferred<&mut T> {
    fn deref_mut(&mut self) -> &mut T {
        // SAFETY: the pointer is valid, non-null and aligned, so this is safe.
        // SAFETY: the caller is still responsible for not giving out any mutable
        // SAFETY: or immutable references to the same place before calling deref_mut(),
        // SAFETY: however, creating such references would have to happen
        // SAFETY: through an `unsafe` block where the caller is responsible for
        // SAFETY: the guarantees that no references can co-exist when
        // SAFETY: deref_mut() is called! hence, this is again safe.
        unsafe {
            &mut *self.as_mut_ptr()
        }
    }
}

impl<T: ?Sized> core::fmt::Pointer for Deferred<&T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <*const T>::fmt(&self.as_ptr(), f)
    }
}
impl<T: ?Sized> core::fmt::Pointer for Deferred<&mut T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        <*mut T>::fmt(&self.as_mut_ptr(), f)
    }
}

// requires #![feature(coerce_unsized)]:
// this doesn't work, compiler panicks: error: internal compiler error: compiler/rustc_mir/src/monomorphize/collector.rs:884:22: unexpected unsized tail: [u8; 1024]
// impl<T, const N: usize> CoerceUnsized<Deferred<&[T]>> for Deferred<&[T; N]> {}
// impl<T, const N: usize> CoerceUnsized<Deferred<&[T]>> for Deferred<&mut [T; N]> {}
// impl<T, const N: usize> CoerceUnsized<Deferred<&mut [T]>> for Deferred<&mut [T; N]> {}
// requires additional #![feature(unsize)]:
// impl<T, U> CoerceUnsized<Deferred<U>> for Deferred<T>
// where
//     T: Reference,
//     T::Target: Unsize<U::Target>,
//     U: Reference,
// {}

impl<I, T> Index<I> for Deferred<T>
where
    T: Reference,
    T::Target: SliceLike,
    I: SlicePointerIndex<T::Target>,
{
    type Output = I::Output;

    fn index(&self, index: I) -> &Self::Output {
        // SAFETY: `Deferred` guarantees that the pointer is valid and safe to dereference
        unsafe {
            &*index.index(self.as_ptr())
        }
    }
}

impl<I, T> IndexMut<I> for Deferred<&mut T>
where
    T: SliceLike + ?Sized,
    I: SlicePointerIndex<T>,
{
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        // SAFETY: `Deferred` guarantees that the pointer is valid and safe to dereference
        unsafe {
            &mut *index.index_mut(self.as_mut_ptr())
        }
    }
}

impl<'a, T: ?Sized> From<Deferred<&'a mut T>> for Deferred<&'a T> {
    fn from(deferred: Deferred<&mut T>) -> Self {
        // SAFETY: downgrading from a (deferred) mutable reference
        // SAFETY: to a (deferred) immutable reference is always safe
        unsafe {
            Deferred::from_raw(deferred.as_ptr())
        }
    }
}

impl<'a, T: ?Sized> From<&'a T> for Deferred<&'a T> {
    fn from(reference: &T) -> Self {
        // SAFETY: an actual immutable reference gives us all the guarantees
        // SAFETY: demanded by the invariant of `Deferred`, so this is safe
        unsafe {
            Deferred::from_raw(reference)
        }
    }
}

impl<'a, T: ?Sized> From<&'a mut T> for Deferred<&'a mut T> {
    fn from(reference: &mut T) -> Self {
        // SAFETY: an actual mutable reference gives us all the guarantees
        // SAFETY: demanded by the invariant of `Deferred`, so this is safe
        unsafe {
            Deferred::from_raw_mut(reference)
        }
    }
}

impl<'a, T, const N: usize> From<Deferred<&'a [T; N]>> for Deferred<&'a [T]> {
    fn from(deferred: Deferred<&[T; N]>) -> Self {
        // SAFETY: we exchange one `Deferred` for another, so this is safe
        unsafe {
            Deferred::from_raw(core::ptr::slice_from_raw_parts(deferred.as_ptr() as *const T, N))
        }
    }
}

impl<'a, T, const N: usize> From<Deferred<&'a mut [T; N]>> for Deferred<&'a mut [T]> {
    fn from(deferred: Deferred<&mut [T; N]>) -> Self {
        // SAFETY: we exchange one `Deferred` for another, so this is safe
        unsafe {
            Deferred::from_raw_mut(core::ptr::slice_from_raw_parts_mut(deferred.as_mut_ptr() as *mut T, N))
        }
    }
}

impl<'a, T, const N: usize> From<Deferred<&'a mut [T; N]>> for Deferred<&'a [T]> {
    fn from(deferred: Deferred<&mut [T; N]>) -> Self {
        // SAFETY: we exchange one `Deferred` for another, so this is safe
        unsafe {
            Deferred::from_raw(core::ptr::slice_from_raw_parts(deferred.as_ptr() as *const T, N))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Defer, DeferMut, Deferred};
    use core::cell::UnsafeCell;

    /// tests for `Index` and `IndexMut` traits
    mod index {
        use super::*;

        #[test]
        fn array() {
            let mut buffer = [0u8; 1024];
            let deferred = Deferred::from(&mut buffer);
            assert_eq!(1024, (&deferred[..]).len());
            // canary triggers miri if something is wrong with the Index trait implementation
            let canary = unsafe { &mut deferred.clone_unchecked()[1000] };
            assert_eq!(23, (&deferred[1001..]).len());
            assert_eq!(&0, &deferred[0]);
            assert_eq!(5, (&deferred[5..10]).len());
            assert_eq!(10, (&deferred[..10]).len());
            assert_eq!(11, (&deferred[..=10]).len());
            assert_eq!(6, (&deferred[5..=10]).len());
            assert_eq!(0, (&deferred[0..0]).len());
            assert_eq!(1, (&deferred[0..1]).len());
            assert_eq!(&mut 0, canary);
        }
        #[test]
        fn array_mut() {
            let mut buffer = [0u8; 1024];
            let mut deferred = Deferred::from(&mut buffer);
            assert_eq!(1024, (&mut deferred[..]).len());
            // canary triggers miri if something is wrong with the IndexMut trait implementation
            let canary = unsafe { &mut deferred.clone_unchecked()[1000] };
            assert_eq!(23, (&mut deferred[1001..]).len());
            assert_eq!(&mut 0, &mut deferred[0]);
            assert_eq!(5, (&mut deferred[5..10]).len());
            assert_eq!(10, (&mut deferred[..10]).len());
            assert_eq!(11, (&mut deferred[..=10]).len());
            assert_eq!(6, (&mut deferred[5..=10]).len());
            assert_eq!(0, (&mut deferred[0..0]).len());
            assert_eq!(1, (&mut deferred[0..1]).len());
            assert_eq!(&mut 0, canary);
        }
        #[test]
        fn slice() {
            let mut buffer = [0u8; 1024];
            let deferred = Deferred::from(&mut buffer[..]);
            let _x = &deferred[..];
            // canary triggers miri if something is wrong with the Index trait implementation
            // this triggers miri on stable rust for now, until the `slice_ptr_len` feature lands.
            // see <https://github.com/rust-lang/rust/issues/71146>.
            #[cfg(feature = "slice_ptr_len")]
            let canary = unsafe { &mut deferred.clone_unchecked()[1000] };
            assert_eq!(23, (&deferred[1001..]).len());
            assert_eq!(&0, &deferred[0]);
            assert_eq!(5, (&deferred[5..10]).len());
            assert_eq!(10, (&deferred[..10]).len());
            assert_eq!(11, (&deferred[..=10]).len());
            assert_eq!(6, (&deferred[5..=10]).len());
            assert_eq!(0, (&deferred[0..0]).len());
            assert_eq!(1, (&deferred[0..1]).len());
            #[cfg(feature = "slice_ptr_len")]
            assert_eq!(&mut 0, canary);
        }
        #[test]
        fn slice_mut() {
            let mut buffer = [0u8; 1024];
            let mut deferred = Deferred::from(&mut buffer[..]);
            let _x = &mut deferred[..];
            // canary triggers miri if something is wrong with the IndexMut trait implementation
            // this triggers miri on stable rust for now, until the `slice_ptr_len` feature lands.
            // see <https://github.com/rust-lang/rust/issues/71146>.
            #[cfg(feature = "slice_ptr_len")]
            let canary = unsafe { &mut deferred.clone_unchecked()[1000] };
            assert_eq!(23, (&mut deferred[1001..]).len());
            assert_eq!(&mut 0, &mut deferred[0]);
            assert_eq!(5, (&mut deferred[5..10]).len());
            assert_eq!(10, (&mut deferred[..10]).len());
            assert_eq!(11, (&mut deferred[..=10]).len());
            assert_eq!(6, (&mut deferred[5..=10]).len());
            assert_eq!(0, (&mut deferred[0..0]).len());
            assert_eq!(1, (&mut deferred[0..1]).len());
            #[cfg(feature = "slice_ptr_len")]
            assert_eq!(&mut 0, canary);
        }
    }

    /// tests for the `From` trait
    mod from  {
        use super::*;
        #[test]
        fn from_ref() {
            let buffer = [0u8; 1024];
            let _deferred = Deferred::from(&buffer);
            let _deferred: Deferred<&[u8]> = Deferred::from(&buffer[..]);
        }
        #[test]
        fn from_mut() {
            let mut buffer = [0u8; 1024];
            let _deferred = Deferred::from(&mut buffer);
            let _deferred: Deferred<&mut [u8]> = Deferred::from(&mut buffer[..]);
        }
        #[test]
        fn ref_array_to_slice() {
            let buffer = UnsafeCell::new([0u8; 1024]);
            let deferred = buffer.defer();
            let _deferred_slice: Deferred<&[u8]> = deferred.into();
            let _deferred_slice: Deferred<&[u8]> = Deferred::from(deferred);
        }
        #[test]
        fn mut_array_to_slice() {
            let buffer = UnsafeCell::new([0u8; 1024]);
            let deferred = unsafe { buffer.defer_mut() };
            let _deferred_slice: Deferred<&mut [u8]> = deferred.into();
            let deferred = unsafe { buffer.defer_mut() };
            let _deferred_slice: Deferred<&mut [u8]> = Deferred::from(deferred);
        }
        #[test]
        fn mut_array_to_ref_slice() {
            let buffer = UnsafeCell::new([0u8; 1024]);
            let deferred = unsafe { buffer.defer_mut() };
            let _deferred_slice: Deferred<&[u8]> = deferred.into();
            let deferred = unsafe { buffer.defer_mut() };
            let _deferred_slice: Deferred<&[u8]> = Deferred::from(deferred);
        }
        #[test]
        fn mut_to_ref() {
            let buffer = UnsafeCell::new([0u8; 1024]);
            let deferred = unsafe { buffer.defer_mut() };
            let _deferred_slice: Deferred<&[u8; 1024]> = deferred.into();
            let deferred = unsafe { buffer.defer_mut() };
            let _deferred_slice: Deferred<&[u8; 1024]> = Deferred::from(deferred);
            let buffer = UnsafeCell::new(1u32);
            let deferred = unsafe { buffer.defer_mut() };
            let _deferred_u32: Deferred<&u32> = deferred.into();
            let deferred = unsafe { buffer.defer_mut() };
            let _deferred_u32: Deferred<&u32> = Deferred::from(deferred);
        }
    }
}