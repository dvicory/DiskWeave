#!/usr/bin/env bash
set -euo pipefail

source_hash=${1:?source snapshot hash required}
source_root=${2:?source root required}
results_root=${3:?results root required}
work=$(mktemp -d)
fixture=$work/fixture
mountpoint=$work/mount
scratch=$work/scratch
server_pid=

cleanup() {
  mountpoint -q "$mountpoint" && umount "$mountpoint" || true
  if [[ -n "$server_pid" ]] && kill -0 "$server_pid" 2>/dev/null; then
    kill -TERM "$server_pid"
    wait "$server_pid" || true
  fi
  rm -rf "$work"
}
trap cleanup EXIT

wait_ready() {
  for _ in $(seq 1 300); do
    [[ -f "$fixture/ready.json" ]] && return
    kill -0 "$server_pid"
    sleep 0.1
  done
  return 1
}

cd "$source_root"
mkdir -p "$mountpoint" "$results_root" "$scratch"
live_started=$SECONDS
./target/release/dwv demo disk probe > "$scratch/dwv-probe.json"
./target/release/dwv demo disk init --root "$fixture" --size 16777216 > "$scratch/dwv-init.json"
jq '
  def hex:
    [ .[] | . as $n |
      "0123456789abcdef"[($n / 16 | floor):(($n / 16 | floor) + 1)] +
      "0123456789abcdef"[($n % 16):(($n % 16) + 1)]
    ] | join("");
  {
    schema: "dwv.array-policy.v1",
    array_id: (.array_id | hex),
    topology_epoch,
    protected_length,
    logical_block_size,
    recovery: {path: .recovery_file},
    members: [
      {
        path: .data_files[0],
        role: "data",
        expected_identity: (.data_identities[0] | hex),
        slot_id: "01010101010101010101010101010101",
        coding_position: 0,
        assignment_instance: "0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b0b",
        assignment_generation: 1,
        store_id: 10
      },
      {
        path: .parity_file,
        role: "parity",
        expected_identity: (.parity_identity | hex),
        slot_id: "02020202020202020202020202020202",
        coding_position: 1,
        assignment_instance: "0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c0c",
        assignment_generation: 1,
        store_id: 20
      }
    ],
    frontend: "linux-ublk"
  }
' "$fixture/fixture.json" > "$fixture/array.json"
./target/release/dwv status --array "$fixture/array.json" --json > "$scratch/dwv-production-status.json"
cp "$fixture/fixture.json" "$scratch/dwv-production-fixture.json"
jq '.array_id = [66,66,66,66,66,66,66,66,66,66,66,66,66,66,66,66]' \
  "$scratch/dwv-production-fixture.json" > "$fixture/fixture.json"
./target/release/dwv start --array "$fixture/array.json" --json \
  > "$scratch/dwv-production-start.json" 2> "$scratch/dwv-production-start-error.json" &
server_pid=$!
published=
for _ in $(seq 1 300); do
  if [[ -b /dev/ublkb0 ]]; then
    published=true
    break
  fi
  kill -0 "$server_pid"
  sleep 0.1
done
if [[ -z "$published" ]]; then
  cat "$scratch/dwv-production-status.json" "$scratch/dwv-production-start.json" \
    "$scratch/dwv-production-start-error.json" >&2
  exit 1
fi
kill -TERM "$server_pid"
wait "$server_pid"
server_pid=
[[ ! -b /dev/ublkb0 ]]
mv "$scratch/dwv-production-fixture.json" "$fixture/fixture.json"
jq -e '
  .schema == "dwv.operator.v1" and
  .command == "start" and
  .outcome == "success" and
  .reason_code == "frontend-published" and
  .publication.status == "published" and
  .publication.device_path == "/dev/ublkb0"
' "$scratch/dwv-production-start.json" >/dev/null
printf 'production publication: %ss\n' "$((SECONDS - live_started))" >&2

./target/release/dwv demo disk serve --root "$fixture" > "$scratch/dwv-serve-first.json" &
server_pid=$!
wait_ready
device=$(jq -r .device_path "$fixture/ready.json")
device_id=$(jq -r .device_id "$fixture/ready.json")
if ./target/release/dwv demo disk serve --root "$fixture" 2> "$scratch/dwv-ownership-conflict.json"; then
  echo "second server unexpectedly acquired the fixture" >&2
  exit 1
fi
cleanup_fixture=$work/cleanup-conflict
./target/release/dwv demo disk init --root "$cleanup_fixture" --size 16777216 > "$scratch/dwv-cleanup-conflict-init.json"
if ./target/release/dwv demo disk cleanup --root "$cleanup_fixture" --device-id "$device_id" 2> "$scratch/dwv-conflicting-cleanup.json"; then
  echo "cleanup unexpectedly removed a differently owned endpoint" >&2
  exit 1
fi
[[ -b "$device" ]]
pre_discard=$(sha256sum "$fixture/data.raw" "$fixture/parity.raw")
if blkdiscard "$device" 2> "$scratch/dwv-unsupported-discard.txt"; then
  echo "discard unexpectedly succeeded" >&2
  exit 1
fi
[[ "$pre_discard" == "$(sha256sum "$fixture/data.raw" "$fixture/parity.raw")" ]]
mkfs.ext4 -F -E nodiscard "$device"
mount -o nodiscard "$device" "$mountpoint"
printf initial-content > "$mountpoint/rename.tmp"
sync "$mountpoint/rename.tmp"
printf final-diskweave-content > "$mountpoint/rename.tmp"
sync "$mountpoint/rename.tmp"
mv "$mountpoint/rename.tmp" "$mountpoint/durable.txt"
sync "$mountpoint"
printf transient > "$mountpoint/delete.me"
sync "$mountpoint/delete.me"
rm "$mountpoint/delete.me"
sync "$mountpoint"
first_hash=$(sha256sum "$mountpoint/durable.txt" | cut -d' ' -f1)
[[ "$first_hash" == d4ad659dcd887413e31f0b6d272b2b353d29734c3cba9f1cb9b74ab45865f4d7 ]]
[[ ! -e "$mountpoint/delete.me" ]]
umount "$mountpoint"
kill -TERM "$server_pid"
wait "$server_pid"
server_pid=
[[ ! -e "$fixture/ready.json" ]]
cp "$fixture/trace.json" "$scratch/dwv-trace-first.json"
printf 'first ext4 lifecycle: %ss\n' "$((SECONDS - live_started))" >&2

./target/release/dwv demo disk serve --root "$fixture" > "$scratch/dwv-serve-second.json" &
server_pid=$!
wait_ready
device=$(jq -r .device_path "$fixture/ready.json")
mount -o ro,noload "$device" "$mountpoint"
second_hash=$(sha256sum "$mountpoint/durable.txt" | cut -d' ' -f1)
[[ "$second_hash" == "$first_hash" ]]
[[ ! -e "$mountpoint/delete.me" ]]
umount "$mountpoint"
kill -TERM "$server_pid"
wait "$server_pid"
server_pid=
[[ ! -e "$fixture/ready.json" ]]
cp "$fixture/trace.json" "$scratch/dwv-trace-second.json"
./target/release/dwv demo disk trace-replay --trace "$scratch/dwv-trace-first.json" > "$scratch/dwv-trace-first-replay.json"
./target/release/dwv demo disk trace-replay --trace "$scratch/dwv-trace-second.json" > "$scratch/dwv-trace-second-replay.json"
first_trace_digest=$(sha256sum "$scratch/dwv-trace-first.json" | cut -d' ' -f1)
second_trace_digest=$(sha256sum "$scratch/dwv-trace-second.json" | cut -d' ' -f1)
jq -e \
  --arg digest "$(jq -r .fixture_digest "$scratch/dwv-serve-first.json")" \
  --argjson count "$(jq -r .trace_count "$scratch/dwv-serve-first.json")" \
  '.schema == "dwv.ublk.trace.v2" and .fixture_digest == $digest and
   .bounds.queue_depth == 8 and .bounds.maximum_transfer == 131072 and
   .bounds.maximum_records == 4096 and .exhausted_records == 0 and
   (.records | length) == $count' "$scratch/dwv-trace-first.json" >/dev/null
jq -e \
  --arg digest "$(jq -r .fixture_digest "$scratch/dwv-serve-second.json")" \
  --argjson count "$(jq -r .trace_count "$scratch/dwv-serve-second.json")" \
  '.schema == "dwv.ublk.trace.v2" and .fixture_digest == $digest and
   .bounds.queue_depth == 8 and .bounds.maximum_transfer == 131072 and
   .bounds.maximum_records == 4096 and .exhausted_records == 0 and
   (.records | length) == $count' "$scratch/dwv-trace-second.json" >/dev/null
[[ $(stat -c %s "$scratch/dwv-trace-first.json") -le 4194304 ]]
[[ $(stat -c %s "$scratch/dwv-trace-second.json") -le 4194304 ]]
printf 'second ext4 lifecycle and trace checks: %ss\n' "$((SECONDS - live_started))" >&2
printf '{}\n' > "$fixture/ready.json"
if ./target/release/dwv demo disk serve --root "$fixture" 2> "$scratch/dwv-stale-readiness.json"; then
  echo "serve unexpectedly accepted stale readiness state" >&2
  exit 1
fi
rm "$fixture/ready.json"
main_fixture=$fixture
fixture=$work/owner-death
./target/release/dwv demo disk init --root "$fixture" --size 16777216 > "$scratch/dwv-owner-death-init.json"
./target/release/dwv demo disk serve --root "$fixture" > "$scratch/dwv-owner-death-first.json" &
server_pid=$!
wait_ready
owner_device_id=$(jq -r .device_id "$fixture/ready.json")
kill -KILL "$server_pid"
wait "$server_pid" || true
server_pid=
if ./target/release/dwv demo disk serve --root "$fixture" 2> "$scratch/dwv-owner-death-reconciliation.json"; then
  echo "serve unexpectedly ignored stale publication after owner death" >&2
  exit 1
fi
./target/release/dwv demo disk cleanup --root "$fixture" --device-id "$owner_device_id" > "$scratch/dwv-owner-death-cleanup.json"
./target/release/dwv demo disk serve --root "$fixture" > "$scratch/dwv-owner-death-restart.json" &
server_pid=$!
wait_ready
kill -TERM "$server_pid"
wait "$server_pid"
server_pid=
fixture=$main_fixture
printf 'owner-death lifecycle: %ss\n' "$((SECONDS - live_started))" >&2

mount -o loop,ro,noload "$fixture/data.raw" "$mountpoint"
backing_hash=$(sha256sum "$mountpoint/durable.txt" | cut -d' ' -f1)
[[ "$backing_hash" == "$first_hash" ]]
[[ ! -e "$mountpoint/delete.me" ]]
umount "$mountpoint"
cmp "$fixture/data.raw" "$fixture/parity.raw"
data_hash=$(sha256sum "$fixture/data.raw" | cut -d' ' -f1)
parity_hash=$(sha256sum "$fixture/parity.raw" | cut -d' ' -f1)
./target/release/dwv demo disk inspect --root "$fixture" > "$scratch/dwv-inspect.json"
if ./target/release/dwv demo disk cleanup --root "$fixture" --device-id 2147483647 2> "$scratch/dwv-unknown-cleanup.json"; then
  echo "cleanup unexpectedly accepted an unknown endpoint" >&2
  exit 1
fi
printf 'final direct inspection: %ss\n' "$((SECONDS - live_started))" >&2
live_elapsed_seconds=$((SECONDS - live_started))
if ((live_elapsed_seconds >= 30)); then
  echo "live Linux disk acceptance reached the 30-second limit: ${live_elapsed_seconds}s" >&2
  exit 1
fi

jq -n \
  --arg source_snapshot_sha256 "$source_hash" \
  --arg kernel_release "$(uname -r)" \
  --arg architecture "$(uname -m)" \
  --argjson live_elapsed_seconds "$live_elapsed_seconds" \
  --arg content_sha256 "$first_hash" \
  --arg data_sha256 "$data_hash" \
  --arg parity_sha256 "$parity_hash" \
  --arg first_trace_sha256 "$first_trace_digest" \
  --arg second_trace_sha256 "$second_trace_digest" \
  --slurpfile probe "$scratch/dwv-probe.json" \
  --slurpfile init "$scratch/dwv-init.json" \
  --slurpfile first_shutdown "$scratch/dwv-serve-first.json" \
  --slurpfile second_shutdown "$scratch/dwv-serve-second.json" \
  --slurpfile inspect "$scratch/dwv-inspect.json" \
  --slurpfile production_start "$scratch/dwv-production-start.json" \
  --slurpfile first_trace "$scratch/dwv-trace-first.json" \
  --slurpfile second_trace "$scratch/dwv-trace-second.json" \
  --slurpfile first_trace_replay "$scratch/dwv-trace-first-replay.json" \
  --slurpfile second_trace_replay "$scratch/dwv-trace-second-replay.json" \
  --rawfile unknown_cleanup "$scratch/dwv-unknown-cleanup.json" \
  --rawfile ownership_conflict "$scratch/dwv-ownership-conflict.json" \
  --rawfile conflicting_cleanup "$scratch/dwv-conflicting-cleanup.json" \
  --rawfile stale_readiness "$scratch/dwv-stale-readiness.json" \
  --rawfile owner_death_reconciliation "$scratch/dwv-owner-death-reconciliation.json" \
  --slurpfile owner_death_cleanup "$scratch/dwv-owner-death-cleanup.json" \
  --slurpfile owner_death_restart "$scratch/dwv-owner-death-restart.json" \
  --rawfile unsupported_discard "$scratch/dwv-unsupported-discard.txt" \
  '{
    schema: "dwv.verification.linux-ublk-ext4.v1",
    source_snapshot_sha256: $source_snapshot_sha256,
    kernel_release: $kernel_release,
    architecture: $architecture,
    live_elapsed_seconds: $live_elapsed_seconds,
    probe: $probe[0],
    init: $init[0],
    production_start: $production_start[0],
    workload: {
      filesystem: "ext4",
      operations: ["format", "mount", "create", "fsync", "overwrite", "rename", "read", "delete", "unmount", "restart", "read-only-remount"],
      content_sha256: $content_sha256
    },
    first_shutdown: $first_shutdown[0],
    second_shutdown: $second_shutdown[0],
    live_traces: [
      {
        phase: "first-shutdown",
        retained_file: "linux-ublk-trace-first.json",
        sha256: $first_trace_sha256,
        schema: $first_trace[0].schema,
        fixture_digest: $first_trace[0].fixture_digest,
        bounds: $first_trace[0].bounds,
        exhausted_records: $first_trace[0].exhausted_records,
        record_count: ($first_trace[0].records | length),
        replay: $first_trace_replay[0]
      },
      {
        phase: "second-shutdown",
        retained_file: "linux-ublk-trace-second.json",
        sha256: $second_trace_sha256,
        schema: $second_trace[0].schema,
        fixture_digest: $second_trace[0].fixture_digest,
        bounds: $second_trace[0].bounds,
        exhausted_records: $second_trace[0].exhausted_records,
        record_count: ($second_trace[0].records | length),
        replay: $second_trace_replay[0]
      }
    ],
    backing_file: {
      independently_mounted_read_only: true,
      content_sha256: $content_sha256,
      data_sha256: $data_sha256,
      parity_sha256: $parity_sha256,
      data_equals_parity: ($data_sha256 == $parity_sha256)
    },
    inspect: $inspect[0],
    unknown_cleanup: $unknown_cleanup,
    negative_cases: {
      ownership_conflict: $ownership_conflict,
      conflicting_cleanup: $conflicting_cleanup,
      stale_readiness: $stale_readiness,
      unsupported_discard: $unsupported_discard,
      protected_payloads_unchanged: true,
      owner_process_death: {
        reconciliation: $owner_death_reconciliation,
        cleanup: $owner_death_cleanup[0],
        restart: $owner_death_restart[0]
      }
    },
    non_claims: ["production durability", "power-loss safety", "multi-device publication", "daemon recovery", "FUA", "discard", "write-zeroes", "online topology mutation", "hardware safety"]
  }' > "$results_root/evidence.json"
cp "$scratch/dwv-trace-first.json" "$results_root/linux-ublk-trace-first.json"
cp "$scratch/dwv-trace-second.json" "$results_root/linux-ublk-trace-second.json"
cat "$results_root/evidence.json"
