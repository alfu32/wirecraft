#!/usr/bin/env bash
set -euo pipefail
# Run from an extracted package. Explicit prefix supports isolated install checks.
package_root="$(cd "$(dirname "$0")" && pwd)"
prefix="${1:-${HOME}/.local}"
if [[ ! -x "$package_root/bin/wirecraft" || ! -x "$package_root/bin/wirecraft-desktop" ]]; then echo 'Run install.sh from an extracted Wirecraft package.' >&2; exit 2; fi
mkdir -p "$prefix/bin" "$prefix/share/applications" "$prefix/share/icons/hicolor/scalable/apps" "$prefix/share/mime/packages" "$prefix/share/doc/wirecraft"
install -m755 "$package_root/bin/wirecraft" "$package_root/bin/wirecraft-desktop" "$prefix/bin/"
install -m644 "$package_root/share/applications/wirecraft.desktop" "$prefix/share/applications/"
install -m644 "$package_root/share/icons/hicolor/scalable/apps/wirecraft.svg" "$prefix/share/icons/hicolor/scalable/apps/"
install -m644 "$package_root/share/mime/packages/wirecraft-mime.xml" "$prefix/share/mime/packages/"
cp -R "$package_root/share/doc/wirecraft/." "$prefix/share/doc/wirecraft/"
# Do not change file associations or other applications' settings implicitly.
if command -v update-desktop-database >/dev/null; then update-desktop-database "$prefix/share/applications"; fi
if command -v update-mime-database >/dev/null; then update-mime-database "$prefix/share/mime"; fi
printf 'Installed Wirecraft to %s; include %s/bin in PATH.\n' "$prefix" "$prefix"
