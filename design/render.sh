#!/bin/sh
# render.sh <page> <width> <height> [query] [out]: mockups/<page>.html → <out or page>.png at 2x
CHROME="/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
cd "$(dirname "$0")"
OUT="${5:-$1}"
"$CHROME" --headless=new --disable-gpu --hide-scrollbars --force-device-scale-factor="${SCALE:-2}" \
  --virtual-time-budget=4000 --window-size="$2,$3" --screenshot="$PWD/$OUT.png" "file://$PWD/mockups/$1.html${4:+?$4}" 2>/dev/null
