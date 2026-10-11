#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 <version> <setup.exe> <standalone.zip> <output-dir>" >&2
  exit 2
}
[ $# -eq 4 ] || usage

VERSION=${1#v}
SETUP=$2
ZIP=$3
OUT=$4
HERE=$(cd "$(dirname "$0")" && pwd)
REPO="https://github.com/MCbabel/Steam-Manifest-Downloader"
BASE="$REPO/releases/download/v$VERSION"

asset_name() {
  basename "$1" | tr ' ' '.'
}

SETUP_URL="$BASE/$(asset_name "$SETUP")"
ZIP_URL="$BASE/$(asset_name "$ZIP")"
SETUP_SHA256=$(sha256sum "$SETUP" | cut -d' ' -f1 | tr 'a-f' 'A-F')
ZIP_SHA256=$(sha256sum "$ZIP" | cut -d' ' -f1)
DATE=$(date -u +%Y-%m-%d)

fill() {
  sed -e "s|@VERSION@|$VERSION|g" \
      -e "s|@DATE@|$DATE|g" \
      -e "s|@SETUP_URL@|$SETUP_URL|g" \
      -e "s|@SETUP_SHA256@|$SETUP_SHA256|g" \
      -e "s|@ZIP_URL@|$ZIP_URL|g" \
      -e "s|@ZIP_SHA256@|$ZIP_SHA256|g" \
      -e "s|@RELEASE_URL@|$REPO/releases/tag/v$VERSION|g" \
      "$1"
}

WORK=$(mktemp -d)
WINGET="$WORK/manifests/m/MCbabel/SteamManifestDownloader/$VERSION"
mkdir -p "$WINGET" "$OUT"
for f in "$HERE"/winget/*.yaml; do
  fill "$f" > "$WINGET/$(basename "$f")"
done
(cd "$WORK" && zip -qr9 "winget-manifests_${VERSION}.zip" manifests)
mv "$WORK/winget-manifests_${VERSION}.zip" "$OUT/"
fill "$HERE/scoop/steam-manifest-downloader.json" > "$OUT/steam-manifest-downloader.json"
rm -rf "$WORK"
ls "$OUT"
