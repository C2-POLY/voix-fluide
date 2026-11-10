#!/usr/bin/env bash
# Builds Voix Fluide.app and Voix-Fluide.dmg (Apple Silicon, ad hoc signed) into
# target/release-bundle/. Runs the same locally and in the CI (release.yml).
#   scripts/bundle.sh            cargo build --release, then the bundle and the dmg
#   scripts/bundle.sh --no-build use target/release/voix-fluide as it is
# The CI sets VF_RELEASE=1 so build.rs marks the binary as a release build.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
DMGBUILD_VERSION="1.6.7"
BUILD=1

case "${1:-}" in
  "") ;;
  --no-build) BUILD=0 ;;
  *) echo "usage: $0 [--no-build]" >&2; exit 1 ;;
esac

cd "$ROOT"

VERSION="$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -n 1)"
if [ -z "$VERSION" ]; then
  echo "error: no version in Cargo.toml" >&2
  exit 1
fi
echo "Voix Fluide $VERSION"

if [ "$BUILD" -eq 1 ]; then
  cargo build --release --locked
fi
BIN="$ROOT/target/release/voix-fluide"
[ -x "$BIN" ] || { echo "error: $BIN is missing; build it first" >&2; exit 1; }

OUT="$ROOT/target/release-bundle"
BUNDLE="$OUT/Voix Fluide.app"
DMG="$OUT/Voix-Fluide.dmg"

rm -rf "$OUT"
mkdir -p "$BUNDLE/Contents/MacOS" "$BUNDLE/Contents/Resources"
cp "$BIN" "$BUNDLE/Contents/MacOS/voix-fluide"

# The plist, with the version from Cargo.toml.
cp "$ROOT/assets/Info.plist" "$BUNDLE/Contents/Info.plist"
plutil -replace CFBundleShortVersionString -string "$VERSION" "$BUNDLE/Contents/Info.plist"
plutil -replace CFBundleVersion -string "$VERSION" "$BUNDLE/Contents/Info.plist"

# icon.icns from the icon the app draws itself (src/icon.rs), at every size iconutil wants.
ICONSET="$OUT/icon.iconset"
mkdir -p "$ICONSET"
for s in 16 32 128 256 512; do
  "$BIN" --icon-png "$s" "$ICONSET/icon_${s}x${s}.png"
  "$BIN" --icon-png "$((s * 2))" "$ICONSET/icon_${s}x${s}@2x.png"
done
iconutil -c icns "$ICONSET" -o "$BUNDLE/Contents/Resources/icon.icns"
rm -rf "$ICONSET"

# Ad hoc signature: no Developer ID, so no notarization. At launch the app copies its
# binary into Application Support and signs that copy with the keychain's certificate.
codesign --force --sign - --timestamp=none "$BUNDLE"
codesign --verify --strict "$BUNDLE"

# The disk image: the app and a link to /Applications, laid out on the background
# of assets/dmg (see dmg-settings.py). dmgbuild lives in its own venv under target/.
VENV="$ROOT/target/dmgbuild"
if ! "$VENV/bin/python" -c "import importlib.metadata as m, sys; sys.exit(m.version('dmgbuild') != '$DMGBUILD_VERSION')" 2>/dev/null; then
  python3 -m venv "$VENV"
  "$VENV/bin/pip" install --quiet --disable-pip-version-check "dmgbuild==$DMGBUILD_VERSION"
fi
"$VENV/bin/dmgbuild" -s "$SCRIPT_DIR/dmg-settings.py" \
  -D app="$BUNDLE" -D assets="$ROOT/assets" \
  "Voix Fluide" "$DMG" >/dev/null 2> >(grep -v 'is deprecated' >&2)

echo "Built:"
echo "  $BUNDLE"
echo "  $DMG"
