//! Utility tests

use super::*;


#[test]
fn rows_len() {
    assert_eq!(ROWS.len(), FIELD_HEIGHT as usize);
    assert_eq!(ROWS.count(), FIELD_HEIGHT as usize);
    assert_eq!(ROWS.rfold(0, |c, _| c + 1), FIELD_HEIGHT as usize);
}


#[test]
fn columns_len() {
    assert_eq!(COLUMNS.len(), FIELD_WIDTH as usize);
    assert_eq!(COLUMNS.count(), FIELD_WIDTH as usize);
    assert_eq!(COLUMNS.rfold(0, |c, _| c + 1), FIELD_WIDTH as usize);
}


#[quickcheck]
fn colour_rotation(colour: Colour, dir: bool) -> bool {
    let c1 = colour.rotate(dir);
    let c2 = c1.rotate(dir);
    let c3 = c2.rotate(dir);
    c1 != colour && c2 != colour && c3 == colour
}

