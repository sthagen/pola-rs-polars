use super::{new_empty_array, new_null_array, Array, Splitable};
use crate::bitmap::Bitmap;
use crate::datatypes::{ArrowDataType, Field};

mod ffi;
pub(super) mod fmt;
mod iterator;

mod mutable;
pub use mutable::*;
use polars_error::{polars_bail, polars_ensure, PolarsResult};
use polars_utils::pl_str::PlSmallStr;

/// The Arrow's equivalent to an immutable `Vec<Option<[T; size]>>` where `T` is an Arrow type.
/// Cloning and slicing this struct is `O(1)`.
#[derive(Clone)]
pub struct FixedSizeListArray {
    size: usize, // this is redundant with `dtype`, but useful to not have to deconstruct the dtype.
    length: usize, // invariant: this is values.len() / size if size > 0
    dtype: ArrowDataType,
    values: Box<dyn Array>,
    validity: Option<Bitmap>,
}

impl FixedSizeListArray {
    /// Creates a new [`FixedSizeListArray`].
    ///
    /// # Errors
    /// This function returns an error iff:
    /// * The `dtype`'s physical type is not [`crate::datatypes::PhysicalType::FixedSizeList`]
    /// * The `dtype`'s inner field's data type is not equal to `values.dtype`.
    /// * The length of `values` is not a multiple of `size` in `dtype`
    /// * the validity's length is not equal to `values.len() / size`.
    pub fn try_new(
        dtype: ArrowDataType,
        length: usize,
        values: Box<dyn Array>,
        validity: Option<Bitmap>,
    ) -> PolarsResult<Self> {
        let (child, size) = Self::try_child_and_size(&dtype)?;

        let child_dtype = &child.dtype;
        let values_dtype = values.dtype();
        if child_dtype != values_dtype {
            polars_bail!(ComputeError: "FixedSizeListArray's child's DataType must match. However, the expected DataType is {child_dtype:?} while it got {values_dtype:?}.")
        }

        polars_ensure!(size == 0 || values.len() % size == 0, ComputeError:
            "values (of len {}) must be a multiple of size ({}) in FixedSizeListArray.",
            values.len(),
            size
        );

        polars_ensure!(size == 0 || values.len() / size == length, ComputeError:
            "length of values ({}) is not equal to given length ({}) in FixedSizeListArray({size}).",
            values.len() / size,
            length,
        );
        polars_ensure!(size != 0 || values.len() == 0, ComputeError:
            "zero width FixedSizeListArray has values (length = {}).",
            values.len(),
        );

        if validity
            .as_ref()
            .map_or(false, |validity| validity.len() != length)
        {
            polars_bail!(ComputeError: "validity mask length must be equal to the number of values divided by size")
        }

        Ok(Self {
            size,
            length,
            dtype,
            values,
            validity,
        })
    }

    #[inline]
    fn has_invariants(&self) -> bool {
        let has_valid_length = (self.size == 0 && self.values().len() == 0)
            || (self.size > 0
                && self.values().len() % self.size() == 0
                && self.values().len() / self.size() == self.length);
        let has_valid_validity = self
            .validity
            .as_ref()
            .map_or(true, |v| v.len() == self.length);

        has_valid_length && has_valid_validity
    }

    /// Alias to `Self::try_new(...).unwrap()`
    #[track_caller]
    pub fn new(
        dtype: ArrowDataType,
        length: usize,
        values: Box<dyn Array>,
        validity: Option<Bitmap>,
    ) -> Self {
        Self::try_new(dtype, length, values, validity).unwrap()
    }

    /// Returns the size (number of elements per slot) of this [`FixedSizeListArray`].
    pub const fn size(&self) -> usize {
        self.size
    }

    /// Returns a new empty [`FixedSizeListArray`].
    pub fn new_empty(dtype: ArrowDataType) -> Self {
        let values = new_empty_array(Self::get_child_and_size(&dtype).0.dtype().clone());
        Self::new(dtype, 0, values, None)
    }

    /// Returns a new null [`FixedSizeListArray`].
    pub fn new_null(dtype: ArrowDataType, length: usize) -> Self {
        let (field, size) = Self::get_child_and_size(&dtype);

        let values = new_null_array(field.dtype().clone(), length * size);
        Self::new(dtype, length, values, Some(Bitmap::new_zeroed(length)))
    }
}

// must use
impl FixedSizeListArray {
    /// Slices this [`FixedSizeListArray`].
    /// # Implementation
    /// This operation is `O(1)`.
    /// # Panics
    /// panics iff `offset + length > self.len()`
    pub fn slice(&mut self, offset: usize, length: usize) {
        assert!(
            offset + length <= self.len(),
            "the offset of the new Buffer cannot exceed the existing length"
        );
        unsafe { self.slice_unchecked(offset, length) }
    }

    /// Slices this [`FixedSizeListArray`].
    /// # Implementation
    /// This operation is `O(1)`.
    ///
    /// # Safety
    /// The caller must ensure that `offset + length <= self.len()`.
    pub unsafe fn slice_unchecked(&mut self, offset: usize, length: usize) {
        self.validity = self
            .validity
            .take()
            .map(|bitmap| bitmap.sliced_unchecked(offset, length))
            .filter(|bitmap| bitmap.unset_bits() > 0);
        self.values
            .slice_unchecked(offset * self.size, length * self.size);
        self.length = length;
    }

    impl_sliced!();
    impl_mut_validity!();
    impl_into_array!();
}

// accessors
impl FixedSizeListArray {
    /// Returns the length of this array
    #[inline]
    pub fn len(&self) -> usize {
        debug_assert!(self.has_invariants());
        self.length
    }

    /// The optional validity.
    #[inline]
    pub fn validity(&self) -> Option<&Bitmap> {
        self.validity.as_ref()
    }

    /// Returns the inner array.
    pub fn values(&self) -> &Box<dyn Array> {
        &self.values
    }

    /// Returns the `Vec<T>` at position `i`.
    /// # Panic:
    /// panics iff `i >= self.len()`
    #[inline]
    pub fn value(&self, i: usize) -> Box<dyn Array> {
        self.values.sliced(i * self.size, self.size)
    }

    /// Returns the `Vec<T>` at position `i`.
    ///
    /// # Safety
    /// Caller must ensure that `i < self.len()`
    #[inline]
    pub unsafe fn value_unchecked(&self, i: usize) -> Box<dyn Array> {
        self.values.sliced_unchecked(i * self.size, self.size)
    }

    /// Returns the element at index `i` or `None` if it is null
    /// # Panics
    /// iff `i >= self.len()`
    #[inline]
    pub fn get(&self, i: usize) -> Option<Box<dyn Array>> {
        if !self.is_null(i) {
            // soundness: Array::is_null panics if i >= self.len
            unsafe { Some(self.value_unchecked(i)) }
        } else {
            None
        }
    }
}

impl FixedSizeListArray {
    pub(crate) fn try_child_and_size(dtype: &ArrowDataType) -> PolarsResult<(&Field, usize)> {
        match dtype.to_logical_type() {
            ArrowDataType::FixedSizeList(child, size) => Ok((child.as_ref(), *size)),
            _ => polars_bail!(ComputeError: "FixedSizeListArray expects DataType::FixedSizeList"),
        }
    }

    pub(crate) fn get_child_and_size(dtype: &ArrowDataType) -> (&Field, usize) {
        Self::try_child_and_size(dtype).unwrap()
    }

    /// Returns a [`ArrowDataType`] consistent with [`FixedSizeListArray`].
    pub fn default_datatype(dtype: ArrowDataType, size: usize) -> ArrowDataType {
        let field = Box::new(Field::new(PlSmallStr::from_static("item"), dtype, true));
        ArrowDataType::FixedSizeList(field, size)
    }
}

impl Array for FixedSizeListArray {
    impl_common_array!();

    fn validity(&self) -> Option<&Bitmap> {
        self.validity.as_ref()
    }

    #[inline]
    fn with_validity(&self, validity: Option<Bitmap>) -> Box<dyn Array> {
        Box::new(self.clone().with_validity(validity))
    }
}

impl Splitable for FixedSizeListArray {
    fn check_bound(&self, offset: usize) -> bool {
        offset <= self.len()
    }

    unsafe fn _split_at_unchecked(&self, offset: usize) -> (Self, Self) {
        let (lhs_values, rhs_values) =
            unsafe { self.values.split_at_boxed_unchecked(offset * self.size) };
        let (lhs_validity, rhs_validity) = unsafe { self.validity.split_at_unchecked(offset) };

        let size = self.size;

        (
            Self {
                dtype: self.dtype.clone(),
                length: offset,
                values: lhs_values,
                validity: lhs_validity,
                size,
            },
            Self {
                dtype: self.dtype.clone(),
                length: self.length - offset,
                values: rhs_values,
                validity: rhs_validity,
                size,
            },
        )
    }
}
