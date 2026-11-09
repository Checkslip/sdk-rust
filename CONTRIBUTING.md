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
```

The README examples are compiled as doctests, so a rename that misses the
README fails the build.

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
