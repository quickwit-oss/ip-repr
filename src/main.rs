use ip_repr::{IntervalEncoding, IpRepr};
use itertools::Itertools;
use std::{
    collections::HashSet,
    io::{self, BufRead},
    net::{IpAddr, Ipv6Addr},
    str::FromStr,
};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct Opt {
}

fn ip_dataset() -> Vec<u128> {
    let mut ip_addr_v4 = 0;

    let stdin = io::stdin();
    let ip_addrs: Vec<u128> = stdin
        .lock()
        .lines()
        .flat_map(|line| {
            let line = line.unwrap();
            let line = line.trim();
            let ip_addr = IpAddr::from_str(line.trim()).ok()?;
            if ip_addr.is_ipv4() {
                ip_addr_v4 += 1;
            }
            let ip_addr_v6: Ipv6Addr = match ip_addr {
                IpAddr::V4(v4) => v4.to_ipv6_mapped(),
                IpAddr::V6(v6) => v6,
            };
            Some(ip_addr_v6)
        })
        .map(|ip_v6| u128::from_be_bytes(ip_v6.octets()))
        .collect();
    println!("IpAddrsAny\t{}", ip_addrs.len());
    println!("IpAddrsV4\t{}", ip_addr_v4);
    ip_addrs
}

fn print_set_stats(ip_addrs: &[u128]) {
    println!("NumIps\t{}", ip_addrs.len());
    let ip_addr_set: HashSet<u128> = ip_addrs.iter().cloned().collect();
    println!("NumUniqueIps\t{}", ip_addr_set.len());
    let ratio_unique = ip_addr_set.len() as f64 / ip_addrs.len() as f64;
    println!("RatioUniqueOverTotal\t{ratio_unique:.4}");

    // histogram
    let mut ip_addrs = ip_addrs.to_vec();
    ip_addrs.sort();
    let mut cnts: Vec<usize> = ip_addrs
        .into_iter()
        .dedup_with_count()
        .map(|(cnt, _)| cnt)
        .collect();
    cnts.sort();

    let top_256_cnt: usize = cnts.iter().rev().take(256).sum();
    let top_128_cnt: usize = cnts.iter().rev().take(128).sum();
    let total: usize = cnts.iter().sum();

    println!("{}", total);
    println!("{}", top_256_cnt);
    println!("{}", top_128_cnt);
    println!("Percentage Top128 {:02}", top_128_cnt as f32 / total as f32);
    println!("Percentage Top256 {:02}", top_256_cnt as f32 / total as f32);

    let mut cnts: Vec<(usize, usize)> = cnts.into_iter().dedup_with_count().collect();
    cnts.sort_by(|a, b| {
        if a.1 == b.1 {
            a.0.cmp(&b.0)
        } else {
            b.1.cmp(&a.1)
        }
    });
    println!("\n\n----\nIP Address histogram");
    println!("IPAddrCount\tFrequency");
    for (ip_addr_count, times) in cnts {
        println!("{}\t{}", ip_addr_count, times);
    }
}

fn main() {
    let args = Opt::from_args();
    let ip_addrs = ip_dataset();
    print_set_stats(&ip_addrs);
    let encoders: Vec<Box<dyn IpRepr>> = (0..16)
        .map(|num_bytes_per_intervals| {
            Box::new(IntervalEncoding(8 * num_bytes_per_intervals)) as Box<dyn IpRepr>
        })
        .collect();

    for encoder in encoders {
        println!("\n\n-----");
        println!("{:?}", encoder);
        let encoded = encoder.encode(&ip_addrs);
        let decoded = encoder.decode(&encoded);
        assert_eq!(&decoded, &ip_addrs);
        let num_bytes = encoded.len();
        println!("num_bytes\t{num_bytes:.2}");
        let bits_per_el = (8 * num_bytes) as f64 / ip_addrs.len() as f64;
        println!("bits_per_el\t{:.2}", bits_per_el);
    }
    // let encoding = IntervalEncoding;
    // let compress = ip_repr::IntervalEncoding.encode(ip_addrs)
}
