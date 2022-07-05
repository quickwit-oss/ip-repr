use std::{fmt::Debug, time::Instant};

use fnv::FnvHashMap;

use crate::{get_most_common, IPWithCount, IpRepr};

#[derive(Debug)]
pub struct HalfDictQ {
    num_most_common: usize,
}

impl HalfDictQ {
    pub fn new(num_most_common: usize) -> HalfDictQ {
        HalfDictQ { num_most_common }
    }
}

struct HalfDictCompressorQ {
    top_ips_ordered: Vec<IPWithCount>,
    remapped_ip_addr: Vec<u128>,
}

impl HalfDictCompressorQ {
    fn compress(&self, _ip_addrs: &[u128]) -> Vec<u8> {
        let bytes: Vec<u8> = q_compress::auto_compress(&self.remapped_ip_addr, 10);

        let storing_dict = self.top_ips_ordered.len() * 16;
        let num_compressed_bytes = bytes.len() + storing_dict;
        println!(
            "HalfDictQ: TopNRemapped:{} - compressed len: {}",
            self.top_ips_ordered.len(),
            num_compressed_bytes
        );
        println!(
            "Compression: {:.2}%",
            100.0 * num_compressed_bytes as f64 / (_ip_addrs.len() as f64 * 16.0)
        );

        bytes
    }
}

impl HalfDictQ {
    fn train(&self, ip_addrs: &[u128]) -> HalfDictCompressorQ {
        let num_most_common = self.num_most_common;
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
