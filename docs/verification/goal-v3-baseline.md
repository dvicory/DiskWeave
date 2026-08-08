# Goal-v3 portable demo baseline

## Run

Date: 2026-08-08

Disposable fixture: `/tmp/dwv-v3.7j71T0`

Commands, in order:

```text
cargo run -q --bin dwv -- demo init --root /tmp/dwv-v3.7j71T0
cargo run -q --bin dwv -- demo run --root /tmp/dwv-v3.7j71T0
cargo run -q --bin dwv -- demo status --root /tmp/dwv-v3.7j71T0
cargo run -q --bin dwv -- demo inspect --root /tmp/dwv-v3.7j71T0
cargo run -q --bin dwv -- demo capabilities --root /tmp/dwv-v3.7j71T0
cargo run -q --bin dwv -- demo verify --root /tmp/dwv-v3.7j71T0
cargo run -q --bin dwv -- demo trace-export --root /tmp/dwv-v3.7j71T0 --trace baseline-trace.json
cargo run -q --bin dwv -- demo trace-render --root /tmp/dwv-v3.7j71T0 --trace baseline-trace.json
cargo run -q --bin dwv -- demo trace-replay --root /tmp/dwv-v3.7j71T0 --trace baseline-trace.json
```

## Observed results

- `demo.init`: success; 512-byte logical blocks, 16 KiB protected length,
  redundant parity-envelope profile, regular data/parity/recovery files.
- `demo.run`: complete; healthy write/read/flush/reopen matched, offline
  degraded read matched the reference, four rebuild chunks verified, final
  verification passed, and source parity was preserved.
- `demo.status`: healthy recovery state; one rebuild, recovery generation 9,
  topology epoch 16.
- `demo.inspect`: matching redundant envelope copies; envelope profile
  comparison accepted bare, redundant-envelope, and envelope-plus-bitmap
  candidates; no required data-payload metadata.
- `demo.capabilities`: file-backed portable-demo boundary; host flush fence
  was probed, while physical block size, torn-write model, volatile cache,
  cancellation, FUA, and physical durability remain unknown or unsupported.
- `demo.verify`: exhaustive equation scan clean, reference matched, zero
  payload writes.
- `demo.trace-export`: schema 1, 11 events.
- `demo.trace-render`: 11 normalized events covering open, write, flush, read,
  recovery intent/checkpoint, checksum, degraded read, rebuild, repair, and
  terminal outcome.
- `demo.trace-replay`: simulator and copied regular-file runs were equivalent;
  payload and parity digests matched.

## Claim boundary

This baseline proves the disposable portable simulator/file-backed workflow.
It does not prove Linux ublk or io_uring behavior, a live macOS bridge,
physical power-loss durability, P/Q, degraded writes, online rebuild, or
production scheduling.
