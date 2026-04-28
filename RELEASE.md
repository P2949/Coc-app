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

## Creating a release

Create a tag and push it:

```bash
git tag v0.1.0
git push origin v0.1.0
```

The `.github/workflows/release.yml` workflow builds release artifacts and creates a GitHub Release from the tag.

## Versioning

Update `Cargo.toml` before tagging a new release:

```toml
version = "0.1.0"
```

Use tags that match the version, for example:

```text
v0.1.0
```

## Notes

The release workflow uses host-native builds on GitHub-hosted runners. It does not cross-compile.
