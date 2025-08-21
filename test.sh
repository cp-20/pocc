#!/bin/bash
target="target/debug/pocc"

# Usage: test.sh [--target path/to/target]
while [[ $# -gt 0 ]]; do
  case "$1" in
  --target)
    shift
    target="$1"
    shift
    ;;
  esac
done

pass=0
fail=0

col_red="\x1b[31m"
col_green="\x1b[32m"
col_yellow="\x1b[33m"
col_blue="\x1b[34m"
col_magenta="\x1b[35m"
col_cyan="\x1b[36m"
col_gray="\033[2;29m"
col_reset="\x1b[0m"

test() {
  local src="$1"
  local args=("${@:2}")
  local base_name="$(basename "$src" .c)"
  local dist_asm=".build/test/${base_name}.s"
  local dist=".build/test/${base_name}"
  local dist_output="${dist}-output"
  local dist_debug_err="${dist}-error"
  local dist_expected_out="${dist}-expected_out"
  local result_file=".build/test/${base_name}.result"
  local output_file=".build/test/${base_name}.test_output"

  mkdir -p "$(dirname "$dist_asm")"

  {
    $target "$src" >"$dist_asm" 2>"$dist_debug_err"
    if [ $? -ne 0 ]; then
      echo -e "${col_red}❌ [NG]${col_reset} $src $args"
      echo -e "    ${col_red}❌ Compile${col_reset}"
      echo -e "    ${col_gray}   Assemble${col_reset}"
      echo -e "    ${col_gray}   Execute${col_reset}"
      echo -e "\n$(cat "$dist_debug_err" | sed "s/\\\\n/\\\\\\\\n/" | sed "s/^/${col_reset} |> ${col_yellow}/")${col_reset}\n\n"
      echo "fail" > "$result_file"
      return
    fi

    clang -o "$dist" "$dist_asm"
    if [ $? -ne 0 ]; then
      echo -e "${col_red}❌ [NG]${col_reset} $src $args"
      echo -e "    ${col_reset}✅ Compile${col_reset}"
      echo -e "    ${col_red}❌ Assemble${col_reset}"
      echo -e "    ${col_gray}   Execute${col_reset}"
      echo "fail" > "$result_file"
      return
    fi

    ./"$dist" ${args[@]} >"$dist_output"
    if [ $? -ne 0 ]; then
      echo -e "${col_red}❌ [NG]${col_reset} $src $args"
      echo -e "    ${col_reset}✅ Compile${col_reset}"
      echo -e "    ${col_reset}✅ Assemble${col_reset}"
      echo -e "    ${col_red}❌ Execute${col_reset}"
      echo -e "\n$(cat "$dist_debug_err" | sed "s/\\\\n/\\\\\\\\n/" | sed "s/^/${col_reset} |> ${col_yellow}/")${col_reset}\n\n"
      echo "fail" > "$result_file"
      return
    fi

    clang $src -o ".build/test/${base_name}_expected" 2>/dev/null && ./".build/test/${base_name}_expected" ${args[@]} >"$dist_expected_out"
    diff -q "$dist_output" "$dist_expected_out" >/dev/null
    if [ $? -ne 0 ]; then
      echo -e "${col_red}❌ [NG]${col_reset} $src $args"
      echo -e "    ${col_reset}✅ Compile${col_reset}"
      echo -e "    ${col_reset}✅ Assemble${col_reset}"
      echo -e "    ${col_reset}✅ Execute${col_reset}"
      echo -e "    ${col_yellow}⚠️  Output differs from expected${col_reset}"
      echo "fail" > "$result_file"
      return
    fi

    echo "pass" > "$result_file"
    echo -e "${col_green}✅ [OK]${col_reset} $src $args"
  } > "$output_file"
}

test "tests/function.c" &
test "tests/variable.c" &
test "tests/bubble_sort.c" 30 &
# test "tests/pointer.c" &

wait

for test_name in "function" "variable" "bubble_sort"; do
  output_file=".build/test/${test_name}.test_output"
  if [ -f "$output_file" ]; then
    cat "$output_file"
  fi
done

# 結果集計
for result in .build/test/*.result; do
  if [ "$(cat "$result")" = "pass" ]; then
    pass=$((pass + 1))
  else
    fail=$((fail + 1))
  fi
done

echo "Passed: $pass, Failed: $fail"
if [ $fail -eq 0 ]; then
  echo -e "${col_green}All tests passed!${col_reset}🎉"
else
  echo -e "${col_red}Some tests failed.${col_reset}🤔"
fi
exit $fail
fi
exit $fail
