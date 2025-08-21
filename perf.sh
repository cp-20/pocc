#!/bin/bash

target="target/debug/pocc"
with_clang=false

# Usage: test.sh [--target path/to/target]
while [[ $# -gt 0 ]]; do
  case "$1" in
  --target)
    shift
    target="$1"
    shift
    ;;
  --with-clang)
    with_clang=true
    shift
    ;;
  esac
done

bench() {
  local src="$1"
  local args=("${@:2}")
  local base_name="$(basename "$src" .c)"
  local dist_asm=".build/perf/${base_name}.s"
  local dist=".build/perf/${base_name}"

  mkdir -p "$(dirname "$dist_asm")"

  $target "$src" >"$dist_asm" 2>/dev/null
  clang -o "$dist" -O0 "$dist_asm" 2>/dev/null

  if [ "$with_clang" = true ]; then
    local dist_o1=".build/perf/${base_name}-o1"
    local dist_o2=".build/perf/${base_name}-o2"
    local dist_o3=".build/perf/${base_name}-o3"
    clang -o "$dist_o1" -O1 "$src" 2>/dev/null
    clang -o "$dist_o2" -O2 "$src" 2>/dev/null
    clang -o "$dist_o3" -O3 "$src" 2>/dev/null

    hyperfine "./$dist ${args[@]}" \
      "./$dist_o1 ${args[@]}" \
      "./$dist_o2 ${args[@]}" \
      "./$dist_o3 ${args[@]}"
  else
    hyperfine "./$dist ${args[@]}"
  fi
}

bench "tests/bubble_sort.c" 10000
