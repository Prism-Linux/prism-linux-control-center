#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
out=$(realpath -m "${1:-$root/../prism-linux-control-center-rc-1.3.0-beta.1}")
source_dir="$out/source"

if [ -e "$out" ]; then
    echo "Refusing to overwrite an existing candidate directory: $out" >&2
    exit 1
fi
mkdir -p "$source_dir" "$out/artifacts" "$out/checksums"

rsync -a --exclude=.git --exclude=target --exclude=tmp --exclude=node_modules \
    --exclude=dist --exclude=.cache --exclude=.codex --exclude=.agents \
    --exclude='.env*' --exclude=.npmrc --exclude='.cargo/credentials*' \
    --exclude='*.deb' --exclude='*.rpm' --exclude='*.pkg.tar.*' \
    --exclude='*.zip' --exclude='*.jsonl' \
    "$root/" "$source_dir/"

rm -rf "$source_dir/docs"
mkdir -p "$source_dir/docs/releases"
for guide in \
    building-from-source.md debian-packages.md desktop-backends.md distrobox.md \
    hardware-compatibility.md hardware-video.md installation.md known-limitations.md \
    lcd-assets.md licensing.md openrgb.md pixel-conditioning.md service-modes.md \
    startup-images.md state-backups.md troubleshooting.md usb-permissions.md; do
    install -m644 "$root/docs/$guide" "$source_dir/docs/$guide"
done
install -m644 "$root/docs/releases/v1.3.0-beta.1.md" "$source_dir/docs/releases/"

# The generated working changelog includes private branch history; public
# release notes are the curated release document above.
rm -f "$source_dir/CHANGELOG.md"

# This community template contains uncleared franchise artwork and is not
# included in the Prism distribution candidate.
rm -rf "$source_dir/templates/assets/rx78-ice"
node - "$source_dir/templates/default_templates.json" <<'NODE'
const fs = require("node:fs");
const filename = process.argv[2];
const catalog = JSON.parse(fs.readFileSync(filename, "utf8"));
catalog.templates = catalog.templates.filter((template) => template.id !== "rx78-ice");
fs.writeFileSync(filename, `${JSON.stringify(catalog, null, 2)}\n`);
NODE

# Replace account-specific screenshot fixtures with generic paths in the
# public snapshot while leaving the owner's working tree untouched.
for fixture in \
    "$source_dir/crates/lianli-gui/tests/managed-media-import.test.mjs" \
    "$source_dir/crates/lianli-gui/tests/plain-data.test.mjs"; do
    sed -i -E 's#/home/[^/]+/Pictures/J[a]rvis\.png#/home/tester/Pictures/sample-display.png#g; s#/home/[^/]+/Pictures/new-panel\.png#/home/tester/Pictures/new-panel.png#g' "$fixture"
done

if find "$source_dir/docs" -type f \( -iname 'phase*' -o -iname '*windows*evidence*' \) -print -quit | grep -q .; then
    echo "Publication-only material remains in the candidate source." >&2
    exit 1
fi
for asset_dir in "$source_dir"/templates/assets/*; do
    name=${asset_dir##*/}
    if [[ "$name" =~ ^[Rr][Xx][0-9] ]]; then
        echo "An uncleared template remains in the candidate source." >&2
        exit 1
    fi
done

git -C "$root" rev-parse HEAD > "$out/source-commit.txt"
git -C "$root" merge-base HEAD main > "$out/upstream-base.txt"
(cd "$source_dir" && find . -type f -print0 | sort -z | xargs -0 sha256sum) > "$out/checksums/SOURCE-SHA256SUMS"

archive="$out/artifacts/prism-linux-control-center-1.3.0-beta.1.tar.gz"
tar --sort=name --mtime='@0' --owner=0 --group=0 --numeric-owner \
    --transform='s,^source,prism-linux-control-center-1.3.0-beta.1,' \
    -C "$out" -cf - source | gzip -n > "$archive"
(cd "$out/artifacts" && sha256sum "$(basename "$archive")" > SHA256SUMS)
printf 'Prepared sanitized source tree: %s\n' "$source_dir"
printf 'Prepared source archive: %s\n' "$archive"
