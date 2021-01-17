//! Definition of a type representing a signle row

use crate::util;


/// A single row within a field
///
/// This type is indended as an internal convenience utility for construcing
/// fields.
///
#[derive(Default)]
pub struct Row<T>
    where T: Default
{
    data: [T; util::FIELD_WIDTH as usize],
}

impl<T> std::ops::IndexMut<util::ColumnIndex> for Row<T>
    where T: Default
{
    fn index_mut(&mut self, index: util::ColumnIndex) -> &mut Self::Output {
        &mut self.data[usize::from(index)]
    }
}

impl<T> std::ops::Index<util::ColumnIndex> for Row<T>
    where T: Default
{
    type Output = T;

    fn index(&self, index: util::ColumnIndex) -> &Self::Output {
        &self.data[usize::from(index)]
    }
}

