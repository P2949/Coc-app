# CoC7e Investigator Creator

A native desktop helper for building Call of Cthulhu 7th Edition investigators.

The app is written in Rust with `eframe`/`egui`. It focuses on rules-aware investigator creation: characteristics, age adjustments, occupation choices, skill allocations, derived stats, backstory fields, and a copyable summary.

## Current status

This is a desktop binary application, not a published Cargo library. The crate is marked `publish = false` and is intended to be distributed through GitHub Releases.

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

3. Create and push a version tag:

   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

4. The GitHub Actions release workflow will build Linux, Windows, and macOS artifacts and attach them to a GitHub Release.

## Downloading release builds

After a tag build finishes, download the appropriate artifact from the repository's Releases page:

- Linux: `.tar.gz`
- Windows: `.zip`
- macOS: `.tar.gz`

## License

No license file has been added yet. Before distributing public binaries broadly, choose and add a `LICENSE` file.
