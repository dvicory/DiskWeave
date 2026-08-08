# OS-011 provisional checksum benchmark

This is tuning evidence for V-009, not a stable format decision. The benchmark
calls the portable `DigestProvider` seam with the default-feature `blake3`
provider and hashes buffers filled with `0xa5`.

Command:

```text
cargo run -p dwv-recovery --example checksum-profile-benchmark --release
```

Environment: macOS arm64, 2026-08-07. One local run produced:

| Buffer | Iterations | Elapsed | Throughput |
|---:|---:|---:|---:|
| 1 MiB | 32 | 0.020096 s | 1592.33 MiB/s |
| 4 MiB | 16 | 0.038963 s | 1642.58 MiB/s |
| 16 MiB | 4 | 0.036663 s | 1745.61 MiB/s |

The result supports retaining the handoff’s provisional 4 MiB extent and
32-byte digest as the initial implementation profile. It does not select
worker concurrency, prove repair cost, measure metadata overhead, or establish
cross-platform performance; those remain open to later V-009 evidence. The
profile and checksum-set generation seams therefore remain explicit.

Provider references: [`blake3` Rust crate documentation](https://docs.rs/blake3/latest/blake3/)
and the [official BLAKE3 repository](https://github.com/BLAKE3-team/BLAKE3).
