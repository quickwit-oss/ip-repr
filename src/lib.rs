mod half_dict;
mod half_dict_quantile;
mod interval;

use fnv::FnvHashMap;
pub use half_dict::HalfDict;
pub use half_dict_quantile::HalfDictQ;
pub use interval::IntervalEncoding;
use std::{collections::BinaryHeap, fmt::Debug};

pub trait IpRepr: Debug {
    fn encode(&self, ip_addrs: &[u128]) -> Vec<u8>;
    fn decode(&self, ip_addrs: &[u8]) -> Vec<u128>;
}

#[derive(Debug)]
struct IPWithCount {
    ip: u128,
    count: usize,
}

impl PartialEq for IPWithCount {
    fn eq(&self, other: &Self) -> bool {
        self.count == other.count
    }
}
impl Eq for IPWithCount {}

impl PartialOrd for IPWithCount {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        other.count.partial_cmp(&self.count)
    }
}
impl Ord for IPWithCount {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.count.cmp(&self.count)
    }
}

fn get_most_common(ip_addrs: &[u128], top_n: usize) -> Vec<IPWithCount> {
    let cnts = ip_addrs.iter().fold(
        FnvHashMap::<u128, usize>::with_capacity_and_hasher(
            ip_addrs.len() / 10,
            Default::default(),
        ),
        |mut acc, ip| {
            let entry = acc.entry(*ip).or_default();
            *entry += 1;
            acc
        },
    );

    let top_ips = cnts
        .iter()
        .fold(BinaryHeap::<IPWithCount>::default(), |mut heap, entry| {
            let ip_with_count = IPWithCount {
                ip: *entry.0,
                count: *entry.1,
            };
            if heap.len() < top_n {
                heap.push(ip_with_count);
                return heap;
            }
            let entry = heap.peek().unwrap();
            if entry > &ip_with_count {
                heap.pop();
                heap.push(ip_with_count);
            }

            heap
        });
    let top_ips_ordered = top_ips.into_sorted_vec();
    top_ips_ordered
}
