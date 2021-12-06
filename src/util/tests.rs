//! Utility tests

use quickcheck::TestResult;

use super::*;


#[quickcheck]
fn position_idendity(pos: Position, dir: Direction) -> TestResult {
    (pos + dir + dir.rotated_cw().rotated_cw())
        .map(|p| if pos == p {
            TestResult::passed()
        } else {
            TestResult::error(format!("Difference: {:?}, {:?}", pos, p))
        }).unwrap_or(TestResult::discard())
}


#[quickcheck]
fn rotation_idendity(dir: Direction) -> bool {
    dir.rotated_cw().rotated_ccw() == dir
}


#[quickcheck]
fn cw_rotation(dir: Direction) -> bool {
    let d1 = dir.rotated_cw();
    let d2 = d1.rotated_cw();
    let d3 = d2.rotated_cw();
    let d4 = d3.rotated_cw();
    d1 != dir && d2 != dir && d3 != dir && d4 == dir
}


#[quickcheck]
fn ccw_rotation(dir: Direction) -> bool {
    let d1 = dir.rotated_ccw();
    let d2 = d1.rotated_ccw();
    let d3 = d2.rotated_ccw();
    let d4 = d3.rotated_ccw();
    d1 != dir && d2 != dir && d3 != dir && d4 == dir
}


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
fn rows_forward(first: RowIndex, last: RowIndex) -> TestResult {
    if let Some(steps) = Step::steps_between(&first, &last) {
        let range = RangeInclusive::new(first, last);
        TestResult::from_bool(
            range.clone().nth(0) == Some(first) && range.clone().nth(steps) == Some(last)
        )
    } else {
        TestResult::discard()
    }
}


#[quickcheck]
fn rows_backward(first: RowIndex, last: RowIndex) -> TestResult {
    if let Some(steps) = Step::steps_between(&first, &last) {
        let range = RangeInclusive::new(first, last).rev();
        TestResult::from_bool(
            range.clone().nth(0) == Some(last) && range.clone().nth(steps) == Some(first)
        )
    } else {
        TestResult::discard()
    }
}


#[quickcheck]
fn colour_rotation(colour: Colour, dir: bool) -> bool {
    let c1 = colour.rotate(dir);
    let c2 = c1.rotate(dir);
    let c3 = c2.rotate(dir);
    c1 != colour && c2 != colour && c3 == colour
}

#[quickcheck]
fn colour_dirot(colour: Colour) -> bool {
    let c1 = colour.rotate(true);
    let c2 = colour.rotate(false);
    colour != c1 && colour != c2 && c1 != c2
}

