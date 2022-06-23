#[derive(Debug, StructOpt)]
struct Opt {
    #[structopt(short, long)]
    filepath: PathBuf,
}

fn ip_dataset(filepath: &Path) -> Vec<u128> {
    let mut ip_addr_v4 = 0;
    let data = std::fs::read_to_string(filepath).unwrap();
    let ip_addrs: Vec<u128> = data.lines()
        .flat_map(|line| {
                let line = line.trim();
                let ip_addr = IpAddr::from_str(line.trim()).ok()?;
                if ip_addr.is_ipv4() {
                    ip_addr_v4 += 1;
                }
                let ip_addr_v6: Ipv6Addr =
                    match ip_addr {
                        IpAddr::V4(v4) => v4.to_ipv6_mapped(),
                        IpAddr::V6(v6) => v6,
                    };
                Some(ip_addr_v6)
        })
        .map(|ip_v6| u128::from_be_bytes(ip_v6.octets()))
        .take(1_000_000)
        .collect();
    println!("IpAddrsAny\t{}", ip_addrs.len());
    println!("IpAddrsV4\t{}", ip_addr_v4);
    ip_addrs
}

use std::{path::{Path, PathBuf}, net::{IpAddr, Ipv6Addr}, str::FromStr, collections::HashSet};
use structopt::StructOpt;
use ip_repr::{IpRepr, IntervalEncoding};

fn print_set_stats(ip_addrs: &[u128]) {
    println!("NumIps\t{}", ip_addrs.len());
    let ip_addr_set: HashSet<u128> = ip_addrs.iter().cloned().collect();
    println!("NumUniqueIps\t{}", ip_addr_set.len());
    let ratio_unique = ip_addr_set.len() as f64 / ip_addrs.len() as f64;
    println!("RatioUniqueOverTotal\t{ratio_unique:.4}");
}

fn main() {
    let args = Opt::from_args();
    let ip_addrs = ip_dataset(&args.filepath);
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
