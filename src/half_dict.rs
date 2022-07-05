use std::{fmt::Debug, time::Instant};

use fnv::FnvHashMap;
use itertools::Itertools;

use crate::{
    get_most_common,
    interval::{train, IntervalCompressor},
    IpRepr,
};

//struct Block {
//// The bit_mask that marks if an element is coming from the dictionary
//bit_mask: BlockBitMask,
//// the non-dictionary data, compressed as vint in compressed space
//residual_data: Vec<u8>,
//}

/// Bit mask of size 1024 (0-1023)
#[derive(Default)]
struct BlockBitMask {
    // The bit_mask that marks if an element is coming from the dictionary
    bit_mask: [u64; 16], // 1024 bits
}

impl Debug for BlockBitMask {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bit_mask = self
            .bit_mask
            .iter()
            .map(|bit_mask| format!("{:032b}", bit_mask))
            .join("|");

        f.debug_struct("BlockBitMask")
            .field("bit_mask", &bit_mask)
            .finish()
    }
}

impl BlockBitMask {
    fn set(&mut self, pos: usize) {
        let byte = pos / 64;
        let bit = pos % 64;
        self.bit_mask[byte] |= 1 << bit;
    }
}

#[test]
fn bit_mask_test() {
    let mut bit_mask = BlockBitMask::default();

    bit_mask.set(0);
    bit_mask.set(1023);
    bit_mask.set(960);
    dbg!(bit_mask);
}

#[derive(Debug)]
pub struct HalfDict {
    block_size: usize,
    num_bits_for_most_common: usize,
}

impl HalfDict {
    pub fn new(block_size: usize, num_bits_for_most_common: usize) -> HalfDict {
        HalfDict {
            block_size,
            num_bits_for_most_common,
        }
    }
}

struct HalfDictCompressor {
    interval_compressor: IntervalCompressor,
    //top_ips_ordered: Vec<IPWithCount>,
    ip_to_ordinal: FnvHashMap<u128, u16>,
    block_size: usize,
    num_bits_for_most_common: usize,
}

struct BlockMetaData {
    pub num_bits_for_dict_encoded: usize,
    block_size: usize,
    num_bits_other: u8,
    num_dict_encoded: usize,
}

impl BlockMetaData {
    fn get_num_bits(&self) -> usize {
        let dict_enc_bits = self.num_dict_encoded * self.num_bits_for_dict_encoded;
        let other_enc_size =
            (self.block_size - self.num_dict_encoded) * self.num_bits_other as usize;
        let bit_mask_num_bits = self.block_size;
        dict_enc_bits + other_enc_size + bit_mask_num_bits
    }
    fn _num_bits_per_elem(&self) -> f32 {
        self.get_num_bits() as f32 / self.block_size as f32
    }
}

impl HalfDictCompressor {
    fn compress(&self, ip_addrs: &[u128]) -> Vec<u8> {
        let mut _num_blocks = ip_addrs.len() / self.block_size;
        if ip_addrs.len() % self.block_size != 0 {
            _num_blocks += 1;
        }
        let iter = ip_addrs.chunks_exact(self.block_size);

        let mut block_metadata = vec![];

        for chunk in iter {
            //let mut chunk = chunk.to_vec();
            let mut dict_enc = vec![];
            let mut residual_data = vec![];
            let mut bit_mask = BlockBitMask::default();

            for (pos, el) in chunk.iter().enumerate() {
                if let Some(ord) = self.ip_to_ordinal.get(el) {
                    dict_enc.push(ord);
                    bit_mask.set(pos);
                } else {
                    residual_data.push(el);
                }
            }

            block_metadata.push(BlockMetaData {
                num_dict_encoded: dict_enc.len(),
                num_bits_for_dict_encoded: self.num_bits_for_most_common,
                block_size: self.block_size,
                num_bits_other: self.interval_compressor.num_bits,
            });
            //let mut dict_entries = vec![];
        }
        // TODO handle chunks remainder

        let num_compressed_bytes: usize = block_metadata
            .iter()
            .map(|block| block.get_num_bits())
            .sum::<usize>()
            / 8;

        println!(
            "HalfDict + Compressed Space: compressed len: {}",
            num_compressed_bytes
        );

        println!(
            "Compression: {:.2}%",
            100.0 * num_compressed_bytes as f64 / (ip_addrs.len() as f64 * 16.0)
        );

        vec![]
    }
}

impl HalfDict {
    fn train(&self, ip_addrs: &[u128]) -> HalfDictCompressor {
        let mut ip_addrs_sorted = ip_addrs.to_vec();
        ip_addrs_sorted.sort();
        let interval_compressor = train(&ip_addrs_sorted, 64);

        let top_ips_ordered = get_most_common(ip_addrs, 1 << self.num_bits_for_most_common);

        let ip_to_ordinal: FnvHashMap<u128, u16> = top_ips_ordered
            .iter()
            .enumerate()
            .map(|(ord, entry)| (entry.ip, ord as u16))
            .collect();

        HalfDictCompressor {
            interval_compressor,
            //top_ips_ordered,
            ip_to_ordinal,
            block_size: self.block_size,
            num_bits_for_most_common: self.num_bits_for_most_common,
        }
    }
}

impl IpRepr for HalfDict {
    fn encode(&self, ip_addrs: &[u128]) -> Vec<u8> {
        if ip_addrs.is_empty() {
            return Vec::new();
        }

        let start = Instant::now();
        let compressor = self.train(ip_addrs);
        println!("Train Time: {}", (Instant::now() - start).as_millis());
        let start = Instant::now();
        let compressed = compressor.compress(ip_addrs);
        println!("Estimate Time: {}", (Instant::now() - start).as_millis());
        compressed
    }

    fn decode(&self, _data: &[u8]) -> Vec<u128> {
        unimplemented!()
    }
}
