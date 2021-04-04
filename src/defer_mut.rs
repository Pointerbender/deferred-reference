use core::cell::UnsafeCell;

use super::{Defer, Deferred};

/// An unsafe macro to create a deferred mutable reference (i.e. a [`Deferred<&mut T>`](Deferred)) to a place,
/// without creating an intermediate reference. See the documentation at [Deferred] explaining the
/// [constructors for deferred mutable references](Deferred#constructors-for-deferred-mutable-references)
/// for safer alternatives.
///
/// Use of this macro is not recommended, because it's a lot more unsafe than obtaining a `Deferred<&mut T>` through
/// a type that implements the [DeferMut] trait. Use [`DeferMut::defer_mut`](DeferMut::defer_mut) or
/// [`Deferred::from(&mut T)`](From::from) for a safer alternative instead.
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
///     let mut buffer = [0u8; 1024];
///     let deferred: Deferred<&mut [u8]> =  unsafe { defer_mut!(buffer) };
///     assert_eq!(buffer[0], deferred[0]);
///     // works also for references to arrays:
///     let deferred: Deferred<&mut [u8; 1024]> =  unsafe { defer_mut!(buffer) };
///     assert_eq!(buffer[0], deferred[0]);
/// }
/// ```
///
/// # Safety
/// This macro is very unsafe and should only be used if there is no other safe way to obtain a deferred mutable reference.
/// See the [DeferMut] trait for the preferred way to create an deferred mutable reference. When using this
/// macro, the caller must uphold the following guarantees:
/// * When dereferencing the [Deferred], the Rust alias rules must be upheld at all times. E.g. don't create mutable and
///   immutable references to the same place (these may not partially overlap either).
/// * The place must be properly aligned and initialized.
/// * The caller must ensure that the invariant of the returned [Deferred] is upheld.
/// * The place must not be moved or dropped for as long as the returned [Deferred] is in use.
/// * No explicit references to the place may be created as long as the [Deferred] is in use. This will invalidate the [Deferred].
/// * Any other instances of [Deferred] that point to the same location, must be reborrowed from the original deferred mutable reference.
///   This is possible using [Deferred::clone_unchecked] and [Deferred::into_ref]. Any other deferred references will becomed invalidated
///   as soon as the deferred mutable reference is dereferenced (unless its target contents are inside an [UnsafeCell]).
///
/// Here is an example that will trigger undefined behavior, in order to illustrate how unsafe this macro is:
/// ```no_run
/// #[macro_use]
/// extern crate deferred_reference;
/// fn main() {
///     let mut buffer = [0u8; 1024];
///     let deferred = unsafe { defer_mut!(buffer) };
///     buffer[0] = 42; // implicitly creates a temporary mutable reference to all of `buffer`
///     // `deferred` is now invalidated !!!
///     // therefore dereferencing `deferred` is undefined behavior, even though
///     // the lifetimes of the immutable and mutable references don't overlap:
///     assert_eq!(buffer[0], deferred[0]); // undefined behavior!!!
/// }
/// ```
///
/// # Caveat
/// The lifetime for the returned [Deferred] is inferred from its usage. To prevent accidental misuse,
/// it's suggested to tie the lifetime to whichever source lifetime is safe in the context, such as
/// by providing a helper function taking the lifetime of a host value, or by explicit annotation.
/// However, this can get very complicated and very unsafe real fast, see the [`defer`](macro@defer#caveat)
/// macro for an example of how to do this without creating an intermediate reference.
///
/// # How can this be safely used together with the `defer!` macro?
/// As mentioned above under section "[*Safety*](#safety)", dereferencing a `Deferred<&mut T>` will invalidate any other [Deferred] instances 
/// which are not re-borrowed, even the ones created by the [`defer`](macro@defer) macro (`Deferred<&T>` instances returned by the
/// [`defer`](macro@defer) macro do not constitute as re-borrows). This means that the [`defer`](macro@defer) macro is only safe to use
/// together with the [`defer_mut`](macro@defer_mut) macro if you take special care to always call [`defer!`](macro@defer) again to refresh
/// its pointer, after a mutable reference has been given out through dereferencing the `Deferred<&mut T>.` For example, this is definately
/// __considered undefined behavior__:
/// ```no_run
/// #[macro_use]
/// extern crate deferred_reference;
/// use deferred_reference::Deferred;
/// fn main() {
///     let mut buffer = [0u8; 1024];
///     // SAFETY: what we are about to do is very unsafe!
///     let mut deferred_mut: Deferred<&mut [u8]> = unsafe { defer_mut!(buffer) };
///     let deferred: Deferred<&[u8]> =  unsafe { defer!(buffer) };
///     assert_eq!(0, deferred_mut[0]);
///     assert_eq!(0, deferred[0]); // so far so good, no UB yet...
///     deferred_mut[0] = 42; // this implicity creates a mutable reference to `buffer`
///     // `deferred` is now invalidated!
///     assert_eq!(42, deferred_mut[0]); // this is not yet UB...
///     assert_eq!(42, deferred[0]); // this is UB!
/// }
/// ```
/// The undefined behavior can be side-stepped if the deferred mutable reference is re-borrowed, like so:
/// ```
/// #[macro_use]
/// extern crate deferred_reference;
/// use deferred_reference::Deferred;
/// fn main() {
///     let mut buffer = [0u8; 1024];
///     // SAFETY: this is safe because we reborrow `deferred_mut` and
///     // SAFETY: we don't create any overlapping references.
///     let mut deferred_mut: Deferred<&mut [u8]> = unsafe { defer_mut!(buffer) };
///     let deferred: Deferred<&[u8]> = unsafe { deferred_mut.clone_unchecked().into_ref() };
///     assert_eq!(0, deferred_mut[0]);
///     assert_eq!(0, deferred[0]);
///     deferred_mut[0] = 42; // this implicity creates a mutable reference to `buffer`
///     assert_eq!(42, deferred_mut[0]);
///     assert_eq!(42, deferred[0]); // but this is not UB thanks the re-borrow
/// }
/// ```
/// If the calls to the [`defer`](macro@defer) macro are timed well, then it is possible to combine
/// the two macros without running into undefined behavior:
/// ```
/// #[macro_use]
/// extern crate deferred_reference;
/// use deferred_reference::Deferred;
/// fn main() {
///     let mut buffer = [0u8; 1024];
///     // SAFETY: this is safe, because we create new deferred references after
///     // SAFETY: dereferencing `deferred_mut` into an actual mutable reference.
///     let mut deferred_mut: Deferred<&mut [u8]> = unsafe { defer_mut!(buffer) };
///     let mut deferred: Deferred<&[u8]> = unsafe { defer!(buffer) };
///     assert_eq!(0, deferred_mut[0]);
///     assert_eq!(0, deferred[0]);
///     deferred_mut[0] = 42; // this implicity creates a temporary mutable reference to `buffer`
///     // `deferred` is now invalidated! we refresh it again:
///     deferred = unsafe { defer!(buffer) };
///     assert_eq!(42, deferred_mut[0]);
///     // this is not UB, because the mutable reference did not overlap
///     // with the re-creation of `deferred`:
///     assert_eq!(42, deferred[0]); 
/// }
/// ```
/// The previous 3 examples consist of very unsafe Rust and should not be attempted unless there
/// exists a specific reason why this is needed and the implementor has intricate knowledge
/// about raw pointer management in Rust. In all other cases, using the [DeferMut::defer_mut]
/// is the much better alternative to the [`defer_mut`](macro@defer_mut) macro.
#[macro_export]
macro_rules! defer_mut {
    ($place:expr) => {
        $crate::Deferred::from_raw_mut(core::ptr::addr_of_mut!($place))
    };
}

/// The [DeferMut] trait offers easy access to deferred mutable references
/// to types that implement [DeferMut]. This trait is already implemented on all types `T: ?Sized` for
/// [`UnsafeCell<T>`](core::cell::UnsafeCell) out-of-the-box and this should be sufficient for most purposes,
/// but it is also possible to implement the [Defer] and [DeferMut] traits for your own types, please see
/// the documentation of this trait on how to do this safely.
/// 
/// # Safety
/// This trait may only be implemented for:
/// 1. All types that support interior mutability. Concretely, this means that
/// for types that have interior mutability, the type must contain an [UnsafeCell] or one of
/// of its derivatives, such as those in the Rust standard library like `RefCell`, `RwLock` or `Mutex`.
/// 2. Other smart-pointers which do not "own" their data. Smart-pointers which contain a
/// mutable pointer `*mut T` don't need to wrap the mutable pointer in an [UnsafeCell], because
/// dereferencing a mutable pointer does not constitute interior mutability.
///
/// Additionally, all types that implement [DeferMut] must uphold the following invariants:
/// * The type must also implement the [Defer] trait.
/// * The deferred reference that the [DeferMut::defer_mut] method returns, must point to the same location
///   as the [Deferred] returned by [Defer::defer].
/// * Both the [Defer] and [DeferMut] trait implementations may not create any references
///   to the location where the returned [Deferred] points, nor may the location
///   be accessed in any way (e.g. dereferencing). Taking an immutable shared reference to a wrapping [UnsafeCell]
///   is okay, but creating a reference to the contents of the [UnsafeCell] is not! Creating a
///   mutable reference to the wrapping [UnsafeCell] is also not okay, because [UnsafeCell] only protects
///   shared references to a place that may be mutated, it does not weaken the rules for mutable references,
///   which say that a mutable reference must be exclusive in order to stay clear of undefined behavior.
///
/// # Example
/// Here is an example for how to implement this trait for custom smart pointers.
/// ```
/// use deferred_reference::{Defer, DeferMut, Deferred};
/// /// `MemoryMappedBuffer` is a simplified representation of a memory mapped slice of bytes.
/// /// Proper implementations would also contain a `MemoryMappedBuffer::new` constructor
/// /// which sets up the owned memory map and `MemoryMappedBuffer` should also implement
/// /// the `Drop` trait to properly clean up the memory map when `MemoryMappedBuffer`
/// /// goes out of scope.
/// pub struct MemoryMappedBuffer {
///     ptr: *mut u8,
///     length: usize,
/// }
/// impl Defer for MemoryMappedBuffer {
///     type Target = [u8];
///     fn defer(&self) -> Deferred<&[u8]> {
///         let slice_ptr = core::ptr::slice_from_raw_parts(self.ptr as *const u8, self.length);
///         // SAFETY: this is safe because the Deferred occupies a shared reference to the
///         // SAFETY: smart pointer `MemoryMappedBuffer` for the duration of lifetime of &self,
///         // SAFETY: which means no other callers can safely obtain a mutable reference
///         // SAFETY: the MemoryMappedBuffer instance.
///         unsafe { Deferred::from_raw(slice_ptr) }
///     }
/// }
/// // SAFETY: this is safe, because the invariant of `Deferred` is upheld.
/// // SAFETY: this is only safe if the memory mapped region is properly aligned and initialized
/// // SAFETY: and `ptr` is non-null and not dangling (i.e. it must point to a valid memory region).
/// unsafe impl DeferMut for MemoryMappedBuffer {
///     unsafe fn defer_mut(&self) -> Deferred<&mut [u8]> {
///         let slice_mut_ptr = core::ptr::slice_from_raw_parts_mut(self.ptr, self.length);
///         Deferred::from_raw_mut(slice_mut_ptr)
///     }
/// }
/// ```
/// If you want to build your own custom smart pointer that also owns the backing memory,
/// then you can use `Vec`, `Box` and `UnsafeCell` to do so through interior mutability like this:
/// ```
/// use deferred_reference::{Defer, DeferMut, Deferred};
/// use core::ops::{Deref, DerefMut};
/// use core::cell::UnsafeCell;
/// pub struct MyBuffer {
///     memory: Box<UnsafeCell<[u8]>>,
/// }
/// impl MyBuffer {
///     fn new(capacity: usize) -> Self {
///         let mut vector = Vec::with_capacity(capacity);
///         // we have to initialize the full vector, otherwise it is undefined behavior
///         // when we give out references to the backing slice of bytes.
///         vector.resize(capacity, 0u8);
///         let boxed_slice: Box<[u8]> = vector.into_boxed_slice();
///         // SAFETY: UnsafeCell is #[repr(transparent)] so this is safe.
///         let memory: Box<UnsafeCell<[u8]>> = unsafe { core::mem::transmute(boxed_slice) };
///         Self { memory }
///     }
/// }
/// // we only need to implement Deref, because the Defer and DeferMut
/// // traits are already implemented for UnsafeCell<[u8]>.
/// impl Deref for MyBuffer {
///     type Target = UnsafeCell<[u8]>;
///     fn deref(&self) -> &Self::Target {
///         self.memory.deref()
///     }
/// }
/// // we also implement DerefMut just to illustrate the invalidation of
/// // Deferred when taking out a mutable borrow. this is optional.
/// impl DerefMut for MyBuffer {
///     fn deref_mut(&mut self) -> &mut Self::Target {
///         self.memory.deref_mut()
///     }
/// }
/// fn main() {
///     let mut my_buffer = MyBuffer::new(1024 * 100); // 100kb buffer
///     // SAFETY: this is safe, because there exist no references to the slice.
///     let deferred_mut: Deferred<&mut [u8]> = unsafe { my_buffer.defer_mut() };
///     // the next line implicitly calls Deref::deref() on `my_buffer`.
///     // this is also okay, because the slice sits inside an UnsafeCell.
///     let deferred: Deferred<&[u8]> = my_buffer.defer();
///     // the next statement is also safe, because by taking out a `&mut self` on MyBuffer
///     // the deferred references are invalidated due to their lifetimes.
///     // this implicitly calls DerefMut::deref_mut() and then UnsafeCell::get_mut():
///     let mut_ref: &mut [u8] = my_buffer.get_mut();
///     // with the next statement uncommmented, the above statement will error:
///     // "cannot borrow `my_buffer` as mutable because it is also borrowed as immutable"
///     //let mut_ref2: &mut [u8] = deferred_mut.deref_mut(); // uncomment for error
/// }
/// ```
pub unsafe trait DeferMut: Defer {
    /// Obtain a deferred mutable reference to [Defer::Target].
    /// 
    /// # Example
    /// ```
    /// use deferred_reference::{Defer, DeferMut};
    /// use core::cell::UnsafeCell;
    /// let buffer = UnsafeCell::new([0u8; 1024]);
    /// // calling defer() or defer_mut() immutably borrows `buffer` for as long
    /// // as the returned `Deferred` is in use.
    /// let deferred = buffer.defer();
    /// // SAFETY: this is safe, because we promise not to create an overlapping mutable reference
    /// let mut deferred_mut = unsafe { buffer.defer_mut() };
    /// // both `deferred` and `deferred_mut` can be safely immutably dereferenced simultaneously:
    /// assert_eq!(&deferred[0], &deferred_mut[0]);
    /// // we can mutate the `buffer` through `deferred_mut` as any other array.
    /// // this implicity creates a temporary mutable reference into the array inside `buffer`.
    /// // even though an immutable reference to `buffer` exists, this is okay because
    /// // the inner array sits inside an `UnsafeCell` which allows interior mutability:
    /// deferred_mut[0] = 42; 
    /// // and observe the change through `deferred`:
    /// assert_eq!(deferred[0], 42);
    /// // all this time, both deferred references are alive, but because
    /// // these are not actual references, this doesn't violate the Rust borrow
    /// // rules and this is not undefined behavior. The lifetimes of the mutable
    /// // and immutable references derived from the Deferred do not overlap,
    /// // so the Rust borrow rules are respected all this time.
    /// assert_eq!(&deferred[0], &deferred_mut[0]);
    /// // this also works for multiple deferred mutable references!
    /// // SAFETY: this is safe, because we promise not to create overlapping references
    /// let mut deferred_mut2 = unsafe { buffer.defer_mut() };
    /// // we can mutate the buffer through 2 distinct deferred mutable references
    /// // (as long as we don't do this at the same time!)
    /// deferred_mut[0] += 1;
    /// deferred_mut2[0] += 1;
    /// assert_eq!(44, deferred[0]);
    /// assert_eq!(deferred_mut[0], deferred_mut2[0]);
    /// // because `Deferred` implements the `Index` and `IndexMut` trait, it is possible
    /// // to create two references that overlap in lifetime, but are disjoint in index:
    /// assert_eq!(&mut deferred_mut[1], &mut deferred_mut2[2]); // indices are disjoint, so no UB
    /// ```
    ///
    /// # Safety
    /// This method is unsafe, because it is possible to call it more than once. This is in contrast
    /// to the regular Rust borrowing rules, where it is only allowed to have one mutable borrow at
    /// a time. [Deferred] instances are not actual references and this is why this is not considered
    /// undefined behavior. However, the absence of instant undefined behavior does not make this
    /// method safe. [Deferred] also implements the [DerefMut](core::ops::DerefMut) trait, which lets
    /// anyone call `<Deferred as DerefMut>::deref_mut(&mut self)` on the [Deferred] and this creates
    /// an actual mutable reference from safe code. Hence, this method must be marked as unsafe,
    /// otherwise it could lead to unsoundness when creating multiple mutable references from safe code.
    /// The caller must take special care not to create any references (mutable or immutable) that
    /// may overlap with the actual mutable reference created from the returned [`Deferred<&mut T>`](Deferred).
    /// Note that overlap means a reference to the same region during the same lifetime. If two
    /// [Deferred] both create a reference to the same region, but with disjoint lifetimes, then
    /// this is safe.
    unsafe fn defer_mut(&self) -> Deferred<&mut Self::Target>;
}

unsafe impl<T: ?Sized> DeferMut for UnsafeCell<T> {
    unsafe fn defer_mut(&self) -> Deferred<&mut T> {
        Deferred::from_raw_mut(self.get())
    }
}

unsafe impl<T: ?Sized> DeferMut for Deferred<&mut T> {
    unsafe fn defer_mut(&self) -> Deferred<&mut Self::Target> {
        Deferred::from_raw_mut(self.as_mut_ptr())
    }
}

#[cfg(test)]
mod tests {
    use crate::{Defer, DeferMut};
    use core::cell::UnsafeCell;

    /// this test is identical to the example doctest of [DeferMut],
    /// but it is repeated here because Miri does not support doctests yet.
    #[test]
    fn doc_test() {
        let buffer = UnsafeCell::new([0u8; 1024]);
        // calling defer() or defer_mut() immutably borrows `buffer` for as long
        // as the returned `Deferred` is in use.
        let deferred = buffer.defer();
        // SAFETY: this is safe, because we promise not to create overlapping references
        let mut deferred_mut = unsafe { buffer.defer_mut() };
        // both `deferred` and `deferred_mut` can be safely immutably dereferenced:
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
        deferred_mut[0] += 1;
        deferred_mut2[0] += 1;
        assert_eq!(44, deferred[0]);
        assert_eq!(deferred_mut[0], deferred_mut2[0]);
        // however, the following is not okay, because this would create two overlapping
        // mutable references and this is undefined behavior (hence the use of `unsafe` above):
        //assert_eq!(&mut deferred_mut[0], &mut deferred_mut2[0]);
        // because `Deferred` implements the `Index` and `IndexMut` trait, it is possible
        // to create two references that overlap in lifetime, but are disjoint in index:
        assert_eq!(&mut deferred_mut[1], &mut deferred_mut2[2]); // indices are disjoint, so no UB
    }

    mod defer_mut_macro {
        use crate::{defer, Deferred};
        #[test]
        fn call_macro() {
            let mut buffer = [1u8; 1024];
            let mut deferred = unsafe { defer_mut!(buffer) };
            assert_eq!(buffer[0], deferred[0]);
            deferred[0] = 2;
            assert_eq!(2, buffer[0]);
        }
        // /// this triggers UB in miri, because deferred gets invalidated
        // #[test]
        // fn example_ub() {
        //     let mut buffer = [2u8; 1024];
        //     let deferred = unsafe { defer_mut!(buffer) };
        //     // this is undefined behavior:
        //     buffer[0] = 42; // because this implicitly creates a temporary mutable reference
        //     assert_eq!(buffer[0], deferred[0]);
        // }
        
        #[test]
        fn assert_no_reference_created() {
            let mut buffer = [3u8; 1024];
            let mut deferred = unsafe { defer_mut!(buffer) };
            assert_eq!(&3, &buffer[0]); // this implicitly creates a temporary immutable reference to all of `buffer`
            // this is undefined behavior:
            //buffer[0] = 42; // because this implicitly creates a temporary mutable reference to all of `buffer`, which invalidates `deferred`
            // but this is not undefined behavior:
            deferred[0] = 42; // because the mutable pointer to `buffer` inside `deferred` is re-borrowed
            assert_eq!(buffer[0], deferred[0]);
            assert_eq!(42, buffer[0]);
        }

        /// extra test to ensure the doctest above doesn't cause UB (miri doesn't work on doctests yet)
        #[test]
        fn doc_test() {
            let mut buffer = [0u8; 1024];
            let deferred: Deferred<&mut [u8]> =  unsafe { defer_mut!(buffer) };
            assert_eq!(buffer[0], deferred[0]);
            // works also for references to arrays:
            let deferred: Deferred<&mut [u8; 1024]> =  unsafe { defer_mut!(buffer) };
            assert_eq!(buffer[0], deferred[0]);
        }

        /// canary test to see if mutable references reborrowed from Deferred invalidate other Deferred.
        /// (so far miri doesn't complain here yet)
        #[test]
        fn overlapping_deferreds() {
            let mut buffer = [3u8; 1024];
            let mut deferred1 = unsafe { defer_mut!(buffer) };
            let mut deferred2 = unsafe { defer_mut!(buffer) };
            assert_eq!(&mut 3, &mut deferred1[0]);
            assert_eq!(&mut 3, &mut deferred2[0]);
            assert_eq!(&mut 3, &mut deferred1[0]);
            deferred1[0] = 42;
            deferred2[1] = 42;
            assert_eq!(&mut 42, &mut deferred2[0]);
            assert_eq!(&mut 42, &mut deferred1[0]);
            assert_eq!(&mut 42, &mut deferred2[0]);
            assert_eq!(&deferred1[..], &deferred2[..]);
        }

        #[test]
        fn mixing_defer_and_defer_mut() {
            let mut buffer = [0u8; 1024];
            let deferred: Deferred<&[u8]> =  unsafe { crate::defer!(buffer) };
            assert_eq!(0, deferred[0]);
            let mut deferred_mut: Deferred<&mut [u8]> =  unsafe { defer_mut!(buffer) };
            assert_eq!(0, deferred[0]); // deferred is still valid
            deferred_mut[0] = 42;
            assert_eq!(42, deferred_mut[0]);
            // the previous line created a mutable reference, so we need to re-obtain Deferred<&[u8]>
            let deferred: Deferred<&[u8]> =  unsafe { crate::defer!(buffer) }; // omitting this line is UB!
            assert_eq!(42, deferred[0]);
            assert_eq!(42, deferred_mut[0]);
        }
        #[test]
        fn mixing_defer_and_defer_mut2() {
            let mut buffer = [0u8; 1024];
            let mut deferred_mut: Deferred<&mut [u8]> =  unsafe { defer_mut!(buffer) };
            assert_eq!(0, deferred_mut[0]);
            let deferred: Deferred<&[u8]> =  unsafe { crate::defer!(buffer) };
            assert_eq!(0, deferred[0]);
            deferred_mut[0] = 42;
            assert_eq!(42, deferred_mut[0]);
            // the previous line created a mutable reference, so we need to re-obtain Deferred<&[u8]>
            let deferred: Deferred<&[u8]> =  unsafe { crate::defer!(buffer) }; // omitting this line is UB!
            assert_eq!(42, deferred[0]);
            assert_eq!(42, deferred_mut[0]);
        }
        #[test]
        fn mixing_defer_and_defer_mut3() {
            let mut buffer = [0u8; 1024];
            let mut deferred_mut: Deferred<&mut [u8]> =  unsafe { defer_mut!(buffer) };
            assert_eq!(0, deferred_mut[0]);
            // here we create a deferred immutable reference which reborrows the pointer:
            let deferred: Deferred<&[u8]> = unsafe { deferred_mut.clone_unchecked().into() };   
            assert_eq!(0, deferred[0]);
            deferred_mut[0] = 42;
            assert_eq!(42, deferred[0]); // NOT UB! thanks to the reborrow
            assert_eq!(42, deferred_mut[0]);
        }

        #[test]
        fn doctest_defer1() {
            let mut buffer = [0u8; 1024];
            // SAFETY: what we are about to do is very unsafe!
            let mut deferred_mut: Deferred<&mut [u8]> = unsafe { defer_mut!(buffer) };
            let deferred: Deferred<&[u8]> =  unsafe { defer!(buffer) };
            assert_eq!(0, deferred_mut[0]);
            assert_eq!(0, deferred[0]); // so far so good, no UB yet...
            deferred_mut[0] = 42; // this implicity creates a mutable reference to `buffer`
            // `deferred` is now invalidated!
            assert_eq!(42, deferred_mut[0]);
            //assert_eq!(42, deferred[0]); // this is UB!
        }
        #[test]
        fn doctest_defer2() {
            let mut buffer = [0u8; 1024];
            // SAFETY: this is safe because we reborrow `deferred_mut` and
            // SAFETY: we don't create any overlapping references.
            let mut deferred_mut: Deferred<&mut [u8]> = unsafe { defer_mut!(buffer) };
            let deferred: Deferred<&[u8]> = unsafe { deferred_mut.clone_unchecked().into_ref() };
            assert_eq!(0, deferred_mut[0]);
            assert_eq!(0, deferred[0]);
            deferred_mut[0] = 42; // this implicity creates a mutable reference to `buffer`
            assert_eq!(42, deferred_mut[0]);
            assert_eq!(42, deferred[0]); // but this is not UB thanks the re-borrow
        }
        #[test]
        fn doctest_defer3() {
            let mut buffer = [0u8; 1024];
            // SAFETY: this is safe, because we create new deferred references after
            // SAFETY: dereferencing `deferred_mut` into an actual mutable reference.
            let mut deferred_mut: Deferred<&mut [u8]> = unsafe { defer_mut!(buffer) };
            let mut deferred: Deferred<&[u8]> = unsafe { defer!(buffer) };
            assert_eq!(0, deferred_mut[0]);
            assert_eq!(0, deferred[0]); // so far so good, no UB yet...
            deferred_mut[0] = 42; // this implicity creates a temporary mutable reference to `buffer`
            // `deferred` is now invalidated! we refresh it again:
            deferred = unsafe { defer!(buffer) };
            assert_eq!(42, deferred_mut[0]);
            // this is not UB, because the mutable reference did not overlap
            // with the re-creation of `deferred`:
            assert_eq!(42, deferred[0]); 
        }
    }
}