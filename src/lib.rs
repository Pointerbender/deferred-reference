//! This crate helps with creating multiple mutable references to the contents of a variable without triggering undefined behavior.
//! The Rust borrow rules dictate that it is undefined behavior to create more than one mutable reference to the same region,
//! even if the mutable reference is not used. However, this can sometimes be a tad too restrictive if the programmer knows
//! that two mutable references will not overlap. Using raw pointers, it is already possible to work-around the Rust borrow rules today,
//! but this requires wizard-like skills and in-depth knowledge of handling raw pointers and this is more prone to error than
//! using Rust references. With the introduction of non-lexical lifetimes in the Rust 2018 edition, the ergonomics around references 
//! have already significantly improved, but there are still some corner-cases where the programmer wished there was some way
//! to create non-overlapping mutable references into the same location (e.g. disjoint indices of a slice or array), without
//! resorting to manually managed raw pointers. In order to aid with this, this crate introduces the concept of a "*deferred reference*"[^1].
//! A deferred reference is almost exactly like a regular reference (e.g. a `&T` or a `&mut T`), but it differs from a regular reference
//! in the following ways:
//! * A deferred reference is not an actual reference, it is merely a smart pointer tied to the lifetime of the location it points to
//!   (regular raw pointers always have a static lifetime, and can thus become dangling if the location it points to is moved or dropped).
//! * It is allowed to keep multiple deferred mutable references around (as long as these are not dereferenced in a way so that
//!   these create an overlap between a mutable reference and another (de)reference).
//!
//! A deferred reference is embodied by an instance of the [Deferred] struct, which can be used like a regular [Reference].
//! There exist only two types of references and therefore there are two flavors of [Deferred], namely `Deferred<&T>` and
//! `Deferred<&mut T>`. Both of these can be used the same way as a regular reference, because `Deferred<&T>` implements 
//! the [Deref](core::ops::Deref) trait and `Deferred<&mut T>` implements both the [Deref](core::ops::Deref) trait and the
//! [DerefMut](core::ops::DerefMut) trait.
//!
//! In order to obtain a `Deferred<&T>` or a `Deferred<&mut T>`, see the documentation at [Deferred] explaining the
//! [constructors for deferred immutable references](Deferred#constructors-for-deferred-immutable-references) and
//! [constructors for deferred mutable references](Deferred#constructors-for-deferred-mutable-references).
//!
//! # Arrays and slices
//! The [Deferred] struct also provides some interesting functionality for deferred arrays and slices, for which it implements the
//! [Index](core::ops::Index) and [IndexMut](core::ops::IndexMut) traits in a way that it only creates references to the
//! queried indices but not to the other disjoint subslices. This allows multiple threads to simultaneouslt mutate the same array
//! or slice as long as these threads don't create mutable references that overlap in index or in lifetime. Currently this functionality
//! is available on stable Rust 1.51.0 for arrays `[T; N]` thanks to the introduction of the
//! [`min_const_generics` feature](https://github.com/rust-lang/rfcs/blob/master/text/2000-const-generics.md), but to use this
//! functionality on slices `[T]`, too, nightly Rust is required until the `slice_ptr_len` feature stabilizes. For more details, see the
//! [methods available for deferred references to slices and arrays](Deferred#methods-only-available-for-deferred-references-to-slices-and-arrays).
//!
//! # Example
//! Here is an example of how one might use the [Deferred] struct:
//! ```
//! use deferred_reference::{Defer, DeferMut, Deferred};
//! use core::cell::UnsafeCell;
//! use core::ops::DerefMut;
//! let buffer = Box::new(UnsafeCell::new([0u8; 1024])); // the `Box` is optional
//! // calling defer() or defer_mut() immutably borrows `buffer` for as long
//! // as the returned `Deferred` is in use.
//! let deferred: Deferred<&[u8; 1024]> = buffer.defer();
//! // SAFETY: this is safe, because we promise not to create an overlapping mutable reference
//! let mut deferred_mut: Deferred<&mut [u8; 1024]> = unsafe { buffer.defer_mut() };
//! // both `deferred` and `deferred_mut` can be safely immutably dereferenced simultaneously:
//! assert_eq!(&deferred[0], &deferred_mut[0]);
//! // we can mutate the `buffer` through `deferred_mut` as any other array.
//! // this implicity creates a temporary mutable reference into the array inside `buffer`.
//! // even though an immutable reference to `buffer` exists, this is okay because
//! // the inner array sits inside an `UnsafeCell` which allows interior mutability:
//! deferred_mut[0] = 42; 
//! // and observe the change through `deferred`:
//! assert_eq!(deferred[0], 42);
//! // all this time, both deferred references are alive, but because
//! // these are not actual references, this doesn't violate the Rust borrow
//! // rules and this is not undefined behavior. The lifetimes of the mutable
//! // and immutable references derived from the Deferred do not overlap,
//! // so the Rust borrow rules are respected all this time.
//! assert_eq!(&deferred[0], &deferred_mut[0]);
//! // this also works for multiple deferred mutable references!
//! // SAFETY: this is safe, because we promise not to create overlapping references
//! let mut deferred_mut2 = unsafe { buffer.defer_mut() };
//! // we can mutate the buffer through 2 distinct deferred mutable references
//! // (as long as we don't do this at the same time!)
//! deferred_mut[0] += 1; // the actual mutable borrow starts and ends here
//! // the next line does not overlap with the previous, thanks to non-lexical lifetimes:
//! deferred_mut2[0] += 1; 
//! assert_eq!(44, deferred[0]);
//! assert_eq!(deferred_mut[0], deferred_mut2[0]);
//! // however, the following is not okay, because this would create two overlapping
//! // mutable references and this is undefined behavior (hence the use of `unsafe` above):
//! //assert_eq!(&mut deferred_mut[0], &mut deferred_mut2[0]); // undefined behavior!
//! // because `Deferred` implements the `Index` and `IndexMut` trait, it is possible
//! // to create two references that overlap in lifetime, but are disjoint in index:
//! assert_eq!(&mut deferred_mut[1], &mut deferred_mut2[2]); // indices are disjoint, so no UB
//! // this is not possible with regular slice references, because these alias the entire slice:
//! //assert_eq!(&mut deferred_mut.deref_mut()[1], &mut deferred_mut2.deref_mut()[2]); // UB!
//! ```
//!
//! # Why not use `<[T]>::split_at_mut` instead of `Deferred`?
//! The Rust core library function [split_at_mut](<slice::split_at_mut>) provides a convenient method to safely split a mutable reference
//! into two mutable references. However, it has one big drawback: in order to call it, you must already have the mutable reference.
//! This might not always be possible. For example, in a multi-threaded environment, some threads may want to temporarily keep an
//! immutable reference to a slice index because these threads only read, while some threads need a temporary mutable reference into
//! another slice index. This is not possible if one thread holds a mutable reference to the entire slice, because then the co-existance
//! of any mutable and immutable references would trigger undefined behavior. With deferred references, this is no longer a problem, because an
//! initial mutable reference is not required. In fact, the first mutable reference is not even created until the first time the
//! `Deferred<&mut>` is dereferenced. This crate also provides a method [Deferred::split_at_mut] which can split a deferred reference
//! to a slice or an array into two deferred references. This method does not create any intermediary references to the subslices.
//!
//! # `#![no_std]` environments
//! This crate is entirely `#![no_std]` and does not depend on the `alloc` crate. No additional `Cargo.toml` features need to be configured
//! in order to support `#![no_std]` environments. This crate also does not have any dependencies in its `Cargo.toml`.
//!
//! # Miri tested
//! This crate is extensively tested using [Miri](https://github.com/rust-lang/miri) using the `-Zmiri-track-raw-pointers` flag:
//! ```bash
//! $ MIRIFLAGS="-Zmiri-track-raw-pointers" cargo miri test
//! ```
//! Miri follows the [Stacked Borrows](https://github.com/rust-lang/unsafe-code-guidelines/blob/master/wip/stacked-borrows.md) model
//! (by Ralf Jung et al.) and so does this crate. If you happen to spot any violations of this model in this crate, feel free
//! to open a Github issue!
//!
//! # Footnotes
//! [^1]: The concept of "deferred references" is inspired by the concept of "[Deferred Borrows](https://c1f.net/pubs/ecoop2020_defborrow.pdf)"
//!       authored by Chris Fallin. However, these are not entirely the same concept. These differ in the sense that deferred borrows bring
//!       an extension to the Rust type system (called *static path-dependent types*) and its implementation is intended to live within the
//!       Rust compiler, while deferred references are implemented in Rust code that is already possible today with its existing type system.
//!       The trade-off made here is that this requires minimal use of `unsafe` code blocks with deferred references, while deferred borrows
//!       would work entirely within "safe Rust" if these were to be implemented in the Rust compiler. There are also some similarities between
//!       the two concepts: both concepts are statically applied during compile-time and due not incur any runtime overhead. Also, with both
//!       approaches an actual reference is not created until the reference is actually in use (i.e. dereferenced or borrowed for an extended
//!       period of time).

#![no_std]
#![doc(html_root_url = "https://docs.rs/deferred-reference/0.1.1")]
// the `slice_ptr_len` feature is needed for deferred references to slices
#![cfg_attr(feature = "slice_ptr_len", feature(slice_ptr_len))]
// experimental: the `coerce_unsized` feature is need to unsize the reference
// through [Deferred::unsize], because we can't implement [CoerceUnsized] on
// [Deferred] (yet, this gives compiler errors).
#![cfg_attr(feature = "coerce_unsized", feature(coerce_unsized))]

#![deny(missing_docs)]
#![forbid(clippy::missing_docs_in_private_items)]

// the `alloc` crate is only used for tests, but it is not used by this crate otherwise.
#[cfg(test)] #[macro_use] extern crate alloc;


// from <https://rust-lang.github.io/unsafe-code-guidelines/glossary.html>:
// "Aliasing occurs when one pointer or reference points to a "span" of memory that overlaps
// with the span of another pointer or reference. A span of memory is similar to how a slice
// works: there's a base byte address as well as a length in bytes.
// Note: a full aliasing model for Rust, defining when aliasing is allowed and when not, has
// not yet been defined. The purpose of this definition is to define when aliasing happens,
// not when it is allowed. The most developed potential aliasing model so far is Stacked Borrows."

mod core_traits_impl;
pub use core_traits_impl::*;

mod defer;
pub use defer::*;

mod defer_mut;
pub use defer_mut::*;

mod deferred;
pub use deferred::*;

mod pointer_length;
pub use pointer_length::*;

mod reference;
pub use reference::*;

mod slice_like;
pub use slice_like::*;

mod slice_like_impl;
pub use slice_like_impl::*;

mod slice_pointer_index;
pub use slice_pointer_index::*;


#[cfg(test)]
mod tests {
    use alloc::boxed::Box;
    use core::{cell::UnsafeCell, fmt::{Debug, Pointer}, ops::{Deref, DerefMut}};

    use super::*;

    #[test]
    fn example() {
        use crate::{Deferred, DeferMut};
        use core::cell::UnsafeCell;
        const N: usize = 1024;
        let buffer = UnsafeCell::new([0usize; N]);
        // SAFETY: this is safe because there are no references into `buffer` yet
        let mut deferred1: Deferred<&mut [usize; N]> = unsafe { buffer.defer_mut() };
        // SAFETY: this is safe because we promise not to create overlap with mutable references
        let mut deferred2: Deferred<&mut [usize; N]> = unsafe { deferred1.clone_unchecked() };
        assert_eq!(&mut deferred1[0..10], &mut deferred2[10..20]); // subslices do not overlap
    }

    /// A method to assert that `t` is still being used.
    fn use_it<T: Pointer>(t: T) {
        assert!(!format!("{:p}", t).is_empty());
    }

    #[test]
    fn doctest1() {
        let buffer = Box::new(UnsafeCell::new([0u8; 1024])); // the `Box` is optional
        // calling defer() or defer_mut() immutably borrows `buffer` for as long
        // as the returned `Deferred` is in use.
        let deferred: Deferred<&[u8; 1024]> = buffer.defer();
        // SAFETY: this is safe, because we promise not to create overlapping references
        let mut deferred_mut: Deferred<&mut [u8; 1024]> = unsafe { buffer.defer_mut() };
        // both `deferred` and `deferred_mut` can be safely immutably dereferenced simultaneously:
        assert_eq!(&deferred[0], &deferred_mut[0]);
        // we can mutate the `buffer` through `deferred_mut` as any other array.
        // this implicity creates a temporary mutable reference into the array inside `buffer`.
        // even though an immutable reference to `buffer` exists, this is okay because
        // the inner array sits inside an `UnsafeCell` which allows interior mutability:
        deferred_mut[0] = 42; 
        // and observe the change through `deferred`:
        assert_eq!(deferred[0], 42);
        // all this time, both deferred references are alive, but because
        // these are not actual references, this doesn't violate the Rust borrow
        // rules and this is not undefined behavior. The lifetimes of the mutable
        // and immutable references derived from the Deferred do not overlap,
        // so the Rust borrow rules are respected all this time.
        assert_eq!(&deferred[0], &deferred_mut[0]);
        // this also works for multiple deferred mutable references!
        // SAFETY: this is safe, because we promise not to create overlapping references
        let mut deferred_mut2 = unsafe { buffer.defer_mut() };
        // we can mutate the buffer through 2 distinct deferred mutable references
        // (as long as we don't do this at the same time!)
        deferred_mut[0] += 1; // the actual mutable borrow starts and ends here
        // the next line does not overlap with the previous, thanks to non-lexical lifetimes:
        deferred_mut2[0] += 1; 
        assert_eq!(44, deferred[0]);
        assert_eq!(deferred_mut[0], deferred_mut2[0]);
        // however, the following is not okay, because this would create two overlapping
        // mutable references and this is undefined behavior (hence the use of `unsafe` above):
        //assert_eq!(&mut deferred_mut[0], &mut deferred_mut2[0]); // undefined behavior!
        // because `Deferred` implements the `Index` and `IndexMut` trait, it is possible
        // to create two references that overlap in lifetime, but are disjoint in index:
        assert_eq!(&mut deferred_mut[1], &mut deferred_mut2[2]); // indices are disjoint, so no UB
        // this is not possible with regular slice references, because these alias the entire slice:
        //assert_eq!(&mut deferred_mut.deref_mut()[1], &mut deferred_mut2.deref_mut()[2]); // UB!
    }

    #[test]
    fn dyn_trait_ref() {
        // multi trait dyn refs are not allowed yet, except for auto traits:
        // error[E0225]: only auto traits can be used as additional traits in a trait object
        // this results in a single vtable in the pointer, hence this is a sized fat pointer
        let x: Box<UnsafeCell<dyn core::fmt::Debug + Send + Sync + Unpin>> = Box::new(UnsafeCell::new(core::fmt::Error));
        let _ = x.defer();
    }

    #[cfg(feature = "coerce_unsized")]
    #[test]
    fn test_coerce_unsized() {
        let b = Box::new(UnsafeCell::new([0u8; 1024]));
        let deferred1: Deferred<&[u8; 1024]> = b.defer();
        let deferred2: Deferred<&[u8]> = deferred1.unsize();
        assert_eq!(deferred1.len(), deferred2.len());
        assert_eq!(deferred1.deref(), deferred2.deref());
        assert_eq!(deferred1.len(), deferred2.len());
    }

    #[test]
    fn test_copy() {
        let b = Box::new(UnsafeCell::new([0u8; 1024]));
        let deferred2;
        {
            let deferred1 = b.defer();
            deferred2 = deferred1; // copies
        }
        //let x = b.get_mut(); // cannot borrow `*b` as mutable because it is also borrowed as immutable
        use_it(deferred2);
    }

    #[test]
    fn test_box() {
        let b = Box::new(UnsafeCell::new([0u8; 1024]));
        let _x = unsafe { b.defer_mut() };
    }

    #[test]
    fn deref_mut_unsafe_cell() {
        let /*mut*/ buffer = UnsafeCell::new([0u8; 1024]);
        let mut deferred = unsafe { buffer.defer_mut() };
        let mut deferred2 = unsafe { buffer.defer_mut() };
        // let tmp = buffer.get_mut(); // not possible
        // let tmp = &mut buffer; // not possible :)
        let _t = deferred.deref_mut();
        let _t = deferred.deref();
        let _t = deferred.deref_mut();
        let _t = deferred.deref();
        let _t = deferred.deref_mut();
        let _t = deferred.deref();
        use_it(&deferred);
        assert_eq!(deferred[0], deferred2[1]); // okay, immutable access
        assert_eq!(&deferred[0], &mut deferred2[1]); // okay, indices are disjoint
        assert_eq!(&mut deferred[0], &mut deferred2[1]); // okay, indices are disjoint
        // assert_eq!(&mut deferred[0], &mut deferred2[0]); // not okay, references overlap
    }

    mod mut_wrapper {
        use super::*;
        use super::super::Defer;

        #[derive(Default, Debug)]
        pub struct MaliciousContainer {
            pub container: UnsafeCell<[u8; 32]>,
        }

        impl MaliciousContainer {
            fn get_mut(&mut self) -> Deferred<&mut [u8; 32]> {
                Deferred::from(self.container.get_mut())
            }
        }
        impl Defer for MaliciousContainer {
            type Target = [u8; 32];

            fn defer(&self) -> Deferred<&Self::Target> {
                self.container.defer()
            }
        }
        unsafe impl DeferMut for MaliciousContainer {
            unsafe fn defer_mut(&self) -> Deferred<&mut Self::Target> {
                self.container.defer_mut()
            }
        }

        #[test]
        fn try_to_trigger_ub() {
            let mut malicious_container = MaliciousContainer::default();
            // this mutable alias works all the way through the [u8; 32] inside the UnsafeCell
            // UnsafeCell only protects immutable references, not mutable references!
            // we ensure it exists until the end of the scope.
            let mut_ref = &mut malicious_container;

            let x = mut_ref.get_mut();
            let y = x.deref();
            assert_eq!(&0, &y[0]);
            use_it(&x);
            use_it(&y);

            let mut x = unsafe { mut_ref.defer_mut() };
            let y = x.deref_mut();
            assert_eq!(&0, &mut y[0]);
            use_it(&y);
            use_it(&x);

            let x = mut_ref.defer();
            let xcopy = x;
            let y = x.deref();
            assert_eq!(&0, &y[0]);
            use_it(&x);
            use_it(&y);
            
            use_it(&mut_ref);
            let xcopy = xcopy.deref();
            use_it(&xcopy);
            use_it(&mut_ref);
        }
    }
}