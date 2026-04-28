# Release checklist

Use this checklist before creating a public binary release.

## Local verification

Run:

```bash
cargo fmt --all -- --check
cargo test --locked
cargo clippy --all-targets --locked -- -D warnings
cargo build --release --locked
```

Then manually smoke-test the app:

```bash
cargo run --release
```

Check at least:

- the app opens successfully;
- the window can be resized and scrolled;
- characteristics can be rolled and edited;
- age controls update derived stats;
- occupation choices unlock skills correctly;
- skill allocation warnings display correctly;
- summary text copies to the clipboard.

## Installer packaging smoke test

Install `cargo-packager` if needed:

```bash
cargo install cargo-packager --locked
```

Build installer packages for the current platform:

```bash
cargo packager --release
```

Check that package files are created under:

```text
dist/installers
```

Expected platform-default packages are:

- Linux: `.deb`, `.AppImage`, and Pacman package output
- Windows: NSIS `.exe` installer
- macOS: `.app` bundle and `.dmg`

For public releases, install or launch at least one generated package on the target platform before tagging.

## Creating a release

Update the release version in both files:

```text
Cargo.toml
Packager.toml
```

Then create a tag and push it:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The `.github/workflows/release.yml` workflow builds raw release archives, builds installer packages with `cargo-packager`, and creates or updates a GitHub Release from the tag.

## Versioning

Update `Cargo.toml` before tagging a new release:

```toml
version = "0.1.0"
```

Update `Packager.toml` to the same version:

```toml
version = "0.1.0"
```

Use tags that match the version, for example:

```text
v0.1.0
```

## License

The project is licensed as GPL-3.0-or-later. Keep the `LICENSE` file included in every source release and packaged artifact.

## Notes

The release workflow uses host-native builds on GitHub-hosted runners. It does not cross-compile.
