# Contributing

## Git hooks

Hooks in `.pre-commit-config.yaml` run `cargo fmt` and basic file checks (trailing
whitespace, final newlines, TOML/YAML syntax, merge markers, large files) on every commit.
Set them up once per clone with [prek](https://github.com/j178/prek):

```sh
cargo install --locked prek
prek install
```

The Python [pre-commit](https://pre-commit.com) tool reads the same file, so
`pre-commit install` works too.

When a hook changes a file, the commit stops. Review the change, `git add` it and commit
again. `prek run --all-files` runs every hook on the whole repository.

## Checks

CI runs these on every push and pull request:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

plus every combination of the Cargo features (with `cargo hack`), a build with the minimum
Rust version (1.92), a static musl release build, and
`cargo deny check` (see `deny.toml`).

`cargo test` also renders every fixture in [`tests/stress/`](tests/stress/README.md) through
the `mar` binary; `python3 tests/stress/generate.py` regenerates them.

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/). They decide the
changelog and the next version:

| Prefix | Changelog section | Version bump |
|---|---|---|
| `feat:` | Added | minor |
| `fix:` | Fixed | patch |
| `feat!:`, `fix!:` or a `BREAKING CHANGE:` footer | Breaking | major (minor before 1.0) |
| `docs:`, `refactor:`, `perf:`, `test:`, `build:`, `ci:`, `chore:` | Other or hidden | patch |

## Generated files

`assets/man/mar.1` and `assets/completions/*` are generated from the CLI definition in
`src/cli.rs`. After changing the CLI, regenerate them:

```sh
UPDATE_GENERATED=1 cargo test
```

`cargo test` fails while they are out of date.

## Releases

1. On every push to `main`, [release-plz](https://release-plz.dev) updates a release
   pull request that bumps the version and extends `CHANGELOG.md`. You can edit it before
   merging.
2. Merging it publishes the crate to crates.io and pushes the `vX.Y.Z` tag.
3. The tag starts `release.yml` ([dist](https://opensource.axo.dev/cargo-dist/)), which
   builds the Linux binaries, the shell installer, checksums and attestations, and creates
   the GitHub Release.

Required repository secrets:

| Secret | What |
|---|---|
| `RELEASE_PLZ_TOKEN` | Fine-grained PAT for this repo with Contents and Pull requests read/write |
| `CARGO_REGISTRY_TOKEN` | crates.io API token with the publish-new and publish-update scopes |

After changing `dist-workspace.toml`, run `dist generate` to update `release.yml`.
