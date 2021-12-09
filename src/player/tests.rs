//! Player tests

use quickcheck::{Arbitrary, Gen};

use super::*;


/// Utility for generting a valid player name
///
#[derive(Clone, Debug)]
pub struct Name(pub String);

impl From<Name> for String {
    fn from(name: Name) -> Self {
        name.0
    }
}

impl Arbitrary for Name {
    fn arbitrary(g: &mut Gen) -> Self {
        let len = usize::arbitrary(g) % (MAX_PLAYER_NAME_LEN - 1) + 1;
        let res = (0..len)
            .filter_map(|_| char::from_u32(u32::arbitrary(g) % (0x7F - 0x20) + 0x20))
            .collect();
        Self(res)
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        let res = self
            .0
            .shrink()
            .filter(|n| n.len() > 0 && n.chars().all(|c| c.is_ascii() && !c.is_ascii_control()))
            .map(Self);
        Box::new(res)
    }
}


#[quickcheck]
fn name_gen(name: Name) -> bool {
    name.0.chars().all(|c| c.is_ascii() && !c.is_ascii_control()) &&
        name.0.len() > 0 &&
        name.0.len() <= MAX_PLAYER_NAME_LEN
}

