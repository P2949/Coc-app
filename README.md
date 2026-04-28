# CoC7e Investigator Creator

A native desktop helper for building Call of Cthulhu 7th Edition investigators.

The app is written in Rust with `eframe`/`egui`. It focuses on rules-aware investigator creation: characteristics, age adjustments, occupation choices, skill allocations, derived stats, backstory fields, and a copyable summary.

## Current status

This is a desktop binary application, not a published Cargo library. The crate is marked `publish = false` and is intended to be distributed through GitHub Releases.

Official release binaries are distributed at no cost. The source code is free software under the GNU General Public License v3.0 or later.

## Requirements for local development

Install a current stable Rust toolchain:

```bash
rustup toolchain install stable
rustup default stable
```

On Linux, native GUI builds may require desktop/X11/Wayland development packages. On Debian/Ubuntu-like systems, install:

```bash
sudo apt-get update
sudo apt-get install -y \
  libgtk-3-dev \
  libxcb-render0-dev \
  libxcb-shape0-dev \
  libxcb-xfixes0-dev \
  libxkbcommon-dev
```

Package names differ by distribution.

## Build and test

```bash
cargo fmt --all -- --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --locked
```

Run locally:

```bash
cargo run --release
```

The release binary will be at:

```text
target/release/coc7e_investigator_creator
```

On Windows, the executable is:

```text
target\release\coc7e_investigator_creator.exe
```

## Installer packaging

Installer packaging is configured through `Packager.toml` and uses `cargo-packager`.

Install the packager locally with:

```bash
cargo install cargo-packager --locked
```

Then build installer packages for the current platform with:

```bash
cargo packager --release
```

By default, `cargo-packager` creates the platform-default package formats:

- Linux: `.deb`, `.AppImage`, and Pacman package output
- Windows: NSIS `.exe` installer
- macOS: `.app` bundle and `.dmg`

Generated packages are written under:

```text
dist/installers
```

## Release process

Releases are created from Git tags.

1. Make sure the working tree is clean.
2. Run the full local checks:

   ```bash
   cargo fmt --all -- --check
   cargo test --locked
   cargo clippy --all-targets --locked -- -D warnings
   cargo build --release --locked
   ```

3. Update both `Cargo.toml` and `Packager.toml` to the release version.
4. Create and push a version tag:

   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

5. The GitHub Actions release workflow will build raw Linux, Windows, and macOS archives, build installer packages, and attach all artifacts to a GitHub Release.

## Downloading release builds

After a tag build finishes, download the appropriate artifact from the repository's Releases page:

- Linux raw archive: `.tar.gz`
- Linux installers/packages: `.deb`, `.AppImage`, and Pacman package output
- Windows raw archive: `.zip`
- Windows installer: NSIS `.exe`
- macOS raw archive: `.tar.gz`
- macOS app bundle / installer: `.app` / `.dmg`

## License

CoC7e Investigator Creator is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or, at your option, any later version.

See [`LICENSE`](LICENSE) for the full license text.
