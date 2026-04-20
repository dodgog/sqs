# Releasing

All local. No CI required.

## Prerequisites

```sh
cargo install cargo-release
```

## Release a new version

```sh
# Bump version, commit, tag, push
cargo release patch   # 0.3.1 → 0.3.2
cargo release minor   # 0.3.1 → 0.4.0
cargo release major   # 0.3.1 → 1.0.0

# Dry run first
cargo release patch --dry-run
```

This will:
1. Bump version in `Cargo.toml`
2. Commit with message `chore(release): X.Y.Z`
3. Create git tag `vX.Y.Z`
4. Push commit and tag

## Build release binary

```sh
cargo build --release
```

Binary at `target/release/sqs`.

## Install locally

```sh
cp target/release/sqs ~/.local/bin/sq
```

## Build with nix

```sh
nix build .
./result/bin/sqs --version
```

## Update flake.nix version

After `cargo release`, update the version in `flake.nix` to match:

```nix
version = "X.Y.Z";
```

Then commit: `git commit -am "chore: update flake.nix version"`
