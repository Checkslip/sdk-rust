# Contributing

Thanks for helping. Bug reports and pull requests are welcome at
https://github.com/Checkslip/sdk-rust.

## Setup

You need a recent stable Rust. The crate also has to build on its minimum
supported version, 1.75, which CI checks on every push.

```sh
cargo test --all-features
cargo clippy --all-features --all-targets -- -D warnings
cargo fmt --check
cargo +1.75 check --all-features    # the MSRV gate
scripts/readme-outside-crate.sh     # the README, as a reader's project sees it
```

The README examples are compiled as doctests, so a rename that misses the
README fails the build. Those doctests compile inside the crate, where `time`
and `uuid` are dependencies that someone who ran `cargo add checkslip` does not
have, so the script above compiles the same blocks again with only the
dependencies the README asks a reader to install. CI runs it on every pull
request.

No test talks to the live API. They run against a local wiremock server and
recorded fixtures, so you need no API key and nothing is charged.

## What the tests expect

`tests/contract.rs` guards the surface the JavaScript, Go, PHP and Rust
clients share: the method names, one page versus every page, how list
envelopes unwrap, and that a 202 is a result rather than an error. Rust has no
runtime reflection, so the naming half is a compile-time check: a function
names every call, and the file stops compiling if one is missing, renamed or
reshaped. If you change the public surface, that test tells you, and the same
change belongs in the other three clients.

## Commit messages

Subjects follow [Conventional Commits](https://www.conventionalcommits.org)
and stay within 72 characters. CI checks every commit on a pull request.
