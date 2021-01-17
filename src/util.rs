//! Core utilities

use std::convert::{TryFrom, TryInto};


pub const FIELD_WIDTH: u8 = 8;
pub const FIELD_HEIGHT: u8 = 16;


/// Row index type
///
/// Instances of this type serve as an index for a row in a field. It represents
/// values from `0` (for the top row) to `15` (for the bottom row).
///
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct RowIndex {
    data: u8,
}

impl RowIndex {
    /// Index of the top row
    ///
    pub const TOP_ROW: Self = Self {data: 0};

    /// Index of the bottom row
    ///
    pub const BOTTOM_ROW: Self = Self {data: FIELD_HEIGHT - 1};
}

impl From<RowIndex> for usize {
    fn from(index: RowIndex) -> Self {
        index.data.into()
    }
}

impl TryFrom<usize> for RowIndex {
    type Error = usize;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        value.try_into().ok().filter(|i| *i < FIELD_HEIGHT).map(|data| Self {data}).ok_or(value)
    }
}


/// Column index type
///
/// Instances of this type serve as an index for a column in a field. It
/// represents values from `0` (for the leftmost column) to `7` (for the
/// rightmost column).
///
#[derive(Copy, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct ColumnIndex {
    data: u8,
}

impl ColumnIndex {
    /// Index of the leftmost column
    ///
    pub const LEFTMOST_COLUMN: Self = Self {data: 0};

    /// Index of the rightmost column
    ///
    pub const RIGHTMOST_COLUMN: Self = Self {data: FIELD_WIDTH - 1};
}

impl From<ColumnIndex> for usize {
    fn from(index: ColumnIndex) -> Self {
        index.data.into()
    }
}

impl TryFrom<usize> for ColumnIndex {
    type Error = usize;
    fn try_from(value: usize) -> Result<Self, Self::Error> {
        value.try_into().ok().filter(|i| *i < FIELD_WIDTH).map(|data| Self {data}).ok_or(value)
    }
}


/// Project-specific partial predefinition of `std::iter::Step`
///
trait Step: Sized {
    /// Checked integer addition
    ///
    /// This function returns an index for the `count`'th next row or column. If
    /// the resulting row or column would be outside the field, the function
    /// returns `None`.
    ///
    fn forward_checked(self, count: usize) -> Option<Self>;

    /// Checked integer substraction
    ///
    /// This function returns an index for the `count`'th previous row or
    /// column. If the resulting row or column would be outside the field, the
    /// function returns `None`.
    ///
    fn backward_checked(self, count: usize) -> Option<Self>;
}

impl<I> Step for I
    where I: TryFrom<usize> + Into<usize>
{
    fn forward_checked(self, count: usize) -> Option<Self> {
        self.into().checked_add(count).and_then(|i| i.try_into().ok())
    }

    fn backward_checked(self, count: usize) -> Option<Self> {
        self.into().checked_sub(count).and_then(|i| i.try_into().ok())
    }
}


/// Inclusive range of rows or columns
///
/// This is somewhat of a reimplementation of `std::ops::RangeInclusive`, which
/// implements `DoubleEndedIterator` for all indices implementing our custom
/// `Step` trait.
///
struct RangeInclusive<I> {
    data: Option<(I, I)>,
}

impl<I> RangeInclusive<I> {
    /// Crate a new inclusive range
    ///
    pub fn new(first: I, last: I) -> Self {
        Self {data: Some((first, last))}
    }
}

impl<I> From<std::ops::RangeInclusive<I>> for RangeInclusive<I> {
    fn from(range: std::ops::RangeInclusive<I>) -> Self {
        Self {data: Some(range.into_inner())}
    }
}

impl<I> std::iter::FusedIterator for RangeInclusive<I>
    where I: Step + PartialEq + Clone
{
}

impl<I> DoubleEndedIterator for RangeInclusive<I>
    where I: Step + PartialEq + Clone
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.data.take().map(|(first, last)| {
            let res = first.clone();
            if first != last {
                self.data = last.backward_checked(1).map(|last| (first, last))
            }
            res
        })
    }
}

impl<I> Iterator for RangeInclusive<I>
    where I: Step + PartialEq + Clone
{
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        self.data.take().map(|(first, last)| {
            let res = first.clone();
            if first != last {
                self.data = first.forward_checked(1).map(|first| (first, last))
            }
            res
        })
    }
}

