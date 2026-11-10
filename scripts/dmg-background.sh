#!/usr/bin/env bash
# Renders assets/dmg/background.svg into the two images the .dmg window shows,
# background.png (660x460) and background@2x.png (1320x920), with Chrome headless.
# Run it after changing the SVG, then commit the two PNGs: bundle.sh uses them as they are.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DIR="$(cd "$SCRIPT_DIR/../assets/dmg" && pwd)"
CHROME="${CHROME:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"

if [ ! -x "$CHROME" ]; then
  echo "error: Chrome not found; set CHROME to its binary" >&2
  exit 1
fi

render() {
  "$CHROME" --headless=new --disable-gpu --hide-scrollbars \
    --force-device-scale-factor="$1" --window-size=660,460 \
    --screenshot="$DIR/$2" "file://$DIR/background.svg" 2>/dev/null
}

render 1 background.png
render 2 background@2x.png
echo "Rendered $DIR/background.png and background@2x.png"
