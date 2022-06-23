use std::collections::{BTreeMap, BinaryHeap};

use crate::IpRepr;
use tantivy_bitpacker::{self, BitPacker, BitUnpacker};

#[derive(Default, Debug)]
pub struct IntervalEncoding(pub usize);

// const COST_IN_BITS: usize = (16 - 2) * 8; // Cost in bits of one

struct Compressor {
    ip_addr_to_compact: BTreeMap<u128, u64>,
    num_bits: u8,
}

const STOP_BIT: u8 = 128u8;

 fn serialize_vint(mut val: u128, output: &mut Vec<u8>)  {
    loop {
        let next_byte: u8 = (val % 128u128) as u8;
        val /= 128u128;
        if val == 0 {
            output.push(next_byte | STOP_BIT);
            return;
        } else {
            output.push(next_byte);
        }
    }
}

fn deserialize_vint(data: &[u8]) -> (u128, &[u8]) {
    let mut result = 0u128;
    let mut shift = 0u64;
    for i in 0..19 {
        let b = data[i];
        result |= u128::from(b % 128u8) << shift;
        if b >= STOP_BIT {
            return (result, &data[i+1..])
        }
        shift += 7;
    }
    panic!("invalid data");
}

fn train(ip_addrs_sorted: &[u128], cost_in_bits: usize) -> Compressor {
    let mut prev_opt = None;
    let mut deltas: BinaryHeap<(u128, usize)> = BinaryHeap::new();
    for (pos, ip_addr) in ip_addrs_sorted.iter().cloned().enumerate() {
        let delta =
             if let Some(prev) = prev_opt {
                ip_addr - prev
             } else {
                ip_addr + 1
             };
        deltas.push((delta, pos));
        prev_opt = Some(ip_addr);
    }
    let mut amplitude = *ip_addrs_sorted.last().unwrap() + 1;
    let mut amplitude_bits: f64 = (amplitude as f64).log2();
    let mut blanks = Vec::new();
    while let Some((delta, pos)) = deltas.pop() {
        let next_amplitude = amplitude - delta + 1;
        let next_amplitude_bits = (next_amplitude as f64).log2();
        let gained_bits =
            ((amplitude_bits - next_amplitude_bits) * ip_addrs_sorted.len() as f64) as usize;
        if cost_in_bits >= gained_bits {
            break;
        }
        amplitude = next_amplitude;
        amplitude_bits = next_amplitude_bits;
        blanks.push(pos);
    }
    blanks.sort();
    assert!(
        amplitude <= u64::MAX as u128,
        "case unsupported for this test program."
    );
    let mut offset = 0;
    let mut ip_addr_to_compact = BTreeMap::new();
    let mut prev_base = 0;
    for pos in blanks {
        let ip_addr = ip_addrs_sorted[pos];
        if pos == 0 {
            ip_addr_to_compact.insert(ip_addr, offset as u64);
            prev_base = ip_addr;
        } else {
            offset += ip_addrs_sorted[pos - 1] - prev_base + 1;
            ip_addr_to_compact.insert(ip_addr, offset as u64);
            prev_base = ip_addr;
        }
    }
    let num_bits = tantivy_bitpacker::compute_num_bits(amplitude as u64);
    let compressor = Compressor {
        ip_addr_to_compact,
        num_bits,
    };
    assert_eq!(compressor.to_compact(*ip_addrs_sorted.last().unwrap()) + 1, amplitude as u64);
    compressor
}

impl Compressor {


    fn to_compact(&self, ip_addr: u128) -> u64 {
        if let Some((&ip_addr_base, &compact_base)) = self.ip_addr_to_compact.range(..=ip_addr).last() {
            compact_base + (ip_addr - ip_addr_base) as u64
        } else {
            ip_addr as u64
        }
    }

    fn write_header(&self, output: &mut Vec<u8>) {
        assert!(output.is_empty());
        serialize_vint(self.ip_addr_to_compact.len() as u128, output);
        let mut prev_ip = 0;
        let mut prev_compact = 0;
        for (&ip, &compact) in &self.ip_addr_to_compact {
            let delta_ip = ip - prev_ip;
            let delta_compact = compact - prev_compact;
            serialize_vint(delta_ip as u128, output);
            serialize_vint(delta_compact as u128, output);
            prev_ip = ip;
            prev_compact = compact;
        }
        output.push(self.num_bits);
        println!("NumIntervals\t{}", self.ip_addr_to_compact.len());
        println!("HeaderLen\t{}", output.len());
        let bits_per_interval = output.len() as f64 / self.ip_addr_to_compact.len() as f64;
        println!("BitsPerInterval\t{}", bits_per_interval);
    }

    pub fn compress(&self, ip_addrs: &[u128]) -> Vec<u8> {
        let mut output: Vec<u8> = Vec::new();
        self.write_header(&mut output);
        serialize_vint(ip_addrs.len() as u128, &mut output);
        let mut bitpacker = BitPacker::default();
        for &ip_addr in ip_addrs {
            let compact = self.to_compact(ip_addr);
            bitpacker
                .write(compact, self.num_bits, &mut output)
                .unwrap();
        }
        bitpacker.close(&mut output).unwrap();
        output
    }
}


struct Decompressor {
    compact_to_ip_addrs: BTreeMap<u64, u128>,
    bit_unpacker: BitUnpacker
}

impl Decompressor {
    fn open(mut data: &[u8]) -> (Decompressor, &[u8]) {
        let (num_ip_addrs, new_data) = deserialize_vint(data);
        data = new_data;
        let mut ip_addr = 0u128;
        let mut compact = 0u64;
        let mut compact_to_ip_addrs: BTreeMap<u64, u128> = Default::default();
        for _ in 0..num_ip_addrs {
            let (ip_addr_delta, new_data) = deserialize_vint(data);
            data = new_data;
            let (compact_delta, new_data) = deserialize_vint(data);
            data = new_data;
            ip_addr += ip_addr_delta;
            compact += compact_delta as u64;
            compact_to_ip_addrs.insert(compact, ip_addr);
        }
        let num_bits = data[0];
        data = &data[1..];
        let decompressor = Decompressor {
            compact_to_ip_addrs,
            bit_unpacker: BitUnpacker::new(num_bits)
        };
        (decompressor, data)
    }

    fn compact_to_ip_addr(&self, compact: u64) -> u128 {
        if let Some((&compact_base, &ip_base)) = self.compact_to_ip_addrs.range(..=compact).last() {
            ip_base + (compact - compact_base) as u128
        } else {
            compact as u128
        }
    }

    pub fn get(&self, idx: usize, data: &[u8]) -> u128 {
        let base = self.bit_unpacker.get(idx as u64, data);
        self.compact_to_ip_addr(base)
    }
}

impl IntervalEncoding {
    fn train(&self, ip_addrs: &[u128]) -> Compressor {
        let mut ip_addrs_sorted = ip_addrs.to_vec();
        ip_addrs_sorted.sort();
        train(&ip_addrs_sorted, self.0)
    }
}

impl IpRepr for IntervalEncoding {
    fn encode(&self, ip_addrs: &[u128]) -> Vec<u8> {
        if ip_addrs.is_empty() {
            return Vec::new();
        }
        let compressor = self.train(ip_addrs);
        compressor.compress(ip_addrs)
    }

    fn decode(&self, data: &[u8]) -> Vec<u128> {
        let (decompressor, data) = Decompressor::open(data);
        let (num_vals, data) = deserialize_vint(data);
        let mut ip_addrs = Vec::new();
        for idx in 0..num_vals as usize {
            let ip_addr = decompressor.get(idx, data);
            ip_addrs.push(ip_addr);
        }
        ip_addrs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_aux_vals<T: IpRepr>(ip_repr: &T, ip_addrs: &[u128]) {
        let data = ip_repr.encode(ip_addrs);
        let decoded_ip_addrs = ip_repr.decode(&data);
        assert_eq!(&decoded_ip_addrs, ip_addrs);
    }

    #[test]
    fn test_compress() {
        let ip_addrs = &[1u128, 100u128, 3u128, 99999u128, 100000u128, 100001u128, 4_000_211_221u128, 4_000_211_222u128, 333u128];
        let interval_encoding = IntervalEncoding::default();
        test_aux_vals(&interval_encoding, ip_addrs)
    }

    #[test]
    fn test_first_large_gaps() {
        let ip_addrs = &[1_000_000_000u128; 100];
        let interval_encoding = IntervalEncoding::default();
        test_aux_vals(&interval_encoding, ip_addrs)
    }
}
