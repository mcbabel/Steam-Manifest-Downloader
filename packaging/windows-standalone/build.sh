#!/usr/bin/env bash
set -euo pipefail

usage() {
  echo "usage: $0 <app.exe> <version-label> <output-dir>" >&2
  exit 2
}
[ $# -eq 3 ] || usage

EXE=$1
LABEL=$2
OUT=$3
HERE=$(cd "$(dirname "$0")" && pwd)

STEAMLESS_VERSION=v3.1.0.5
STEAMLESS_ZIP="Steamless.${STEAMLESS_VERSION}.-.by.atom0s.zip"
STEAMLESS_URL="https://github.com/atom0s/Steamless/releases/download/${STEAMLESS_VERSION}/${STEAMLESS_ZIP}"
BYPASS_BASE="https://raw.githubusercontent.com/MCbabel/Steam-API-Check-Bypass/master/Release_dlls"
GBE_API="${GBE_API:-https://api.github.com/repos/Detanup01/gbe_fork/releases/latest}"
GBE_LICENSE_URL="https://raw.githubusercontent.com/Detanup01/gbe_fork/dev/LICENSE"
SHARPDISASM_LICENSE_URL="https://raw.githubusercontent.com/justinstenning/SharpDisasm/master/LICENSE.md"

NAME="Steam-Manifest-Downloader_${LABEL}_windows-standalone"
WORK=$(mktemp -d)
STAGE="$WORK/$NAME"
TP="$STAGE/third-party"
mkdir -p "$TP/gbe_fork" "$TP/steamless" "$TP/steam-api-check-bypass" "$OUT"

auth=()
if [ -n "${GH_TOKEN:-}" ]; then
  auth=(-H "Authorization: Bearer ${GH_TOKEN}")
fi

cp "$EXE" "$STAGE/Steam Manifest Downloader.exe"
cp "$HERE/ANLEITUNG.txt" "$HERE/README.txt" "$STAGE/"

curl -fsSL "${auth[@]}" -H "Accept: application/vnd.github+json" "$GBE_API" -o "$WORK/gbe_release.json"
python3 - "$WORK/gbe_release.json" "$WORK/gbe_pick.env" <<'PY'
import json, re, sys

release = json.load(open(sys.argv[1]))
assets = release["assets"]

def vs(name):
    m = re.search(r"vs(\d+)", name.lower())
    return int(m.group(1)) if m else None

def pick(keyword, ext, preferred):
    names = [a["name"] for a in assets]
    cands = [n for n in names
             if keyword in n.lower() and n.lower().endswith(ext)
             and "release" in n.lower() and "debug" not in n.lower()]
    with_vs = sorted((n for n in cands if vs(n) is not None), key=vs, reverse=True)
    if with_vs:
        return with_vs[0]
    if preferred in names:
        return preferred
    if cands:
        return cands[0]
    sys.exit(f"no {keyword} release asset in {release['tag_name']}: {names}")

win = pick("win", ".7z", "emu-win-release-vs26.7z")
linux = pick("linux", ".tar.bz2", "emu-linux-release.tar.bz2")
urls = {a["name"]: a["browser_download_url"] for a in assets}
with open(sys.argv[2], "w") as f:
    f.write(f"GBE_TAG={release['tag_name']}\n")
    f.write(f"GBE_PUBLISHED={release.get('published_at', '')}\n")
    f.write(f"GBE_WIN={win}\nGBE_WIN_URL={urls[win]}\n")
    f.write(f"GBE_LINUX={linux}\nGBE_LINUX_URL={urls[linux]}\n")
PY
. "$WORK/gbe_pick.env"

curl -fsSL "$GBE_WIN_URL" -o "$TP/gbe_fork/$GBE_WIN"
curl -fsSL "$GBE_LINUX_URL" -o "$TP/gbe_fork/$GBE_LINUX"
curl -fsSL "$GBE_LICENSE_URL" -o "$TP/gbe_fork/LICENSE"
cat > "$TP/gbe_fork/release.json" <<JSON
{"tag": "$GBE_TAG", "published_at": "$GBE_PUBLISHED", "windows": "$GBE_WIN", "linux": "$GBE_LINUX"}
JSON

curl -fsSL "$STEAMLESS_URL" -o "$TP/steamless/$STEAMLESS_ZIP"
cp "$HERE/steamless-LICENSE.txt" "$TP/steamless/LICENSE.txt"
curl -fsSL "$SHARPDISASM_LICENSE_URL" >> "$TP/steamless/LICENSE.txt"

for dll in SteamAPICheckBypass.dll SteamAPICheckBypass_x32.dll; do
  curl -fsSL "$BYPASS_BASE/$dll" -o "$TP/steam-api-check-bypass/$dll"
done

sed -e "s|@GBE_TAG@|$GBE_TAG|g" \
    -e "s|@GBE_WIN@|$GBE_WIN|g" \
    -e "s|@GBE_LINUX@|$GBE_LINUX|g" \
    -e "s|@STEAMLESS_VERSION@|$STEAMLESS_VERSION|g" \
    "$HERE/THIRD-PARTY-NOTICES.txt" > "$STAGE/THIRD-PARTY-NOTICES.txt"

(cd "$WORK" && zip -qr9 "$NAME.zip" "$NAME")
mv "$WORK/$NAME.zip" "$OUT/"
rm -rf "$WORK"
echo "$OUT/$NAME.zip"
