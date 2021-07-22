//! Core utilities

use std::convert::{TryFrom, TryInto};

use rand::Rng;
use rand::distributions::{Distribution, Standard as StandardDist};

#[cfg(test)]
use quickcheck::{Arbitrary, Gen};


pub const FIELD_WIDTH: u8 = 8;
pub const FIELD_HEIGHT: u8 = 16;


/// Convenience type for positions
///
pub type Position = (RowIndex, ColumnIndex);

impl std::ops::Add<Direction> for Position {
    type Output = Option<Self>;

    fn add(self, rhs: Direction) -> Self::Output {
        match rhs {
            Direction::Left  => self.1.backward_checked(1).map(|c| (self.0, c)),
            Direction::Right => self.1.forward_checked(1).map(|c| (self.0, c)),
            Direction::Above => self.0.backward_checked(1).map(|r| (r, self.1)),
            Direction::Below => self.0.forward_checked(1).map(|r| (r, self.1)),
        }
    }
}

impl std::ops::Add<Direction> for Option<Position> {
    type Output = Option<Position>;

    fn add(self, rhs: Direction) -> Self::Output {
        self.and_then(|p| p + rhs)
    }
}

/// Description of a direction
///
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
    Above,
    Below,
}

impl Direction {
    /// Rotate the direction clockwise
    ///
    pub const fn rotated_cw(self) -> Self {
        match self {
            Self::Left  => Self::Above,
            Self::Above => Self::Right,
            Self::Right => Self::Below,
            Self::Below => Self::Left,
        }
    }

    /// Rotate the direction counter clockwise
    ///
    pub const fn rotated_ccw(self) -> Self {
        match self {
            Self::Left  => Self::Below,
            Self::Below => Self::Right,
            Self::Right => Self::Above,
            Self::Above => Self::Left,
        }
    }
}


/// Row index type
///
/// Instances of this type serve as an index for a row in a field. It represents
/// values from `0` (for the top row) to `15` (for the bottom row).
///
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
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


/// Range including all rows
///
pub const ROWS: RangeInclusive<RowIndex> = RangeInclusive::new(RowIndex::TOP_ROW, RowIndex::BOTTOM_ROW);


/// Column index type
///
/// Instances of this type serve as an index for a column in a field. It
/// represents values from `0` (for the leftmost column) to `7` (for the
/// rightmost column).
///
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd, Eq, Ord, Hash)]
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


/// Range including all columns
///
pub const COLUMNS: RangeInclusive<ColumnIndex> = RangeInclusive::new(
    ColumnIndex::LEFTMOST_COLUMN,
    ColumnIndex::RIGHTMOST_COLUMN
);


/// Project-specific partial predefinition of `std::iter::Step`
///
pub trait Step: Sized {
    /// Number of successor steps from start to end
    ///
    fn steps_between(start: &Self, end: &Self) -> Option<usize>;

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
    where I: TryFrom<usize> + Into<usize> + Clone
{
    fn steps_between(start: &Self, end: &Self) -> Option<usize> {
        end.clone().into().checked_sub(start.clone().into())
    }

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
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct RangeInclusive<I> {
    data: Option<(I, I)>,
}

impl<I> RangeInclusive<I> {
    /// Crate a new inclusive range
    ///
    pub const fn new(first: I, last: I) -> Self {
        Self {data: Some((first, last))}
    }
}

impl<I> From<std::ops::RangeInclusive<I>> for RangeInclusive<I> {
    fn from(range: std::ops::RangeInclusive<I>) -> Self {
        Self {data: Some(range.into_inner())}
    }
}

impl<I> std::iter::FusedIterator for RangeInclusive<I>
    where I: Step + PartialOrd + Clone
{
}

impl<I> ExactSizeIterator for RangeInclusive<I>
    where I: Step + PartialOrd + Clone
{
}

impl<I> DoubleEndedIterator for RangeInclusive<I>
    where I: Step + PartialOrd + Clone
{
    fn next_back(&mut self) -> Option<Self::Item> {
        self.data.take().map(|(first, last)| {
            let res = first.clone();
            if first < last {
                self.data = last.backward_checked(1).map(|last| (first, last))
            }
            res
        })
    }
}

impl<I> Iterator for RangeInclusive<I>
    where I: Step + PartialOrd + Clone
{
    type Item = I;

    fn next(&mut self) -> Option<Self::Item> {
        self.data.take().map(|(first, last)| {
            let res = first.clone();
            if first < last {
                self.data = first.forward_checked(1).map(|first| (first, last))
            }
            res
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self
            .data
            .as_ref()
            .and_then(|(first, last)| Step::steps_between(first, last))
            .map(|len| len.saturating_add(1)) // Should never saturate
            .unwrap_or(0);
        (len, Some(len))
    }
}


/// Create an iterator over all positions in the given row
///
pub fn complete_row(row: RowIndex) -> impl Iterator<Item = Position> {
    COLUMNS.map(move |c| (row, c))
}


/// Colour of viruses and capsule elements
///
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Colour {
    Red,
    Yellow,
    Blue,
}

impl Colour {
    /// Cycle through the colours
    ///
    /// This function chooses anothe colour based on `dir`. Each colour will be
    /// returned only once for three "rotatios" with a given `dir`.
    pub fn rotate(self, dir: bool) -> Self {
        match (self, dir) {
            (Self::Red,    false) => Self::Yellow,
            (Self::Red,    true ) => Self::Blue,
            (Self::Yellow, false) => Self::Blue,
            (Self::Yellow, true ) => Self::Red,
            (Self::Blue,   false) => Self::Red,
            (Self::Blue,   true ) => Self::Yellow,
        }
    }
}

impl Distribution<Colour> for StandardDist {
    fn sample<R: Rng + ?Sized>(&self, rng: &mut R) -> Colour {
        match rng.gen_range(0u8..3u8) {
            0 => Colour::Red,
            1 => Colour::Yellow,
            _ => Colour::Blue,
        }
    }
}

#[cfg(test)]
impl Arbitrary for Colour {
    fn arbitrary(g: &mut Gen) -> Self {
        *g.choose(&[Self::Red, Self::Yellow, Self::Blue]).unwrap()
    }
}


/// Trait for potentially coloured tile contents
///
pub trait PotentiallyColoured {
    /// Retrieve the colour of this item
    ///
    fn colour(&self) -> Option<Colour>;

    /// Convert the item into its potential colour
    ///
    fn into_colour(self) -> Option<Colour>
        where Self: Sized
    {
        self.colour()
    }
}

impl PotentiallyColoured for Option<Colour> {
    fn colour(&self) -> Option<Colour> {
        *self
    }
}

