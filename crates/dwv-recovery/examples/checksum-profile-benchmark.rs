//! Small reproducible benchmark for the provisional checksum profile.
//!
//! This intentionally uses the semantic provider seam. It is evidence for
//! tuning only; it does not promote an extent size or provider into a format
//! compatibility decision.

use dwv_recovery::{Blake3Provider, DigestProvider};
use std::hint::black_box;
use std::time::Instant;

fn main() {
    let provider = Blake3Provider;
    println!("size_bytes,iterations,elapsed_seconds,throughput_mib_per_second");

    for (size, iterations) in [(1 << 20, 32), (4 << 20, 16), (16 << 20, 4)] {
        let bytes = vec![0xa5; size];
        let start = Instant::now();
        let sink = (0..iterations)
            .map(|_| {
                provider
                    .digest(black_box(&bytes))
                    .expect("BLAKE3 is supported")
            })
            .next_back()
            .expect("benchmark has at least one iteration");
        black_box(sink);
        let elapsed = start.elapsed().as_secs_f64();
        let mib = (size * iterations) as f64 / (1024.0 * 1024.0);
        let throughput = mib / elapsed.max(f64::MIN_POSITIVE);
        println!("{size},{iterations},{elapsed:.6},{throughput:.2}");
    }
}
