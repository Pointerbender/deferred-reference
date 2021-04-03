use core::cell::UnsafeCell;

use crate::Reference;

use super::Deferred;

/// An unsafe macro to create a deferred immutable reference (i.e. a [`Deferred<&T>`](Deferred)) to a place,
/// without creating an intermediate reference. See the documentation at [Deferred] explaining the
/// [constructors for deferred immutable references](Deferred#constructors-for-deferred-immutable-references)
/// for safe alternatives.
///
/// Use of this macro is not recommended, because it's a lot more unsafe than obtaining a `Deferred<&T>` through
/// a type that implements the [Defer] trait. Use [`Defer::defer`](Defer::defer) or [`Deferred::from(&T)`](From::from)
/// for a safe alternative instead.
///
/// The only use case for this macro is when you can't create a reference to a place and also can't use [UnsafeCell]
/// to achieve interior mutability. However, this is a rare case and requires advanced knowledge of unsafe Rust.
///
/// # Example
/// ```
/// #[macro_use]
/// extern crate deferred_reference;
/// use deferred_reference::Deferred;
/// fn main() {
///     let buffer = [0u8; 1024];
///     // works for deferred references to slices:
///     let deferred: Deferred<&[u8]> =  unsafe { defer!(buffer) };
///     assert_eq!(buffer[0], deferred[0]);
///     // and works also for deferred references to arrays:
///     let deferred: Deferred<&[u8; 1024]> =  unsafe { defer!(buffer) };
///     assert_eq!(buffer[0], deferred[0]);
/// }
/// ```
///
/// # Safety
/// This macro is very unsafe and should only be used if there is no other safe way to obtain a deferred reference.
/// See the [Defer] trait for the preferred way to create an immutable deferred reference. When using this
/// macro, the caller must uphold the following guarantees:
/// * When dereferencing the [Deferred], the Rust alias rules must be upheld at all times. E.g. don't create mutable and
///   immutable references to the same place (these may not partially overlap either).
/// * The place must be properly aligned and initialized.
/// * The caller must ensure that the invariant of the returned [Deferred] is upheld.
/// * The place must not be moved or dropped for as long as the returned [Deferred] is in use.
/// * No mutable references to the place may be created as long as the [Deferred] is in use. This will invalidate the [Deferred].
///
/// Here is an example that will cause undefined behavior to illustrate how unsafe this macro is.
/// The compiler will happilly compile this and not give any warning:
/// ```no_run
/// #[macro_use]
/// extern crate deferred_reference;
/// fn main() {
///     let mut buffer = [0u8; 1024];
///     let deferred = unsafe { defer!(buffer) };
///     buffer[0] = 42; // this implicitly creates a temporary mutable reference to `buffer`
///     // `deferred` is now invalidated 
///     // this is undefined behavior, even though the lifetimes of
///     // the immutable and mutable reference don't overlap:
///     assert_eq!(buffer[0], deferred[0]);
/// }
/// ```
/// This kind of pitfalls are not possible with instances of `Deferred<&T>` obtained through [Defer::defer],
/// because then the Rust compiler can detect these violations due to the bounded lifetime of
/// `Deferred<&T>`. In this case the compiler will warn you before-hand:
/// ```compile_fail
/// use deferred_reference::Defer;
/// use core::cell::UnsafeCell;
/// let mut buffer = UnsafeCell::new([0u8; 1024]);
/// let deferred = buffer.defer(); // note the absence of the unsafe block!
/// // the next line will cause the compilation to fail with the error:
/// // "cannot borrow `buffer` as mutable because it is also borrowed as immutable"
/// buffer.get_mut()[0] = 42;
/// assert_eq!(0, deferred[0]); // assures `deferred` is in use until here
/// ```
///
/// # Caveat
/// The lifetime for the returned [Deferred] is inferred from its usage. To prevent accidental misuse,
/// it's suggested to tie the lifetime to whichever source lifetime is safe in the context, such as
/// by providing a helper function taking the lifetime of a host value, or by explicit annotation.
/// However, this can get very complicated and very unsafe real fast, here is an example of how this
/// could end up looking like (accompanied by the obligatory "*do not try this at home, kids!*"):
/// ```compile_fail
/// #[macro_use]
/// extern crate deferred_reference;
/// use deferred_reference::Deferred;
/// use core::marker::PhantomData;
/// fn shorten_lifetime<'a, 'b: 'a>(a: &'a PhantomData<[u8]>, b: Deferred<&'b [u8]>)
///     -> (&'a PhantomData<[u8]>, Deferred<&'a [u8]>)
/// {
///     // SAFETY: shortening the lifetime of 'b to 'a is always safe
///     unsafe { core::mem::transmute((a, b)) }
/// }
/// fn borrow_mut<'a>(_accountant: &'a mut PhantomData<[u8]>, borrow_mut: &'a mut [u8])
///     -> &'a mut [u8]
/// {
///     borrow_mut
/// }
/// macro_rules! get_deferred {
///     ($accountant:ident, $place:expr) => {
///         shorten_lifetime(&$accountant, defer!($place)).1
///     };
/// }
/// macro_rules! borrow_mut {
///     ($accountant:ident, $place:expr) => {
///         borrow_mut(&mut $accountant, &mut $place)
///     };
/// }
/// fn main() {
///     let mut buffer = [0u8; 1024];
///     // the `accountant` acts as a compile-time semaphore through the Rust borrow rules.
///     let mut accountant = PhantomData::<[u8]>;
///     // SAFETY: we promise to only use the `get_deferred` and `borrow_mut` macros.
///     // SAFETY: this is safe, because we don't take out explicit borrows like
///     // SAFETY: `&mut buffer` which are not tracked by our `accountant`.
///     let deferred: Deferred<&[u8]> = unsafe { get_deferred!(accountant, buffer) };
///     assert_eq!(0, deferred[0]);
///     // the next line will give an error at compile-time:
///     // "cannot borrow `accountant` as mutable because it is also borrowed as immutable"
///     let ref_mut: &mut [u8] = borrow_mut!(accountant, buffer); // mutably borrows
///     assert_eq!(0, deferred[0]); // asserts that `deferred` is in use until here
/// }
/// ```
#[macro_export]
macro_rules! defer {
    ($place:expr) => {
        $crate::Deferred::from_raw(core::ptr::addr_of!($place))
    };
}

/// The [Defer] trait offers easy access to deferred immutable references
/// to types that implement [Defer].
///
/// # Examples
/// ```
/// use deferred_reference::{Defer, Deferred};
/// use core::cell::UnsafeCell;
///
/// // Defer is implemented by default for any `UnsafeCell` holding an array or a slice:
/// let buffer = UnsafeCell::new([0u8; 1024]);
/// let deferred: Deferred<&[u8; 1024]> = buffer.defer();
/// // A deferred reference to an *array* can be unsized to a deferred reference to a *slice*:
/// let deferred_unsized: Deferred<&[u8]> = deferred.into();
/// // Dereferencing will create an actual reference `&[u8]`:
/// assert_eq!(*deferred, *deferred_unsized);
/// assert_eq!(*deferred, &[0u8; 1024][..]);
///
/// // Even though this crate is `#![no_std]`, it works with heap allocated contents as well:
/// let buffer = Box::new(UnsafeCell::new([0u8; 1024]));
/// let deferred /* : Deferred<&[u8; 1024]> */ = buffer.defer(); // omitting the type is okay
/// let deferred_unsized: Deferred<&[u8]> = deferred.into(); // unsize needs explicit type
/// // Dereferencing will create an actual reference `&[u8]`:
/// assert_eq!(*deferred, *deferred_unsized);
/// assert_eq!(*deferred, &[0u8; 1024][..]);
/// ```
///
/// # Why is `Defer` not implemented for all types?
/// The [Defer] trait comes implemented for all [`UnsafeCell<T: ?Sized>`](core::cell::UnsafeCell),
/// but it is not implemented for all types `T: ?Sized` which are not wrapped in an [UnsafeCell].
/// This is by design, because calling the `Defer::defer(&self)` method will take out an immutable reference
/// for as long as the returned [Deferred] is in use. In other words, this is not a deferrred
/// immutable reference, but an actual immutable reference to the underlying type. As long as this
/// immutable reference is in use, it is not allowed to create additional (deferred) mutable references
/// due to the Rust borrowing rules, unless the type is wrapped in an [UnsafeCell], which allows for
/// interior mutability (in fact this is the only way that Rust supports interior mutability).
/// If you don't intend to mutate the underlying type, then there is no use-case for [Defer]
/// and you're better off using an immutable reference `&T` instead of a `Deferred<&T>`.
/// If you do intend to mutate the underlying type, but have a good reason not to use [UnsafeCell],
/// then you may do so using the unsafe [`defer!`](macro@defer) and [`defer_mut!`](macro@defer_mut) macros.
/// However, use of these macros is not recommend and you should probably only use these if you know what you're
/// doing, because the lifetime of the [Deferred] returned by these macros is unbounded and the burden of
/// managing the lifetimes falls on the implementor. Note that there are also
/// [other (safe) ways to construct a deferred immutable reference](Deferred#constructors-for-deferred-immutable-references),
/// see the documentation at [Deferred] for more information on how to do this.
pub trait Defer {
    /// The type that the deferred reference points to.
    type Target: ?Sized;
    /// Obtain a deferred immutable reference to a [Defer::Target].
    fn defer(&self) -> Deferred<&Self::Target>;
}

impl<T: ?Sized> Defer for UnsafeCell<T> {
    type Target = T;
    fn defer(&self) -> Deferred<&T> {
        unsafe {
            Deferred::from_raw(self.get() as *const _)
        }
    }
}

impl<T: Reference> Defer for Deferred<T> {
    type Target = T::Target;

    fn defer(&self) -> Deferred<&Self::Target> {
        // SAFETY: Deferred (i.e. Self) already holds up the invariant, so this is safe.
        // SAFETY: this yields an almost identical Deferred, except it has a shorter lifetime
        // SAFETY: (it takes the lifetime of `&self` instead of from `T`)
        unsafe {
            Deferred::from_raw(self.as_ptr())
        }
    }
}

#[cfg(test)]
mod tests {
    mod defer_macro {
        use core::marker::PhantomData;

        use crate::Deferred;

        #[test]
        fn call_macro() {
            let buffer = [1u8; 1024];
            let deferred = unsafe { defer!(buffer) };
            assert_eq!(buffer[0], deferred[0]);
        }
        // /// this triggers UB in miri, because deferred gets invalidated
        // #[test]
        // fn example_ub() {
        //     let mut buffer = [2u8; 1024];
        //     let deferred = unsafe { defer!(buffer) };
        //     // this is undefined behavior:
        //     buffer[0] = 42; // because this implicitly creates a temporary mutable reference
        //     assert_eq!(buffer[0], deferred[0]);
        // }
        
        // /// this is also not possible, Rust detects the mutable borrow:
        // /// cannot borrow `buffer` as immutable because it is also borrowed as mutable
        // #[test]
        // fn assert_no_reference_created() {
        //     let mut buffer = [2u8; 1024];
        //     let mut_ref = &mut buffer;
        //     // next line errors: cannot borrow `buffer` as immutable because it is also borrowed as mutable
        //     let deferred = unsafe { defer!(buffer) };
        //     assert_eq!(2, mut_ref[0]);
        //     assert_eq!(buffer[0], deferred[0]);
        // }

        fn shorten_lifetime<'a, 'b: 'a>(a: &'a PhantomData<[u8]>, b: Deferred<&'b [u8]>)
            -> (&'a PhantomData<[u8]>, Deferred<&'a [u8]>) {
            // SAFETY: shortening the lifetime of 'b to 'a is always safe
            unsafe { core::mem::transmute((a, b)) }
        }

        fn borrow_mut<'a>(_accountant: &'a mut PhantomData<[u8]>, borrow_mut: &'a mut [u8]) -> &'a mut [u8] {
            borrow_mut
        }

        macro_rules! get_deferred {
            ($accountant:ident, $place:expr) => {
                shorten_lifetime(&$accountant, defer!($place)).1
            };
        }

        macro_rules! borrow_mut {
            ($accountant:ident, $place:expr) => {
                borrow_mut(&mut $accountant, &mut $place)
            };
        }
        #[test]
        fn bounded_lifetime() {
            let mut buffer = [0u8; 1024];
            let mut accountant = PhantomData::<[u8]>;
            let deferred: Deferred<&[u8]> = unsafe { get_deferred!(accountant, buffer) };
            assert_eq!(0, deferred[0]);
            let ref_mut: &mut [u8] = borrow_mut!(accountant, buffer);
            // assert_eq!(0, deferred[0]);
            assert_eq!(0, ref_mut[0]);
        }
    }

    mod defer_trait {
        use alloc::boxed::Box;
        use core::cell::UnsafeCell;
        use crate::{Defer, Deferred};

        #[test]
        fn doctest1() {
            // Defer is implemented by default for any `UnsafeCell` holding an array or a slice:
            let buffer = UnsafeCell::new([0u8; 1024]);
            let deferred: Deferred<&[u8; 1024]> = buffer.defer();
            // A deferred reference to an *array* can be unsized to a deferred reference to a *slice*:
            let deferred_unsized: Deferred<&[u8]> = deferred.into();
            // Dereferencing will create an actual reference `&[u8]`:
            assert_eq!(*deferred, *deferred_unsized);
            assert_eq!(*deferred, &[0u8; 1024][..]);
            
            // Even though this crate is `#![no_std]`, it works with heap allocated contents as well:
            let buffer = Box::new(UnsafeCell::new([0u8; 1024]));
            let deferred /* : Deferred<&[u8; 1024]> */ = buffer.defer(); // omitting the type is okay
            let deferred_unsized: Deferred<&[u8]> = deferred.into(); // unsize needs explicit type
            // Dereferencing will create an actual reference `&[u8]`:
            assert_eq!(*deferred, *deferred_unsized);
            assert_eq!(*deferred, &[0u8; 1024][..]);
        }
    }
}