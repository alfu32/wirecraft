# Build and release

Wirecraft uses the same six-target distribution matrix and release workflow as
Meshcraft (`alfu32/meshcraft`), adapted to Wirecraft's two executables: the
`wirecraft-desktop` application, which also runs its own backend, and the
`wirecraft` command line (MCP, compile, validate, `wirecraft desktop`).

| Native target | Portable archives | Additional installers |
| --- | --- | --- |
| Linux x86_64 / aarch64 | ZIP, tar.gz | AppImage, Snap, DEB, RPM |
| Windows x64 / ARM64 | ZIP, tar.gz | MSI, signed MSIX, public signing CER |
| macOS Intel / Apple Silicon | ZIP, tar.gz | DMG containing Wirecraft.app |

Each release contains 28 primary artifacts, per-file SHA-256 sidecars and a combined
`SHA256SUMS.txt`. Portable archives include a file manifest, source revision,
version, licenses, notices and the WebServer examples. Fonts, archetypes and
compiler templates are embedded in the executables. User projects under
`examples/`, configuration and build directories are never collected: payload
files are listed explicitly.

`packaging/wirecraft.png` (drawn from `assets/wirecraft.svg`) is the icon of the
portable archives, AppImage, Snap, macOS ICNS and MSIX logos;
`packaging/windows/wirecraft.ico` is the MSI shortcut icon.

## Release a tag

1. Bump `[workspace.package] version` in `Cargo.toml`, update `Cargo.lock`
   (`cargo update --workspace`), commit, tag that exact commit `vX.Y.Z` and push
   both. Follow SemVer; the user owns major-version increments.
2. In GitHub Actions, choose **Release → Run workflow** and select that existing
   tag reference. With the CLI: `gh workflow run distribution.yaml --ref vX.Y.Z`.
3. The workflow rejects branches, a tag without the `v` prefix, a different
   workspace version or a tag pointing at another commit. Every job checks out the
   resolved SHA. The Rust toolchain is pinned by `RUST_TOOLCHAIN` in the workflow
   (not by a `rust-toolchain.toml`, which would also switch local rust-analyzer);
   builds use `--locked`.
4. Validation and all six native builds/installers run in parallel. Publication
   requires the complete artifact matrix, every checksum and matching source/file
   manifests. A failed platform prevents publication; the workflow never creates
   or moves a tag.

Only the final publication job has `contents: write`. Actions and AppImage inputs
are pinned to commits/digests (`packaging/appimage-inputs.json`, notices under
`packaging/licenses/appimage-runtime`).

## Platform notes

- **Linux** binaries are built in Ubuntu 18.04 (`packaging/linux/Dockerfile`) and
  must not need glibc newer than 2.27. `scripts/release/native_linux.py` bundles
  every non-driver ELF dependency of both executables, with their copyright
  notices. The `wirecraft` and `wirecraft-desktop` launchers in the payload root
  put system library directories first, so the compilers and language servers
  the backend starts keep their own libraries. DEB/RPM install to `/opt/wirecraft`
  with `/usr/bin/wirecraft` and `/usr/bin/wirecraft-desktop`.
- **Snap** (strict confinement): `wirecraft` is the command line and
  `wirecraft.desktop` starts the application. Host clang, clangd, rust-analyzer,
  Go and Node.js are not reachable from the Snap.
- **Windows** builds link the static CRT. The MSIX carries the reserved Microsoft
  Store identity (`K3D0.wirecraft`, publisher display name `K3D0`, in
  `packaging/windows/msix-identity.json`) and is signed with a self-signed
  certificate for that publisher created per run; its public CER ships for
  Trusted People installation, and the Store re-signs packages it accepts. The
  manifest has one application, `wirecraft-desktop.exe`, with the
  `wirecraft-desktop.exe` and `wirecraft.exe` execution aliases: Windows refuses
  to start an undeclared packaged executable, and the Store refuses an unlisted
  (headless) second application without a waiver. Release CI installs the MSIX,
  runs `--version` from the install location and through both aliases, and
  removes it. The MSI is unsigned.
- **macOS** bundles are ad-hoc signed, without Developer ID signing or
  notarization.

Native GUI behaviour on Windows, macOS and ARM64 has not been qualified; the
release checks start each installed executable with `--version`, and the Linux
x86_64/aarch64 AppImages must paint their window in a private Xvfb.

## Local checks

```sh
python3 -m unittest discover -s scripts/release -p 'test_*.py' -v
```

DEB/RPM tests are skipped unless `dpkg-deb`, `rpmbuild`, `rpm`, `rpm2cpio` and
`openssl` are installed.
