//! Player tests

use quickcheck::{Arbitrary, Gen, TestResult};

use super::*;


#[quickcheck]
fn handle_drop(name: Name, addr: std::net::SocketAddr) -> std::io::Result<bool> {
    let rt = tokio::runtime::Runtime::new()?;

    let (notifier, mut receiver) = tokio::sync::mpsc::unbounded_channel();
    let task = rt.spawn(std::future::pending());
    let handle = Handle::new(Arc::new(Data::new(name.into(), addr, task)), notifier);

    let tag = handle.tag();
    drop(handle);
    Ok(!tag.is_connected() && rt.block_on(receiver.recv()) == Some(tag))
}


#[quickcheck]
fn tag_eq(name: Name, addr: std::net::SocketAddr) -> std::io::Result<bool> {
    let rt = tokio::runtime::Runtime::new()?;

    let (notifier, _) = tokio::sync::mpsc::unbounded_channel();
    let task = rt.spawn(std::future::pending());
    let handle = Handle::new(Arc::new(Data::new(name.into(), addr, task)), notifier);

    let tag1 = handle.tag();
    let tag2 = handle.tag();
    Ok(handle == tag1 && tag1 == tag2)
}


#[quickcheck]
fn tag_neq(name: Name, addr: std::net::SocketAddr) -> std::io::Result<bool> {
    let rt = tokio::runtime::Runtime::new()?;

    let name: String = name.into();
    let (notifier, _) = tokio::sync::mpsc::unbounded_channel();
    let handle1 = {
        let task = rt.spawn(std::future::pending());
        Handle::new(Arc::new(Data::new(name.clone(), addr, task)), notifier.clone())
    };

    let handle2 = {
        let task = rt.spawn(std::future::pending());
        Handle::new(Arc::new(Data::new(name.clone(), addr, task)), notifier.clone())
    };

    let tag1 = handle1.tag();
    let tag2 = handle2.tag();
    Ok(tag1 != tag2)
}


#[quickcheck]
fn data_score(name: Name, addr: std::net::SocketAddr, add: Vec<u32>) -> std::io::Result<TestResult> {
    if let Some(expected) = add.iter().try_fold(0, |a: u32, v| a.checked_add(*v)) {
        let rt = tokio::runtime::Runtime::new()?;

        let name: String = name.into();
        let task = rt.spawn(std::future::pending());
        let data = Data::new(name.clone(), addr, task);
        add.into_iter().for_each(|v| { data.add_score(v); });
        Ok(TestResult::from_bool(expected == data.score()))
    } else {
        Ok(TestResult::discard())
    }
}


#[quickcheck]
fn data_init(name: Name, addr: std::net::SocketAddr) -> std::io::Result<bool> {
    let rt = tokio::runtime::Runtime::new()?;

    let name: String = name.into();
    let task = rt.spawn(std::future::pending());
    let data = Data::new(name.clone(), addr, task);
    Ok(data.name() == name && data.addr() == &addr && data.score() == 0 && data.is_connected())
}


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
        use std::iter::{from_fn, once};

        let trail_len = usize::arbitrary(g) % (MAX_PLAYER_NAME_LEN - 1);
        let mut res: String = once(char::from_u32(u32::arbitrary(g) % (0x7F - 0x21) + 0x21).unwrap())
            .chain(from_fn(|| char::from_u32(u32::arbitrary(g) % (0x7F - 0x20) + 0x20)).take(trail_len))
            .collect();
        while res.ends_with(" ") {
            res.pop();
        }
        Self(res)
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        let res = self
            .0
            .shrink()
            .filter(|n| name_is_valid(&n))
            .map(Self);
        Box::new(res)
    }
}


#[quickcheck]
fn name_gen(name: Name) -> bool {
    name_is_valid(&name.0)
}


/// Check whether a given player name is valid
///
pub fn name_is_valid(name: &str) -> bool {
    name.chars().all(|c| c.is_ascii() && !c.is_ascii_control()) &&
        !name.starts_with(" ") &&
        !name.ends_with(" ") &&
        name.len() > 0 &&
        name.len() <= MAX_PLAYER_NAME_LEN
}

