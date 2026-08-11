#!/usr/bin/env bash
set -euo pipefail

script_dir=$(cd -- "$(dirname -- "$0")" && pwd -P)
repo=$(cd -- "$script_dir/../.." && pwd -P)
output=${1:-"$repo/verification/linux-ublk-ext4-acceptance.json"}
[[ "$output" = /* ]] || output=$PWD/$output
output_dir=$(dirname -- "$output")
vm=dwv-linux-acceptance

command -v limactl >/dev/null
command -v shasum >/dev/null
command -v shlock >/dev/null
[[ $(uname -m) == arm64 ]]

tmp=$(mktemp -d)
lock=${tmp%/*}/dwv-linux-disk-acceptance.lock
if ! shlock -f "$lock" -p "$$"; then
  rm -rf "$tmp"
  printf 'Linux disk acceptance is already running\n' >&2
  exit 1
fi

cleanup() {
  rm -f "$lock"
  rm -rf "$tmp"
}
trap cleanup EXIT

started=$SECONDS
archive=$tmp/source.tar.gz
COPYFILE_DISABLE=1 tar \
  --no-xattrs \
  --exclude=.git \
  --exclude=.jj \
  --exclude=.omp \
  --exclude=target \
  --exclude=tools/macos-bridge-probe/.build \
  -C "$repo" \
  -czf "$archive" .
source_hash=$(shasum -a 256 "$archive" | cut -d' ' -f1)

prepared=false
if status=$(limactl list "$vm" --format '{{.Status}}' 2>/dev/null); then
  [[ "$status" == Running ]] || limactl start --tty=false "$vm"
else
  limactl start --name="$vm" --tty=false "$script_dir/lima.yaml"
fi
limactl shell --tty=false "$vm" -- sudo modprobe ublk_drv
guest_state=$(limactl shell --tty=false "$vm" -- bash -lc '
  state=${XDG_STATE_HOME:-$HOME/.local/state}/diskweave/linux-disk-acceptance
  mkdir -p "$state"
  printf "%s" "$state"
')
if limactl shell --tty=false "$vm" -- test -f "$guest_state/prepared"; then
  prepared=true
fi
guest_work=$(limactl shell --tty=false "$vm" -- mktemp -d)
limactl copy "$archive" "$vm:$guest_work/source.tar.gz"
limactl shell --tty=false "$vm" -- bash -lc '
  set -euo pipefail
  state=$1
  work=$2
  source_hash=$3
  cache=${XDG_CACHE_HOME:-$HOME/.cache}/diskweave/linux-disk-acceptance
  source_root=$work/source
  results_root=$state/results

  cleanup_work() {
    rm -rf -- "$work"
  }
  trap cleanup_work EXIT

  command -v flock >/dev/null
  exec 9>"$state/run.lock"
  flock -n 9
  rm -rf -- "$results_root"
  mkdir -p "$source_root" "$results_root" "$cache/target"
  tar -xzf "$work/source.tar.gz" -C "$source_root"
  ln -s "$cache/target" "$source_root/target"
  cd "$source_root"
  cargo build --release --bin dwv
  sudo "$source_root/tools/linux-disk-acceptance/guest.sh" \
    "$source_hash" "$source_root" "$results_root"
' _ "$guest_state" "$guest_work" "$source_hash"

mkdir -p "$tmp/results"
limactl copy "$vm:$guest_state/results/evidence.json" "$tmp/results/evidence.json"
limactl copy "$vm:$guest_state/results/linux-ublk-trace-first.json" \
  "$tmp/results/linux-ublk-trace-first.json"
limactl copy "$vm:$guest_state/results/linux-ublk-trace-second.json" \
  "$tmp/results/linux-ublk-trace-second.json"
elapsed=$((SECONDS - started))
if [[ "$prepared" == true && "$elapsed" -ge 30 ]]; then
  printf 'warm Linux disk acceptance reached the 30-second limit: %ss\n' "$elapsed" >&2
  exit 1
fi

mkdir -p "$output_dir"
mv "$tmp/results/evidence.json" "$output"
mv "$tmp/results/linux-ublk-trace-first.json" "$output_dir/linux-ublk-trace-first.json"
mv "$tmp/results/linux-ublk-trace-second.json" "$output_dir/linux-ublk-trace-second.json"
limactl shell --tty=false "$vm" -- touch "$guest_state/prepared"
printf 'Linux disk acceptance completed in %ss (%s runner)\n' \
  "$elapsed" "$([[ "$prepared" == true ]] && printf warm || printf cold)" >&2
cat "$output"
