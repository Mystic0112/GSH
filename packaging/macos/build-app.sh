#!/usr/bin/env bash
# Monta o GSH.app (e opcionalmente o DMG) a partir dos binários em Rust.
#
#   packaging/macos/build-app.sh [--sign IDENTIDADE] [--dmg] [--skip-build]
#
# Sem --sign usa assinatura ad-hoc ("-"): roda neste Mac e em quem compila, mas o
# Gatekeeper bloqueia a primeira abertura em outro Mac. Com uma identidade
# "Developer ID Application: ..." assina com hardened runtime e timestamp, o que
# a notarização exige (ver README.md).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HERE="$ROOT/packaging/macos"
OUT="${OUT_DIR:-$ROOT/target/macos}"
APP="$OUT/GSH.app"
BUNDLE_ID="${BUNDLE_ID:-work.gorila.gsh}"
IDENTITY="-"
MAKE_DMG=0
BUILD=1

while [ $# -gt 0 ]; do
  case "$1" in
    --sign) IDENTITY="${2:?falta a identidade}"; shift 2 ;;
    --dmg) MAKE_DMG=1; shift ;;
    --skip-build) BUILD=0; shift ;;
    -h|--help) sed -n '2,10p' "$0"; exit 0 ;;
    *) echo "argumento desconhecido: $1" >&2; exit 2 ;;
  esac
done

[ "$(uname -s)" = "Darwin" ] || { echo "só roda no macOS" >&2; exit 1; }

VERSION="$(sed -n '/^\[workspace.package\]/,/^\[/{s/^version *= *"\(.*\)"/\1/p;}' "$ROOT/Cargo.toml" | head -1)"
[ -n "$VERSION" ] || { echo "versão não encontrada em Cargo.toml" >&2; exit 1; }
BUILD_NUMBER="${BUILD_NUMBER:-$(git -C "$ROOT" rev-list --count HEAD 2>/dev/null || echo 1)}"

if [ "$BUILD" = 1 ]; then
  (cd "$ROOT" && cargo build --release -p gsh-app -p gsh-cli)
fi
APP_BIN="$ROOT/target/release/gsh-app"
CLI_BIN="$ROOT/target/release/gsh"
[ -x "$APP_BIN" ] && [ -x "$CLI_BIN" ] || { echo "binários não encontrados em target/release" >&2; exit 1; }

# Versão mínima do macOS lida do próprio binário, para o plist não divergir dele.
MIN_OS="$(otool -l "$APP_BIN" | awk '/LC_BUILD_VERSION/{f=1} f&&/minos/{print $2; exit}')"
MIN_OS="${MIN_OS:-11.0}"

rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources/bin"
# O executável do app não pode se chamar `gsh`: o macOS é case-insensitive e
# colidiria com a CLI, que vai em Resources/bin.
cp "$APP_BIN" "$APP/Contents/MacOS/GSH"
cp "$CLI_BIN" "$APP/Contents/Resources/bin/gsh"

sed -e "s|@BUNDLE_ID@|$BUNDLE_ID|g" \
    -e "s|@VERSION@|$VERSION|g" \
    -e "s|@BUILD@|$BUILD_NUMBER|g" \
    -e "s|@MIN_OS@|$MIN_OS|g" \
    "$HERE/Info.plist.in" > "$APP/Contents/Info.plist"
plutil -lint "$APP/Contents/Info.plist" >/dev/null

# Ícone: um PNG 1024 mestre e o iconset gerado com `sips`.
ICONSET="$(mktemp -d)/AppIcon.iconset"
mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
  sips -z "$size" "$size" "$HERE/AppIcon-1024.png" --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
  sips -z "$((size * 2))" "$((size * 2))" "$HERE/AppIcon-1024.png" --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"

if [ "$IDENTITY" = "-" ]; then
  codesign --force --deep --sign - "$APP/Contents/Resources/bin/gsh" "$APP"
else
  codesign --force --options runtime --timestamp --sign "$IDENTITY" "$APP/Contents/Resources/bin/gsh"
  codesign --force --options runtime --timestamp --sign "$IDENTITY" "$APP"
fi
codesign --verify --strict --verbose=2 "$APP" 2>&1 | tail -2

echo "GSH.app $VERSION ($BUILD_NUMBER), macOS >= $MIN_OS: $APP"

if [ "$MAKE_DMG" = 1 ]; then
  DMG="$OUT/GSH-$VERSION-macos-$(uname -m).dmg"
  STAGE="$(mktemp -d)"
  cp -R "$APP" "$STAGE/GSH.app"
  ln -s /Applications "$STAGE/Applications"
  rm -f "$DMG"
  hdiutil create -volname "GSH" -srcfolder "$STAGE" -ov -format UDZO "$DMG" >/dev/null
  [ "$IDENTITY" = "-" ] || codesign --force --timestamp --sign "$IDENTITY" "$DMG"
  echo "DMG: $DMG"
fi
