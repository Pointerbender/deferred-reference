use core::ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive};

use crate::{PointerLength, SliceLike};

/// A helper trait used for indexing operations, which is modeled after the [SliceIndex](core::slice::SliceIndex) trait
/// from the Rust core library, but which promises not to take a reference to the underlying slice.
///
/// # Safety
/// Implementations of this trait have to promise that:
/// * If the argument to get_(mut_)unchecked is a safe pointer, then so is the result.
/// * The pointers may not be dereferenced (both pointers given as arguments as well as the returned pointers).
pub unsafe trait SlicePointerIndex<T>/*: private_slice_index::Sealed*/
where
    T: SliceLike + ?Sized,
{
    /// The output type returned by methods.
    type Output: ?Sized;

    /// Returns a shared pointer to the output at this location, if in bounds.
    fn get(self, slice: *const T) -> Option<*const Self::Output>;

    /// Returns a mutable poiner to the output at this location, if in bounds.
    fn get_mut(self, slice: *mut T) -> Option<*mut Self::Output>;

    /// Returns a shared pointer to the output at this location, without
    /// performing any bounds checking.
    /// Calling this method with an out-of-bounds index or a dangling `slice` pointer
    /// is *[undefined behavior]* even if the resulting reference is not used.
    ///
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    unsafe fn get_unchecked(self, slice: *const T) -> *const Self::Output;

    /// Returns a mutable pointer to the output at this location, without
    /// performing any bounds checking.
    /// Calling this method with an out-of-bounds index or a dangling `slice` pointer
    /// is *[undefined behavior]* even if the resulting reference is not used.
    ///
    /// [undefined behavior]: https://doc.rust-lang.org/reference/behavior-considered-undefined.html
    unsafe fn get_unchecked_mut(self, slice: *mut T) -> *mut Self::Output;

    /// Returns a shared pointer to the output at this location, panicking
    /// if out of bounds.
    #[track_caller]
    fn index(self, slice: *const T) -> *const Self::Output;

    /// Returns a mutable pointer to the output at this location, panicking
    /// if out of bounds.
    #[track_caller]
    fn index_mut(self, slice: *mut T) -> *mut Self::Output;
}

#[inline(never)]
#[cold]
#[track_caller]
fn slice_start_index_len_fail(index: usize, len: usize) -> ! {
    panic!("range start index {} out of range for slice pointer of length {}", index, len);
}

#[inline(never)]
#[cold]
#[track_caller]
fn slice_end_index_len_fail(index: usize, len: usize) -> ! {
    panic!("range end index {} out of range for slice pointer of length {}", index, len);
}

#[inline(never)]
#[cold]
#[track_caller]
fn slice_index_order_fail(index: usize, end: usize) -> ! {
    panic!("slice pointer index starts at {} but ends at {}", index, end);
}

// #[inline(never)]
// #[cold]
// #[track_caller]
// pub(crate) fn slice_start_index_overflow_fail() -> ! {
//     panic!("attempted to index slice pointer from after maximum usize");
// }

#[inline(never)]
#[cold]
#[track_caller]
fn slice_end_index_overflow_fail() -> ! {
    panic!("attempted to index slice pointer up to maximum usize");
}

#[inline(never)]
#[cold]
#[track_caller]
fn slice_index_overflow_fail(index: usize, len: usize) -> ! {
    panic!("index {} out of range for slice pointer of length {}", index, len);
}

unsafe impl<T> SlicePointerIndex<T> for usize
where
    T: SliceLike + ?Sized,
{
    type Output = T::Element;

    #[inline]
    fn get(self, slice: *const T) -> Option<*const Self::Output> {
        // SAFETY: `self` is checked to be in bounds.
        if self < PointerLength::len(slice) { unsafe { Some(self.get_unchecked(slice)) } } else { None }
    }

    #[inline]
    fn get_mut(self, slice: *mut T) -> Option<*mut Self::Output> {
        // SAFETY: `self` is checked to be in bounds.
        if self < PointerLength::len(slice) { unsafe { Some(self.get_unchecked_mut(slice)) } } else { None }
    }

    #[inline]
    unsafe fn get_unchecked(self, slice: *const T) -> *const Self::Output {
        // SAFETY: the caller guarantees that `slice` is not dangling, so it
        // cannot be longer than `isize::MAX`. They also guarantee that
        // `self` is in bounds of `slice` so `self` cannot overflow an `isize`,
        // so the call to `add` is safe.
        (slice as *const T::Element).add(self)
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut T) -> *mut Self::Output {
        // SAFETY: see comments for `get_unchecked` above.
        (slice as *mut T::Element).add(self)
    }

    #[inline]
    fn index(self, slice: *const T) -> *const Self::Output {
        if self > PointerLength::len(slice) {
            slice_index_overflow_fail(self, PointerLength::len(slice))
        }
        // SAFETY: this is safe, bounds are checked above
        unsafe {
            self.get_unchecked(slice)
        }
    }

    #[inline]
    fn index_mut(self, slice: *mut T) -> *mut Self::Output {
        if self > PointerLength::len(slice) {
            slice_index_overflow_fail(self, PointerLength::len(slice))
        }
        // SAFETY: this is safe, bounds are checked above
        unsafe {
            self.get_unchecked_mut(slice)
        }
    }
}

unsafe impl<T> SlicePointerIndex<T> for Range<usize>
where
    T: SliceLike + ?Sized,
{
    type Output = [T::Element];

    #[inline]
    fn get(self, slice: *const T) -> Option<*const Self::Output> {
        if self.start > self.end || self.end > PointerLength::len(slice) {
            None
        } else {
            // SAFETY: `self` is checked to be valid and in bounds above.
            unsafe { Some(self.get_unchecked(slice)) }
        }
    }

    #[inline]
    fn get_mut(self, slice: *mut T) -> Option<*mut Self::Output> {
        if self.start > self.end || self.end > PointerLength::len(slice) {
            None
        } else {
            // SAFETY: `self` is checked to be valid and in bounds above.
            unsafe { Some(self.get_unchecked_mut(slice)) }
        }
    }

    #[inline]
    unsafe fn get_unchecked(self, slice: *const T) -> *const Self::Output {
        // SAFETY: the caller guarantees that `slice` is not dangling, so it
        // cannot be longer than `isize::MAX`. They also guarantee that
        // `self` is in bounds of `slice` so `self` cannot overflow an `isize`,
        // so the call to `add` is safe.
        core::ptr::slice_from_raw_parts((slice as *const T::Element).add(self.start), self.end - self.start)
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut T) -> *mut Self::Output {
        // SAFETY: see comments for `get_unchecked` above.
        core::ptr::slice_from_raw_parts_mut((slice as *mut T::Element).add(self.start), self.end - self.start)
    }

    #[inline]
    fn index(self, slice: *const T) -> *const Self::Output {
        if self.start > self.end {
            slice_index_order_fail(self.start, self.end);
        } else if self.end > PointerLength::len(slice) {
            slice_end_index_len_fail(self.end, PointerLength::len(slice));
        }
        // SAFETY: `self` is checked to be valid and in bounds above.
        unsafe { self.get_unchecked(slice) }
    }

    #[inline]
    fn index_mut(self, slice: *mut T) -> *mut Self::Output {
        if self.start > self.end {
            slice_index_order_fail(self.start, self.end);
        } else if self.end > PointerLength::len(slice) {
            slice_end_index_len_fail(self.end, PointerLength::len(slice));
        }
        // SAFETY: `self` is checked to be valid and in bounds above.
        unsafe { self.get_unchecked_mut(slice) }
    }
}

unsafe impl<T> SlicePointerIndex<T> for RangeTo<usize>
where
    T: SliceLike + ?Sized,
{
    type Output = [T::Element];

    #[inline]
    fn get(self, slice: *const T) -> Option<*const Self::Output> {
        (0..self.end).get(slice)
    }

    #[inline]
    fn get_mut(self, slice: *mut T) -> Option<*mut Self::Output> {
        (0..self.end).get_mut(slice)
    }

    #[inline]
    unsafe fn get_unchecked(self, slice: *const T) -> *const Self::Output {
        // SAFETY: the caller has to uphold the safety contract for `get_unchecked`.
        (0..self.end).get_unchecked(slice)
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut T) -> *mut Self::Output {
        // SAFETY: the caller has to uphold the safety contract for `get_unchecked_mut`.
        (0..self.end).get_unchecked_mut(slice)
    }

    #[inline]
    fn index(self, slice: *const T) -> *const Self::Output {
        (0..self.end).index(slice)
    }

    #[inline]
    fn index_mut(self, slice: *mut T) -> *mut Self::Output {
        (0..self.end).index_mut(slice)
    }
}

unsafe impl<T> SlicePointerIndex<T> for RangeFrom<usize>
where
    T: SliceLike + ?Sized,
{
    type Output = [T::Element];

    #[inline]
    fn get(self, slice: *const T) -> Option<*const Self::Output> {
        (self.start..PointerLength::len(slice)).get(slice)
    }

    #[inline]
    fn get_mut(self, slice: *mut T) -> Option<*mut Self::Output> {
        (self.start..PointerLength::len(slice)).get_mut(slice)
    }

    #[inline]
    unsafe fn get_unchecked(self, slice: *const T) -> *const Self::Output {
        // SAFETY: the caller has to uphold the safety contract for `get_unchecked`.
        (self.start..PointerLength::len(slice)).get_unchecked(slice)
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut T) -> *mut Self::Output {
        // SAFETY: the caller has to uphold the safety contract for `get_unchecked_mut`.
        (self.start..PointerLength::len(slice)).get_unchecked_mut(slice)
    }

    #[inline]
    fn index(self, slice: *const T) -> *const Self::Output {
        if self.start > PointerLength::len(slice) {
            slice_start_index_len_fail(self.start, PointerLength::len(slice));
        }
        // SAFETY: `self` is checked to be valid and in bounds above.
        unsafe { self.get_unchecked(slice) }
    }

    #[inline]
    fn index_mut(self, slice: *mut T) -> *mut Self::Output {
        if self.start > PointerLength::len(slice) {
            slice_start_index_len_fail(self.start, PointerLength::len(slice));
        }
        // SAFETY: `self` is checked to be valid and in bounds above.
        unsafe { self.get_unchecked_mut(slice) }
    }
}

unsafe impl<T> SlicePointerIndex<[T]> for RangeFull {
    type Output = [T];

    #[inline]
    fn get(self, slice: *const [T]) -> Option<*const [T]> {
        Some(slice)
    }

    #[inline]
    fn get_mut(self, slice: *mut [T]) -> Option<*mut [T]> {
        Some(slice)
    }

    #[inline]
    unsafe fn get_unchecked(self, slice: *const [T]) -> *const [T] {
        slice
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut [T]) -> *mut [T] {
        slice
    }

    #[inline]
    fn index(self, slice: *const [T]) -> *const [T] {
        slice
    }

    #[inline]
    fn index_mut(self, slice: *mut [T]) -> *mut [T] {
        slice
    }
}

unsafe impl<T, const N: usize> SlicePointerIndex<[T; N]> for RangeFull {
    type Output = [T];

    #[inline]
    fn get(self, slice: *const [T; N]) -> Option<*const [T]> {
        Some(slice)
    }

    #[inline]
    fn get_mut(self, slice: *mut [T; N]) -> Option<*mut [T]> {
        Some(slice)
    }

    #[inline]
    unsafe fn get_unchecked(self, slice: *const [T; N]) -> *const [T] {
        slice
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut [T; N]) -> *mut [T] {
        slice
    }

    #[inline]
    fn index(self, slice: *const [T; N]) -> *const [T] {
        slice
    }

    #[inline]
    fn index_mut(self, slice: *mut [T; N]) -> *mut [T] {
        slice
    }
}

/// Converts to an exclusive `Range` for `SliceIndex` implementations.
/// The caller is responsible for dealing with `end == usize::MAX`.
#[inline]
fn into_slice_range(range_inclusive: RangeInclusive<usize>) -> Range<usize> {
    // If we're not exhausted, we want to simply slice `start..end + 1`.
    // If we are exhausted, then slicing with `end + 1..end + 1` gives us an
    // empty range that is still subject to bounds-checks for that endpoint.
    let exclusive_end = *range_inclusive.end() + 1;
    let start = if range_inclusive.is_empty() { exclusive_end } else { *range_inclusive.start() };
    start..exclusive_end
}

unsafe impl<T> SlicePointerIndex<T> for RangeInclusive<usize>
where
    T: SliceLike + ?Sized,
{
    type Output = [T::Element];

    #[inline]
    fn get(self, slice: *const T) -> Option<*const Self::Output> {
        if *self.end() == usize::MAX { None } else { into_slice_range(self).get(slice) }
    }

    #[inline]
    fn get_mut(self, slice: *mut T) -> Option<*mut Self::Output> {
        if *self.end() == usize::MAX { None } else { into_slice_range(self).get_mut(slice) }
    }

    #[inline]
    unsafe fn get_unchecked(self, slice: *const T) -> *const Self::Output {
        // SAFETY: the caller has to uphold the safety contract for `get_unchecked`.
        into_slice_range(self).get_unchecked(slice)
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut T) -> *mut Self::Output {
        // SAFETY: the caller has to uphold the safety contract for `get_unchecked_mut`.
        into_slice_range(self).get_unchecked_mut(slice)
    }

    #[inline]
    fn index(self, slice: *const T) -> *const Self::Output {
        if *self.end() == usize::MAX {
            slice_end_index_overflow_fail();
        }
        into_slice_range(self).index(slice)
    }

    #[inline]
    fn index_mut(self, slice: *mut T) -> *mut Self::Output {
        if *self.end() == usize::MAX {
            slice_end_index_overflow_fail();
        }
        into_slice_range(self).index_mut(slice)
    }
}

unsafe impl<T> SlicePointerIndex<T> for RangeToInclusive<usize>
where
    T: SliceLike + ?Sized,
{
    type Output = [T::Element];

    #[inline]
    fn get(self, slice: *const T) -> Option<*const Self::Output> {
        (0..=self.end).get(slice)
    }

    #[inline]
    fn get_mut(self, slice: *mut T) -> Option<*mut Self::Output> {
        (0..=self.end).get_mut(slice)
    }

    #[inline]
    unsafe fn get_unchecked(self, slice: *const T) -> *const Self::Output {
        // SAFETY: the caller has to uphold the safety contract for `get_unchecked`.
        (0..=self.end).get_unchecked(slice)
    }

    #[inline]
    unsafe fn get_unchecked_mut(self, slice: *mut T) -> *mut Self::Output {
        // SAFETY: the caller has to uphold the safety contract for `get_unchecked_mut`.
        (0..=self.end).get_unchecked_mut(slice)
    }

    #[inline]
    fn index(self, slice: *const T) -> *const Self::Output {
        (0..=self.end).index(slice)
    }

    #[inline]
    fn index_mut(self, slice: *mut T) -> *mut Self::Output {
        (0..=self.end).index_mut(slice)
    }
}
