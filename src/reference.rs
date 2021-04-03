mod private {
    pub trait Sealed {}
    impl<T: ?Sized> Sealed for &T {}
    impl<T: ?Sized> Sealed for &mut T {}
}

/// This trait is implemented for all references. Rust has only two types
/// of references to a type `T: ?Sized`, namely `&T` and `&mut T`.
/// This trait is sealed and can not be implemented for any other types.
///
/// # Example use
/// This trait makes it possible to accept both immutable and mutable references
/// as generic type parameters.
/// ```
/// use deferred_reference::Reference;
/// use core::fmt::Debug;
/// pub struct RefContainer<T>(T) where T: Reference;
/// fn main() {
///     let mut value = 42;
///     let immutable_ref = RefContainer(&value); // takes an immutable...
///     let mutable_ref = RefContainer(&mut value); // ... or mutable reference.
///     //let owned = RefContainer(value); // not a reference, so not allowed
///
///     // this also works for references to trait objects:
///     fn dyn_ref(reference: &dyn Debug) -> RefContainer<&dyn Debug> {
///         RefContainer(reference)
///     }
///     let dyn_ref: RefContainer<&dyn Debug> = dyn_ref(&value);
/// }
/// ```
pub trait Reference: private::Sealed {
    /// The type that the reference points to.
    type Target: ?Sized;
}
impl<T: ?Sized> Reference for &T {
    type Target = T;
}
impl<T: ?Sized> Reference for &mut T {
    type Target = T;
}