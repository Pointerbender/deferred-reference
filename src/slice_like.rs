use crate::PointerLength;

/// This trait is implemented for all slice-like structures. Currently there are only 2 of these types: arrays `[T; N]` and slices `[T]`.
pub trait SliceLike: PointerLength {
    /// The type of the elements held by the slice-like structure.
    type Element;
}

impl<T> SliceLike for [T] {
    type Element = T;
}

impl<T, const N: usize> SliceLike for [T; N] {
    type Element = T;
}