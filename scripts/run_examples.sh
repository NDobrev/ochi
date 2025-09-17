#!/usr/bin/env bash
set -euo pipefail

# Run from repo root. Assemble all examples/*.asm, write bins to examples/out,
# and disassemble each for a quick sanity check.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")"/.. && pwd)"
EX_DIR="$ROOT_DIR/examples"
OUT_DIR="$EX_DIR/out"
mkdir -p "$OUT_DIR"

BASE=0 # match assembler default --start

for asm in "$EX_DIR"/*.asm; do
  name="$(basename "$asm" .asm)"
  bin="$OUT_DIR/$name.bin"
  txt="$OUT_DIR/$name.disasm.txt"
  json="$OUT_DIR/$name.analysis.json"
  cpu_json="$OUT_DIR/$name.cpu.json"

  echo "[ASM ] $asm -> $bin"
  cargo run -q -p tricore-disasm --bin asm -- --input "$asm" --output "$bin" --start "$BASE"

  size=$(stat -c%s "$bin")
  end=$((BASE + size))
  echo "[DISM] $bin -> $txt (size=$size)"
  cargo run -q -p tricore-disasm --bin tricore-disasm -- --base "$BASE" "$bin" range "$BASE" "$end" --show-bytes --out "$txt" || true

  echo "[ANAL] $bin -> $json"
  cargo run -q -p tricore-disasm --bin tricore-disasm -- --base "$BASE" "$bin" analyze --format json --out "$json" || true

  echo "[EMUL] $bin -> $cpu_json"
  cargo run -q --bin tricore-run -- --load-addr "$BASE" "$bin" --dump-cpu "$cpu_json" || true
done

echo "Done. Outputs in $OUT_DIR"
