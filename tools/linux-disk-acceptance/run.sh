#!/usr/bin/env bash
set -euo pipefail

repo=$(cd "$(dirname "$0")/../.." && pwd)
output=${1:-$repo/verification/linux-ublk-ext4-acceptance.json}
vm=dwv-linux-acceptance-$(date +%s)
tmp=$(mktemp -d)

cleanup() {
  limactl stop --force "$vm" >/dev/null 2>&1 || true
  limactl delete --force "$vm" >/dev/null 2>&1 || true
  rm -rf "$tmp"
}
trap cleanup EXIT

command -v limactl >/dev/null
command -v shasum >/dev/null
[[ $(uname -m) == arm64 ]]

cd "$repo"
COPYFILE_DISABLE=1 tar \
  --no-xattrs \
  --exclude=.git \
  --exclude=.jj \
  --exclude=.omp \
  --exclude=target \
  --exclude=tools/macos-bridge-probe/.build \
  -czf "$tmp/source.tar.gz" .
source_hash=$(shasum -a 256 "$tmp/source.tar.gz" | cut -d' ' -f1)

limactl start --name="$vm" --tty=false tools/linux-disk-acceptance/lima.yaml
limactl copy "$tmp/source.tar.gz" "$vm:/tmp/source.tar.gz"
limactl shell "$vm" -- bash -lc 'rm -rf /tmp/diskweave-src && mkdir /tmp/diskweave-src && tar -xzf /tmp/source.tar.gz -C /tmp/diskweave-src'
limactl shell "$vm" -- sudo /tmp/diskweave-src/tools/linux-disk-acceptance/guest.sh "$source_hash"
mkdir -p "$(dirname "$output")"
limactl copy "$vm:/tmp/dwv-linux-acceptance-evidence.json" "$output"
limactl copy "$vm:/tmp/dwv-trace-first.json" "$(dirname "$output")/linux-ublk-trace-first.json"
limactl copy "$vm:/tmp/dwv-trace-second.json" "$(dirname "$output")/linux-ublk-trace-second.json"
cat "$output"
