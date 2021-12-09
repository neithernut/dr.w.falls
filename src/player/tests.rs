//! Player tests

use quickcheck::{Arbitrary, Gen, TestResult};

use super::*;


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

