#!/usr/bin/env bash
set -euo pipefail

source_hash=${1:?source snapshot hash required}
source_root=/tmp/diskweave-src
fixture=/var/tmp/dwv-linux-acceptance
mountpoint=/mnt/dwv-linux-acceptance
server_pid=

cleanup() {
  mountpoint -q "$mountpoint" && umount "$mountpoint" || true
  if [[ -n "$server_pid" ]] && kill -0 "$server_pid" 2>/dev/null; then
    kill -TERM "$server_pid"
    wait "$server_pid" || true
  fi
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
cargo test -p dwv-frontend-ublk
cargo test -p dwv-transaction-ref partial_multi_store_fence_is_rejected
cargo build --bin dwv
./target/debug/dwv demo disk probe > /tmp/dwv-probe.json
rm -rf "$fixture" /var/tmp/dwv-too-small /var/tmp/dwv-negative /var/tmp/dwv-recovery-loss /var/tmp/dwv-cleanup-conflict /var/tmp/dwv-owner-death
mkdir -p "$mountpoint"
./target/debug/dwv demo disk init --root "$fixture" --size 67108864 > /tmp/dwv-init.json
if ./target/debug/dwv demo disk init --root /var/tmp/dwv-too-small --size 4096 2> /tmp/dwv-resource-bound.json; then
  echo "fixture initializer unexpectedly accepted an undersized device" >&2
  exit 1
fi
[[ ! -e /var/tmp/dwv-too-small ]]
negative_fixture=/var/tmp/dwv-negative
./target/debug/dwv demo disk init --root "$negative_fixture" --size 16777216 > /tmp/dwv-negative-init.json
jq '.data_files += ["unsupported.raw"]' "$negative_fixture/fixture.json" > /tmp/dwv-negative-manifest.json
mv /tmp/dwv-negative-manifest.json "$negative_fixture/fixture.json"
negative_before=$(sha256sum "$negative_fixture/data.raw" "$negative_fixture/parity.raw")
if ./target/debug/dwv demo disk inspect --root "$negative_fixture" 2> /tmp/dwv-unsupported-topology.json; then
  echo "inspection unexpectedly accepted an unsupported topology" >&2
  exit 1
fi
[[ "$negative_before" == "$(sha256sum "$negative_fixture/data.raw" "$negative_fixture/parity.raw")" ]]
recovery_loss_fixture=/var/tmp/dwv-recovery-loss
./target/debug/dwv demo disk init --root "$recovery_loss_fixture" --size 16777216 > /tmp/dwv-recovery-loss-init.json
recovery_loss_before=$(sha256sum "$recovery_loss_fixture/data.raw" "$recovery_loss_fixture/parity.raw")
rm "$recovery_loss_fixture/recovery.sqlite3"
if ./target/debug/dwv demo disk inspect --root "$recovery_loss_fixture" 2> /tmp/dwv-recovery-loss.json; then
  echo "inspection unexpectedly accepted missing recovery authority" >&2
  exit 1
fi
[[ "$recovery_loss_before" == "$(sha256sum "$recovery_loss_fixture/data.raw" "$recovery_loss_fixture/parity.raw")" ]]

./target/debug/dwv demo disk serve --root "$fixture" > /tmp/dwv-serve-first.json &
server_pid=$!
wait_ready
device=$(jq -r .device_path "$fixture/ready.json")
device_id=$(jq -r .device_id "$fixture/ready.json")
if ./target/debug/dwv demo disk serve --root "$fixture" 2> /tmp/dwv-ownership-conflict.json; then
  echo "second server unexpectedly acquired the fixture" >&2
  exit 1
fi
cleanup_fixture=/var/tmp/dwv-cleanup-conflict
./target/debug/dwv demo disk init --root "$cleanup_fixture" --size 16777216 > /tmp/dwv-cleanup-conflict-init.json
if ./target/debug/dwv demo disk cleanup --root "$cleanup_fixture" --device-id "$device_id" 2> /tmp/dwv-conflicting-cleanup.json; then
  echo "cleanup unexpectedly removed a differently owned endpoint" >&2
  exit 1
fi
[[ -b "$device" ]]
pre_discard=$(sha256sum "$fixture/data.raw" "$fixture/parity.raw")
if blkdiscard "$device" 2> /tmp/dwv-unsupported-discard.txt; then
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
cp "$fixture/trace.json" /tmp/dwv-trace-first.json

./target/debug/dwv demo disk serve --root "$fixture" > /tmp/dwv-serve-second.json &
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
cp "$fixture/trace.json" /tmp/dwv-trace-second.json
./target/debug/dwv demo disk trace-replay --trace /tmp/dwv-trace-first.json > /tmp/dwv-trace-first-replay.json
./target/debug/dwv demo disk trace-replay --trace /tmp/dwv-trace-second.json > /tmp/dwv-trace-second-replay.json
first_trace_digest=$(sha256sum /tmp/dwv-trace-first.json | cut -d' ' -f1)
second_trace_digest=$(sha256sum /tmp/dwv-trace-second.json | cut -d' ' -f1)
jq -e \
  --arg digest "$(jq -r .fixture_digest /tmp/dwv-serve-first.json)" \
  --argjson count "$(jq -r .trace_count /tmp/dwv-serve-first.json)" \
  '.schema == "dwv.ublk.trace.v2" and .fixture_digest == $digest and
   .bounds.queue_depth == 8 and .bounds.maximum_transfer == 131072 and
   .bounds.maximum_records == 4096 and .exhausted_records == 0 and
   (.records | length) == $count' /tmp/dwv-trace-first.json >/dev/null
jq -e \
  --arg digest "$(jq -r .fixture_digest /tmp/dwv-serve-second.json)" \
  --argjson count "$(jq -r .trace_count /tmp/dwv-serve-second.json)" \
  '.schema == "dwv.ublk.trace.v2" and .fixture_digest == $digest and
   .bounds.queue_depth == 8 and .bounds.maximum_transfer == 131072 and
   .bounds.maximum_records == 4096 and .exhausted_records == 0 and
   (.records | length) == $count' /tmp/dwv-trace-second.json >/dev/null
[[ $(stat -c %s /tmp/dwv-trace-first.json) -le 4194304 ]]
[[ $(stat -c %s /tmp/dwv-trace-second.json) -le 4194304 ]]
printf '{}\n' > "$fixture/ready.json"
if ./target/debug/dwv demo disk serve --root "$fixture" 2> /tmp/dwv-stale-readiness.json; then
  echo "serve unexpectedly accepted stale readiness state" >&2
  exit 1
fi
rm "$fixture/ready.json"
main_fixture=$fixture
fixture=/var/tmp/dwv-owner-death
./target/debug/dwv demo disk init --root "$fixture" --size 16777216 > /tmp/dwv-owner-death-init.json
./target/debug/dwv demo disk serve --root "$fixture" > /tmp/dwv-owner-death-first.json &
server_pid=$!
wait_ready
owner_device_id=$(jq -r .device_id "$fixture/ready.json")
kill -KILL "$server_pid"
wait "$server_pid" || true
server_pid=
if ./target/debug/dwv demo disk serve --root "$fixture" 2> /tmp/dwv-owner-death-reconciliation.json; then
  echo "serve unexpectedly ignored stale publication after owner death" >&2
  exit 1
fi
./target/debug/dwv demo disk cleanup --root "$fixture" --device-id "$owner_device_id" > /tmp/dwv-owner-death-cleanup.json
./target/debug/dwv demo disk serve --root "$fixture" > /tmp/dwv-owner-death-restart.json &
server_pid=$!
wait_ready
kill -TERM "$server_pid"
wait "$server_pid"
server_pid=
fixture=$main_fixture

mount -o loop,ro,noload "$fixture/data.raw" "$mountpoint"
backing_hash=$(sha256sum "$mountpoint/durable.txt" | cut -d' ' -f1)
[[ "$backing_hash" == "$first_hash" ]]
[[ ! -e "$mountpoint/delete.me" ]]
umount "$mountpoint"
cmp "$fixture/data.raw" "$fixture/parity.raw"
data_hash=$(sha256sum "$fixture/data.raw" | cut -d' ' -f1)
parity_hash=$(sha256sum "$fixture/parity.raw" | cut -d' ' -f1)
./target/debug/dwv demo disk inspect --root "$fixture" > /tmp/dwv-inspect.json
if ./target/debug/dwv demo disk cleanup --root "$fixture" --device-id 2147483647 2> /tmp/dwv-unknown-cleanup.json; then
  echo "cleanup unexpectedly accepted an unknown endpoint" >&2
  exit 1
fi

jq -n \
  --arg source_snapshot_sha256 "$source_hash" \
  --arg kernel_release "$(uname -r)" \
  --arg architecture "$(uname -m)" \
  --arg content_sha256 "$first_hash" \
  --arg data_sha256 "$data_hash" \
  --arg parity_sha256 "$parity_hash" \
  --arg first_trace_sha256 "$first_trace_digest" \
  --arg second_trace_sha256 "$second_trace_digest" \
  --slurpfile probe /tmp/dwv-probe.json \
  --slurpfile init /tmp/dwv-init.json \
  --slurpfile first_shutdown /tmp/dwv-serve-first.json \
  --slurpfile second_shutdown /tmp/dwv-serve-second.json \
  --slurpfile inspect /tmp/dwv-inspect.json \
  --slurpfile first_trace /tmp/dwv-trace-first.json \
  --slurpfile second_trace /tmp/dwv-trace-second.json \
  --slurpfile first_trace_replay /tmp/dwv-trace-first-replay.json \
  --slurpfile second_trace_replay /tmp/dwv-trace-second-replay.json \
  --rawfile unknown_cleanup /tmp/dwv-unknown-cleanup.json \
  --rawfile resource_bound /tmp/dwv-resource-bound.json \
  --rawfile unsupported_topology /tmp/dwv-unsupported-topology.json \
  --rawfile ownership_conflict /tmp/dwv-ownership-conflict.json \
  --rawfile conflicting_cleanup /tmp/dwv-conflicting-cleanup.json \
  --rawfile stale_readiness /tmp/dwv-stale-readiness.json \
  --rawfile recovery_loss /tmp/dwv-recovery-loss.json \
  --rawfile owner_death_reconciliation /tmp/dwv-owner-death-reconciliation.json \
  --slurpfile owner_death_cleanup /tmp/dwv-owner-death-cleanup.json \
  --slurpfile owner_death_restart /tmp/dwv-owner-death-restart.json \
  --rawfile unsupported_discard /tmp/dwv-unsupported-discard.txt \
  '{
    schema: "dwv.verification.linux-ublk-ext4.v1",
    source_snapshot_sha256: $source_snapshot_sha256,
    kernel_release: $kernel_release,
    architecture: $architecture,
    probe: $probe[0],
    init: $init[0],
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
      resource_bound: $resource_bound,
      unsupported_topology: $unsupported_topology,
      ownership_conflict: $ownership_conflict,
      conflicting_cleanup: $conflicting_cleanup,
      stale_readiness: $stale_readiness,
      unsupported_discard: $unsupported_discard,
      protected_payloads_unchanged: true,
      recovery_authority_loss: $recovery_loss,
      owner_process_death: {
        reconciliation: $owner_death_reconciliation,
        cleanup: $owner_death_cleanup[0],
        restart: $owner_death_restart[0]
      },
      partial_fence_coverage_rejected_by: "cargo test -p dwv-transaction-ref partial_multi_store_fence_is_rejected"
    },
    non_claims: ["production durability", "power-loss safety", "multi-device publication", "daemon recovery", "FUA", "discard", "write-zeroes", "online topology mutation", "hardware safety"]
  }' > /tmp/dwv-linux-acceptance-evidence.json
cat /tmp/dwv-linux-acceptance-evidence.json
