mod interval;

pub use interval::IntervalEncoding;
use std::fmt::Debug;

pub trait IpRepr: Debug {
    fn encode(&self, ip_addrs: &[u128]) -> Vec<u8>;
    fn decode(&self, ip_addrs: &[u8]) -> Vec<u128>;
}
