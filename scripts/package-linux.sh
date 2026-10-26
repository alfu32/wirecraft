#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
profile="${1:-release}"
case "$profile" in release) build_dir=release ;; dev) build_dir=debug ;; *) echo 'Usage: package-linux.sh [release|dev]' >&2; exit 2 ;; esac
if [[ "$(uname -s)" != Linux ]]; then echo 'This package script supports Linux only.' >&2; exit 2; fi
cargo build --workspace --locked --profile "$profile" -j "${WIRECRAFT_BUILD_JOBS:-2}"
version="$(cargo metadata --no-deps --format-version 1 | python3 -c 'import json,sys; print(json.load(sys.stdin)["packages"][0]["version"])')"
package="wirecraft-${version}-linux-$(uname -m)"
mkdir -p dist
stage="$(mktemp -d "$(pwd)/dist/.package-XXXXXXXX")"
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage/$package/bin" "$stage/$package/share/applications" "$stage/$package/share/icons/hicolor/scalable/apps" "$stage/$package/share/mime/packages" "$stage/$package/share/doc/wirecraft"
install -m755 "target/$build_dir/wirecraft" "target/$build_dir/wirecraft-desktop" "$stage/$package/bin/"
install -m644 packaging/wirecraft.desktop "$stage/$package/share/applications/"
install -m644 assets/wirecraft.svg "$stage/$package/share/icons/hicolor/scalable/apps/"
install -m644 packaging/wirecraft-mime.xml "$stage/$package/share/mime/packages/"
install -m644 assets/fonts/OFL.txt "$stage/$package/share/doc/wirecraft/Monaspace-OFL.txt"
install -m644 README.md "$stage/$package/share/doc/wirecraft/"
cp -R docs/migration "$stage/$package/share/doc/wirecraft/"
install -m755 scripts/install-linux.sh "$stage/$package/install.sh"
tar -C "$stage" -czf "dist/$package.tar.gz" "$package"
(cd dist && sha256sum "$package.tar.gz") > "dist/$package.tar.gz.sha256"
printf 'Created %s (%s profile)\n' "dist/$package.tar.gz" "$profile"
