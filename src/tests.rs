//! General tests and testing utilities

use std::fmt;

use quickcheck::{Arbitrary, Gen};


/// Utility for generating random ASCII text
///
#[derive(Clone, Debug)]
pub struct ASCIIString(pub String);

impl From<String> for ASCIIString {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<ASCIIString> for String {
    fn from(s: ASCIIString) -> Self {
        s.0
    }
}

impl fmt::Display for ASCIIString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl Arbitrary for ASCIIString {
    fn arbitrary(g: &mut Gen) -> Self {
        let len = u8::arbitrary(g) as usize + 1;
        std::iter::from_fn(|| char::from_u32(u32::arbitrary(g) % (0x7F - 0x20) + 0x20))
            .take(len)
            .collect::<String>()
            .into()
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        let res = self
            .0
            .shrink()
            .filter(|n| n.len() > 0 && n.chars().all(|c| c.is_ascii() && !c.is_ascii_control()))
            .map(Into::into);
        Box::new(res)
    }
}

