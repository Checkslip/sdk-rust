# Releasing

Releases are driven by the commit messages on `main`. Nobody edits a version
number by hand.

## How it works

1. Every merge to `main` updates a release pull request, opened by
   release-please. It works out the next version from the Conventional Commits
   since the last release, writes `CHANGELOG.md` and bumps the version in
   `Cargo.toml` and `Cargo.lock`.
2. Merging that pull request tags `vX.Y.Z` and publishes a GitHub Release.
3. The release triggers `publish.yml`, which waits for the checks on that exact
   commit, checks the tag against `Cargo.toml`, refuses a version crates.io
   already has, runs the tests and `cargo package`, publishes with crates.io
   trusted publishing, and attaches the packaged crate to the release.

`feat:` gives a minor bump, `fix:` a patch, and a `!` or a `BREAKING CHANGE:`
footer a major one. Before 1.0.0 a breaking change bumps the minor.

## What a human has to do once

- **0.1.0 has to be published by hand**, because crates.io will only accept a
  trusted publisher for a crate that already exists. After that, add the
  trusted publisher for the `checkslip` crate on crates.io as:
  - repository: `Checkslip/sdk-rust`
  - workflow: `release-please.yml`
  - environment: `release`

  The workflow is **`release-please.yml`, not `publish.yml`**, even though the
  publish happens in `publish.yml`. The OIDC token's `workflow_ref` claim names
  the workflow that started the run, and `publish.yml` is a reusable workflow
  called by the release job. Naming the reusable file makes the registry reject
  the token. Once it is linked, the workflow takes over with no token stored
  anywhere.
- Allow GitHub Actions to create pull requests in the repository settings, or
  release-please cannot open its release pull request.
- A `release` environment gates the publish job, so a release can require a
  review before it goes out.

## Minimum supported Rust version

The crate builds on Rust 1.75, checked by CI on every push. Raising it is a
breaking change for anyone pinned to an older toolchain, so it belongs in a
release of its own with a note in the changelog.
