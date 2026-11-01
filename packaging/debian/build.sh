#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
output=$(realpath -m "${1:-$root/tmp/debian-artifacts}")
. /etc/os-release
case "$ID:$VERSION_ID" in
    ubuntu:24.04) distribution=ubuntu24.04 ;;
    ubuntu:26.04) distribution=ubuntu26.04 ;;
    debian:13) distribution=debian13 ;;
    *) echo "Build inside Ubuntu 24.04, Ubuntu 26.04 or Debian 13." >&2; exit 1 ;;
esac
if [ "$(dpkg --print-architecture)" != amd64 ]; then
    echo "Initial release packages target native amd64 builds." >&2
    exit 1
fi
version=$(sed -n 's/^version = "\([^"]*\)"$/\1/p' "$root/Cargo.toml")
if [[ "$version" =~ ^([0-9]+\.[0-9]+\.[0-9]+)-beta\.([1-9][0-9]*)$ ]]; then
    deb_version="${BASH_REMATCH[1]}~beta${BASH_REMATCH[2]}"
elif [[ "$version" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    deb_version="$version"
else
    echo "Cannot determine the workspace release version." >&2
    exit 1
fi
stage=$(mktemp -d "${TMPDIR:-/tmp}/lianli-deb.XXXXXX")
mkdir -p "$stage/source" "$output"
rsync -a --exclude=.git --exclude=target --exclude=tmp --exclude=node_modules \
    --exclude=dist --exclude=.cache --exclude=.codex --exclude=.agents \
    --exclude='.env*' --exclude=.npmrc --exclude='.cargo/credentials*' \
    --exclude='*.deb' --exclude='*.rpm' \
    --exclude=packaging/archlinux \
    "$root/" "$stage/source/"
cd "$stage/source"
export DEBFULLNAME=sgtaziz DEBEMAIL=sgtaziz013@gmail.com
dch --newversion "$deb_version+$distribution" --distribution "$VERSION_CODENAME" \
    --force-distribution "Build for $ID $VERSION_ID."
dpkg-buildpackage --build=binary --no-sign
bash packaging/debian/check-package.sh "$stage/lian-li-linux_${deb_version}+${distribution}_amd64.deb"
cp "$stage"/*.deb "$stage"/*.buildinfo "$stage"/*.changes "$output/"
printf 'Build tree retained for inspection: %s\n' "$stage"
