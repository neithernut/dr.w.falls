//! Field tests

use super::*;


#[quickcheck]
fn row_of_four_len(row: items::RowOfFour) -> bool {
    row.len() == row.count()
}

