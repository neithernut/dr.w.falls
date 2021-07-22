//! Utility tests

use super::*;


#[quickcheck]
fn colour_rotation(colour: Colour, dir: bool) -> bool {
    let c1 = colour.rotate(dir);
    let c2 = c1.rotate(dir);
    let c3 = c2.rotate(dir);
    c1 != colour && c2 != colour && c3 == colour
}

