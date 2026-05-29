# CoC7e Investigator Creator

A native desktop helper for building Call of Cthulhu 7th Edition investigators.

This is an unofficial fan-made character creation helper. It is not affiliated with, sponsored by, or endorsed by Chaosium Inc.

The app is written in Rust with `eframe`/`egui`. It focuses on rules-aware investigator creation: characteristics, age adjustments, occupation choices, skill allocations, derived stats, backstory fields, JSON save/load for editable investigators, file-path based save/load helpers, and a copyable summary.

## Current status

This is a desktop binary application, not a published Cargo library. The crate is marked `publish = false` and is intended to be distributed through GitHub Releases.

Official release binaries are distributed at no cost. The source code is free software under the GNU General Public License v3.0 or later. Early releases should still be treated as technical builds until their target-platform installers have been smoke-tested and, on Windows/macOS, signed or notarized.

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


## Save compatibility and custom skills

Save files include a numeric schema version. Missing-version JSON is treated as a legacy v0 save, version 1 JSON is migrated into the current version 2 schema, and future unsupported versions are rejected instead of being partially loaded. Imports still sanitize invalid data, but the UI reports when allocations, custom-skill labels, Luck evidence, or other imported fields had to be corrected.

Custom occupations default to the standard eight occupation-skill slots, but the required count can be lowered for Keeper-approved custom or simplified occupations. Custom occupation skill slots may also have independent display labels such as `Language (Latin)`, `Language (Greek)`, `Pilot (Boat)`, or `Survival (Desert)` while keeping the underlying canonical rule skill for base values. Duplicate specialty slots are tracked as separate sheet rows and keep independent custom-slot allocation points. Temporarily lowering the custom occupation skill count preserves inactive valid slot names and clamped inactive slot allocations so they can be restored when the count is raised again.

Dice rolls use the app's saved RNG seed and bounded roll-side history as convenience roll evidence for character creation, not as cryptographic randomness. When that bounded history fills, the app reseeds from the live RNG stream and starts a fresh bounded history so future saves still restore the next roll position. Stored characteristic, Luck, and EDU roll results remain the authoritative audit trail in the JSON save.

## Installer packaging

Installer packaging is configured through `Packager.toml` and uses `cargo-packager`.

Install the packager locally with:

```bash
cargo install cargo-packager --version 0.11.2 --locked
```

Then build installer packages for the current platform with:

```bash
cargo packager --release
```

By default, `cargo-packager` creates the platform-default package formats:

- Linux: `.deb`, `.AppImage`, and Pacman package output
- Windows: NSIS `.exe` installer
- macOS: `.app` bundle and `.dmg` locally; GitHub Releases archive `.app` bundles as `.app.tar.gz`

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
4. For a first packaging pass, push a release-candidate tag:

   ```bash
   git tag v0.1.0-rc1
   git push origin v0.1.0-rc1
   ```

5. For the final release, create and push a version tag:

   ```bash
   git tag v0.1.0
   git push origin v0.1.0
   ```

6. The GitHub Actions release workflow will build raw Linux, Windows, and macOS archives, build installer packages, generate `SHA256SUMS.txt`, generate artifact attestations, and attach all artifacts to a GitHub Release.

## Downloading release builds

After a tag build finishes, download the appropriate artifact from the repository's Releases page:

- Linux raw archive: `.tar.gz`
- Linux installers/packages: `.deb`, `.AppImage`, and Pacman package output
- Windows raw archive: `.zip`
- Windows installer: NSIS `.exe`
- macOS raw archive: `.tar.gz`
- macOS app bundle archive / installer: `.app.tar.gz` / `.dmg`
- Checksums: `SHA256SUMS.txt`

The Linux `.tar.gz` and `.AppImage` are the easiest artifacts to verify on non-Debian Linux systems. `.deb`, Windows, and macOS packages are generated by CI and should be considered experimental until they are tested on their target platforms.

Verify downloaded artifact checksums with:

```bash
sha256sum -c SHA256SUMS.txt
```

Verify GitHub artifact attestations with:

```bash
gh attestation verify <downloaded-artifact> --repo P2949/Coc-app
```

## Code signing status

Release artifacts are currently unsigned. This is acceptable for early technical releases, but users may see warnings on Windows and macOS.

Future release hardening should add Windows Authenticode signing and macOS Developer ID signing/notarization before presenting those installers as polished end-user packages.

## License

CoC7e Investigator Creator is free software: you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, either version 3 of the License, or, at your option, any later version.

See [`LICENSE`](LICENSE) for the full license text.
