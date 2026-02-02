#!/usr/bin/env bash
set -euo pipefail
KB_BASE_PATH="/sys/devices/pci0000:00/0000:00:08.3/0000:07:00.0/usb5/5-1/5-1.1/5-1.1:1.1/0003:048D:600B.0002/leds/"
BAR_BASE_PATH="/sys/class/leds/rgb:lightbar"
COLOR_FILE="$HOME/.rusty-kb/colors.txt"

if [ -z $COLOR_FILE ]; then
    #using fallback path
    COLOR_FILE="/usr/lib/rusty-kb/colors.txt"
fi

#US iso layout
declare -A KEY_MAP
KEY_MAP=(
  ["l-ctrl"]="rgb:kbd_backlight"
  ["fn"]="rgb:kbd_backlight_2"
  ["super"]="rgb:kbd_backlight_3"
  ["alt"]="rgb:kbd_backlight_4"
  ["altgr"]="rgb:kbd_backlight_10"
  ["r-ctrl"]="rgb:kbd_backlight_12"
  ["left-arrow"]="rgb:kbd_backlight_13"
  ["up-arrow"]="rgb:kbd_backlight_14"
  ["right-arrow"]="rgb:kbd_backlight_15"
  ["num-0"]="rgb:kbd_backlight_16"
  ["num-."]="rgb:kbd_backlight_17"
  ["down-arrow"]="rgb:kbd_backlight_18"
  ["l-shift"]="rgb:kbd_backlight_22"
  ["<"]="rgb:kbd_backlight_23"
  ["z"]="rgb:kbd_backlight_24"
  ["x"]="rgb:kbd_backlight_25"
  ["c"]="rgb:kbd_backlight_26"
  ["v"]="rgb:kbd_backlight_27"
  ["b"]="rgb:kbd_backlight_28"
  ["n"]="rgb:kbd_backlight_29"
  ["m"]="rgb:kbd_backlight_30"
  [","]="rgb:kbd_backlight_31"
  ["."]="rgb:kbd_backlight_32"
  ["/"]="rgb:kbd_backlight_33"
  ["r-shift"]="rgb:kbd_backlight_35"
  ["num-1"]="rgb:kbd_backlight_36"
  ["num-2"]="rgb:kbd_backlight_37"
  ["num-3"]="rgb:kbd_backlight_38"
  ["num-return"]="rgb:kbd_backlight_39"
  ["caps"]="rgb:kbd_backlight_42"
  ["a"]="rgb:kbd_backlight_44"
  ["s"]="rgb:kbd_backlight_45"
  ["d"]="rgb:kbd_backlight_46"
  ["f"]="rgb:kbd_backlight_47"
  ["g"]="rgb:kbd_backlight_48"
  ["h"]="rgb:kbd_backlight_49"
  ["j"]="rgb:kbd_backlight_50"
  ["k"]="rgb:kbd_backlight_51"
  ["l"]="rgb:kbd_backlight_52"
  [";"]="rgb:kbd_backlight_53"
  ["'"]="rgb:kbd_backlight_54"
  ["\\"]="rgb:kbd_backlight_55"
  ["num-4"]="rgb:kbd_backlight_57"
  ["num-5"]="rgb:kbd_backlight_58"
  ["num-6"]="rgb:kbd_backlight_59"
  ["tab"]="rgb:kbd_backlight_63"
  ["q"]="rgb:kbd_backlight_65"
  ["w"]="rgb:kbd_backlight_66"
  ["e"]="rgb:kbd_backlight_67"
  ["r"]="rgb:kbd_backlight_68"
  ["t"]="rgb:kbd_backlight_69"
  ["y"]="rgb:kbd_backlight_70"
  ["u"]="rgb:kbd_backlight_71"
  ["i"]="rgb:kbd_backlight_72"
  ["o"]="rgb:kbd_backlight_73"
  ["p"]="rgb:kbd_backlight_74"
  ["["]="rgb:kbd_backlight_75"
  ["]"]="rgb:kbd_backlight_76"
  ["return"]="rgb:kbd_backlight_77"
  ["num-7"]="rgb:kbd_backlight_78"
  ["num-8"]="rgb:kbd_backlight_79"
  ["num-9"]="rgb:kbd_backlight_80"
  ["num-+"]="rgb:kbd_backlight_81"
  ["~"]="rgb:kbd_backlight_84"
  ["1"]="rgb:kbd_backlight_85"
  ["2"]="rgb:kbd_backlight_86"
  ["3"]="rgb:kbd_backlight_87"
  ["4"]="rgb:kbd_backlight_88"
  ["5"]="rgb:kbd_backlight_89"
  ["6"]="rgb:kbd_backlight_90"
  ["7"]="rgb:kbd_backlight_91"
  ["8"]="rgb:kbd_backlight_92"
  ["9"]="rgb:kbd_backlight_93"
  ["0"]="rgb:kbd_backlight_94"
  ["-"]="rgb:kbd_backlight_95"
  ["="]="rgb:kbd_backlight_96"
  ["backspace"]="rgb:kbd_backlight_98"
  ["num"]="rgb:kbd_backlight_99"
  ["num-/"]="rgb:kbd_backlight_100"
  ["num-*"]="rgb:kbd_backlight_101"
  ["num--"]="rgb:kbd_backlight_102"
  ["esc"]="rgb:kbd_backlight_105"
  ["f1"]="rgb:kbd_backlight_106"
  ["f2"]="rgb:kbd_backlight_107"
  ["f3"]="rgb:kbd_backlight_108"
  ["f4"]="rgb:kbd_backlight_109"
  ["f5"]="rgb:kbd_backlight_110"
  ["f6"]="rgb:kbd_backlight_111"
  ["f7"]="rgb:kbd_backlight_112"
  ["f8"]="rgb:kbd_backlight_113"
  ["f9"]="rgb:kbd_backlight_114"
  ["f10"]="rgb:kbd_backlight_115"
  ["f11"]="rgb:kbd_backlight_116"
  ["f12"]="rgb:kbd_backlight_117"
  ["sc"]="rgb:kbd_backlight_118"
  ["prtsc"]="rgb:kbd_backlight_119"
  ["del"]="rgb:kbd_backlight_120"
  ["home"]="rgb:kbd_backlight_121"
  ["pgup"]="rgb:kbd_backlight_122"
  ["pgdn"]="rgb:kbd_backlight_123"
  ["end"]="rgb:kbd_backlight_124"
  [' ']="rgb:kbd_backlight_7"
)

# Function to set color for all keys (parallelized)
set_all_keys_color() {
    local color="$1"
    for key in "${!KEY_MAP[@]}"; do
        if [[ -n "${KEY_MAP[$key]}" ]]; then
            echo "$color" > "${KB_BASE_PATH}${KEY_MAP[$key]}/multi_intensity" &
        fi
    done
    wait
}

# Function to set brightness for all keys (parallelized)
set_all_keys_brightness() {
    local level="$1"
    local brightness=$((level * 10))
    for key in "${!KEY_MAP[@]}"; do
        if [[ -n "${KEY_MAP[$key]}" ]]; then
            echo "$brightness" > "${KB_BASE_PATH}${KEY_MAP[$key]}/brightness" &
        fi
    done
    wait
}

# Function to set lightbar color
set_lightbar_color() {
    local color="$1"
    echo "$color" > "${BAR_BASE_PATH}/multi_intensity"
}

# Function to set lightbar brightness
set_lightbar_brightness() {
    local level="$1"
    local brightness=$((level * 10))
    echo "$brightness" > "${BAR_BASE_PATH}/brightness"
}

# Read colors from file
if [[ -f "$COLOR_FILE" ]]; then
    read -r kb_r kb_g kb_b kb_brightness < "$COLOR_FILE"
    read -r bar_r bar_g bar_b bar_brightness < <(tail -n 1 "$COLOR_FILE")
    
    kb_color="$kb_r $kb_g $kb_b"
    bar_color="$bar_r $bar_g $bar_b"
    
    set_all_keys_color "$kb_color"
    set_all_keys_brightness "$kb_brightness"
    set_lightbar_color "$bar_color"
    set_lightbar_brightness "$bar_brightness"
else
    echo "Error: Color file not found at $COLOR_FILE"
    exit 1
fi
