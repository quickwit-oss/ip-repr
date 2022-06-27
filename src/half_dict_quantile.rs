use std::{fmt::Debug, time::Instant};

use fnv::FnvHashMap;

use crate::{get_most_common, IPWithCount, IpRepr};

#[derive(Debug)]
pub struct HalfDictQ {}

impl HalfDictQ {
    pub fn new() -> HalfDictQ {
        HalfDictQ {}
    }
}

struct HalfDictCompressorQ {
    top_ips_ordered: Vec<IPWithCount>,
    //ip_to_ordinal: FnvHashMap<u128, u128>,
    remapped_ip_addr: Vec<u128>,
}

impl HalfDictCompressorQ {
    fn compress(&self, _ip_addrs: &[u128]) -> Vec<u8> {
        let bytes: Vec<u8> = q_compress::auto_compress(&self.remapped_ip_addr, 10);

        let storing_dict = self.top_ips_ordered.len() * 16;
        println!("bytes len: {}", bytes.len() + storing_dict);

        bytes
    }
}

impl HalfDictQ {
    fn train(&self, ip_addrs: &[u128]) -> HalfDictCompressorQ {
        let num_most_common = 4096;
        let top_ips_ordered = get_most_common(ip_addrs, num_most_common);
        let ip_to_ordinal: FnvHashMap<u128, u128> = top_ips_ordered
            .iter()
            .enumerate()
            .map(|(ord, entry)| (entry.ip, ord as u128))
            .collect();

        let remapped_ip_addr = ip_addrs
            .iter()
            .map(|ip| {
                ip_to_ordinal
                    .get(ip)
                    .cloned()
                    .unwrap_or(ip + num_most_common as u128)
            })
            .collect();

        HalfDictCompressorQ {
            top_ips_ordered,
            //ip_to_ordinal,
            remapped_ip_addr,
        }
    }
}

impl IpRepr for HalfDictQ {
    fn encode(&self, ip_addrs: &[u128]) -> Vec<u8> {
        if ip_addrs.is_empty() {
            return Vec::new();
        }
        let start = Instant::now();
        let compressor = self.train(ip_addrs);
        println!("Train Time: {}ms", (Instant::now() - start).as_millis());
        let start = Instant::now();
        let compressed = compressor.compress(ip_addrs);
        println!("Compress Time: {}ms", (Instant::now() - start).as_millis());
        //println!(
        //"Elems / s: {}",
        //ip_addrs.len() as f32 / ((Instant::now() - start).as_millis() as f32 / 1000.0)
        //);

        compressed
    }

    fn decode(&self, _data: &[u8]) -> Vec<u128> {
        unimplemented!()
    }
}
