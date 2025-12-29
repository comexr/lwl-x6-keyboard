#!/usr/bin/env bash
set -euo pipefail

# Reads ~/.rusty-kb/colors.txt (or a custom path) and applies keyboard/lightbar
# colors and brightness to the sysfs LED interfaces.
#
# File format (four integers per line):
# 1st line: kb_r kb_g kb_b kb_brightness
# 2nd line: lb_r lb_g lb_b lb_brightness
#
# Usage:
#   scripts/apply_colors_from_file.sh [path_to_colors_file]

FILE="${1:-"$HOME/.rusty-kb/colors.txt"}"
FALLBACK="/usr/lib/rusty-kb/colors.txt"

if [[ ! -f "$FILE" && -f "$FALLBACK" ]]; then
  FILE="$FALLBACK"
fi

if [[ ! -f "$FILE" ]]; then
  echo "Color file not found: $FILE" >&2
  exit 1
fi

# Read file lines
mapfile -t LINES < "$FILE"
if ((${#LINES[@]} < 1)); then
  echo "Color file is empty: $FILE" >&2
  exit 1
fi

read -r KB_R KB_G KB_B KB_BRIGHT <<<"${LINES[0]:-}"
read -r LB_R LB_G LB_B LB_BRIGHT <<<"${LINES[1]:-0 0 0 0}"

KB_PATTERN="/sys/class/leds/rgb:kbd_backlight*"
LB_PATH="/sys/class/leds/rgb:lightbar"

apply_kb() {
  local r g b bright
  r="$1"; g="$2"; b="$3"; bright="$4"
  shopt -s nullglob
  local paths=($KB_PATTERN)
  shopt -u nullglob
  if ((${#paths[@]} == 0)); then
    echo "No keyboard LED paths found under $KB_PATTERN" >&2
    return 1
  fi

  for p in "${paths[@]}"; do
    if [[ -w "$p/multi_intensity" ]]; then
      printf "%s %s %s\n" "$r" "$g" "$b" > "$p/multi_intensity" || true
    else
      echo "No write access to $p/multi_intensity" >&2
    fi

    if [[ -w "$p/brightness" ]]; then
      printf "%s\n" "$bright" > "$p/brightness" || true
    else
      echo "No write access to $p/brightness" >&2
    fi
  done
}

apply_lb() {
  local r g b bright
  r="$1"; g="$2"; b="$3"; bright="$4"
  if [[ ! -d "$LB_PATH" ]]; then
    return 0
  fi
  if [[ -w "$LB_PATH/multi_intensity" ]]; then
    printf "%s %s %s\n" "$r" "$g" "$b" > "$LB_PATH/multi_intensity" || true
  else
    echo "No write access to $LB_PATH/multi_intensity" >&2
  fi

  if [[ -w "$LB_PATH/brightness" ]]; then
    printf "%s\n" "$bright" > "$LB_PATH/brightness" || true
  else
    echo "No write access to $LB_PATH/brightness" >&2
  fi
}

apply_kb "$KB_R" "$KB_G" "$KB_B" "$KB_BRIGHT"
apply_lb "$LB_R" "$LB_G" "$LB_B" "$LB_BRIGHT"

echo "Applied keyboard: $KB_R $KB_G $KB_B (brightness $KB_BRIGHT)"
if [[ -d "$LB_PATH" ]]; then
  echo "Applied lightbar: $LB_R $LB_G $LB_B (brightness $LB_BRIGHT)"
else
  echo "Lightbar path not present; skipped"
fi
