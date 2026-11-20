# Changelog

All notable changes to this crate are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.10.0](https://github.com/Checkslip/sdk-rust/compare/v0.9.1...v0.10.0) (2026-11-20)


### Features

* carry the credit lots and the other fields the API already sends ([d577631](https://github.com/Checkslip/sdk-rust/commit/d577631efcfbd0689c98f4640f5ac13e196fcaf3))


### Bug Fixes

* decode the export manifest the API really sends ([833851c](https://github.com/Checkslip/sdk-rust/commit/833851cdc958e3f01fb2fd9e4c808de40a071b74))
* hand back a Retry-After this client will not wait out ([ec9d2a0](https://github.com/Checkslip/sdk-rust/commit/ec9d2a0d776574798cb6bc93af5f68d7246a5c86))

## [0.9.1](https://github.com/Checkslip/sdk-rust/compare/v0.9.0...v0.9.1) (2026-11-20)


### Bug Fixes

* read a balance that carries no threshold_basis ([05a4a6d](https://github.com/Checkslip/sdk-rust/commit/05a4a6d63e4b8eb34fe618c43e9da53b435ddf73))
* `Balance::threshold_basis` is now `Option<ThresholdBasis>`. The API stopped sending the field to customers, so it is always `None`; code that read it as a value needs a match or `as_ref()`. Reading a balance at all was failing with `Error::Decode` before this.

## [0.9.0](https://github.com/Checkslip/sdk-rust/compare/v0.8.2...v0.9.0) (2026-11-20)


### Features

* read the reason a lookup has no definite answer ([af6bdb6](https://github.com/Checkslip/sdk-rust/commit/af6bdb6fb03013a86e44a56669a9518c9cb20488))

## [0.8.2](https://github.com/Checkslip/sdk-rust/compare/v0.8.1...v0.8.2) (2026-11-17)


### Documentation

Wording only; no code or behaviour changed in this release.

* The `Slip` struct is documented as the transfer record the banking system
  returned. This is the type the lookup hands back, and its doc is what
  docs.rs serves.
* Why it matters: a lookup is answered through the banking system's sources,
  so naming the bank that sent the money as the one acting reads as though
  Checkslip contacts that bank itself. That bank is still named where it
  belongs, as the holder of the record.

## [0.8.1](https://github.com/Checkslip/sdk-rust/compare/v0.8.0...v0.8.1) (2026-11-17)


### Documentation

Wording only; no code or behaviour changed in this release.

* Every sentence that says where a lookup is answered now says "the banking
  system", because that is how a transfer is read: through the banking
  system's sources, not over a direct line to the bank that sent the money.
  That bank is still named, but only as the holder of the record. Corrected in
  README and the crate docs on docs.rs.
* The sender and receiver names and the last 4 account digits were promised
  without conditions. Those fields can be `null`, and a receiver that is not a
  bank account has no bank or account at all, so the docs now say "as far as
  the bank shows them". Code that compares a name should allow for a missing
  one.

## [0.8.0](https://github.com/Checkslip/sdk-rust/compare/v0.7.0...v0.8.0) (2026-11-14)


### Features

* Order::vat, Package::price_total and the two other VAT fields ([29a814a](https://github.com/Checkslip/sdk-rust/commit/29a814a5b54b3b6bbf77947d51328dc87a497df4))

## [0.7.0](https://github.com/Checkslip/sdk-rust/compare/v0.6.1...v0.7.0) (2026-11-14)


### Features

* BillingInfo can be built with new() and one method per field ([9fa5fd9](https://github.com/Checkslip/sdk-rust/commit/9fa5fd912ef527b186bbbd62ff048db02978f5e1))
* day(2026, 11, 1) builds a day without importing time types ([a4d6442](https://github.com/Checkslip/sdk-rust/commit/a4d644263b4fb98e57f205526d2ed8936b646890))


### Bug Fixes

* **docs:** the crate docs no longer promise a recheck and a webhook ([c9e98bb](https://github.com/Checkslip/sdk-rust/commit/c9e98bb3663c745f4f6549fea674a26503065190))
* **docs:** the Thai quick start no longer matches on PENDING ([cbbb082](https://github.com/Checkslip/sdk-rust/commit/cbbb0829b40810cc9dcecfe372d3bfd1963ea372))
* **docs:** the webhook docs stop describing the cancelled recheck ([0c72eca](https://github.com/Checkslip/sdk-rust/commit/0c72eca80e53e977382c9d3d060acbe4f8b2dd11))

## [0.6.1](https://github.com/Checkslip/sdk-rust/compare/v0.6.0...v0.6.1) (2026-11-14)


### Bug Fixes

* **docs:** PENDING is not returned; you get a 503 UNAVAILABLE error ([129ff12](https://github.com/Checkslip/sdk-rust/commit/129ff1235f55486ef7cd52e76842150e8b0e97bb))
* **docs:** the README usage example compiles with checkslip alone ([45f4eea](https://github.com/Checkslip/sdk-rust/commit/45f4eeadff57e05769eb6be202d63c5b4f562c1a))

## [0.6.0](https://github.com/Checkslip/sdk-rust/compare/v0.5.0...v0.6.0) (2026-11-12)


### ⚠ BREAKING CHANGES

* the reason on a restored account's credit entry used to read back as CreditReason::Unknown("ACCOUNT_RESTORED"). It now reads back as CreditReason::AccountRestored, so an arm that matched the unknown string stops firing for it.

### Features

* a credit entry's reason can say ACCOUNT_RESTORED ([233960e](https://github.com/Checkslip/sdk-rust/commit/233960e9cdd89cde7421981605772149b7979446))

## [0.5.0](https://github.com/Checkslip/sdk-rust/compare/v0.4.1...v0.5.0) (2026-11-11)


### ⚠ BREAKING CHANGES

* a STAFF endpoint used to read back as DisabledReason::Unknown("STAFF"). It now reads back as DisabledReason::Staff, so an arm that matched the unknown string stops firing for it.

### Features

* disabled_reason can say STAFF ([1c2678f](https://github.com/Checkslip/sdk-rust/commit/1c2678ffbe11dcc7cfd1fa4ddd030e175ad87434))

## [0.4.1](https://github.com/Checkslip/sdk-rust/compare/v0.4.0...v0.4.1) (2026-11-10)


### ⚠ BREAKING CHANGES

* The minimum supported Rust version moves from 1.75 to 1.80, to pick up the fix for three OpenSSL advisories: RUSTSEC-2026-0474, RUSTSEC-2026-0476 and RUSTSEC-2026-0477. They concern the OpenSSL that the optional native-tls feature pulls in, and the functions they name are not reachable from this crate or the crates it pulls in, so this is about not shipping a flagged dependency rather than a vulnerability reachable in your build. Upgrading keeps your own cargo-deny or cargo audit from flagging this crate. This goes out as a patch, 0.4.1, even though moving the minimum is a break for anyone on an older toolchain. On Rust 1.75 you can stay on 0.4.0, which has the same API, but only with the default rustls-tls: with native-tls, 0.4.0 keeps the OpenSSL this release patches.

### Bug Fixes

* the minimum supported Rust version is 1.80, for a patched OpenSSL ([b68f5dc](https://github.com/Checkslip/sdk-rust/commit/b68f5dcb73b5d5e76b29487207d4fa8e551bfb71))

## [0.4.0](https://github.com/Checkslip/sdk-rust/compare/v0.3.1...v0.4.0) (2026-11-10)


### ⚠ BREAKING CHANGES

* ListSlipsParams::free_reason and SlipExportRequest::free_reason are Vec<CustomerFreeReason>, the nine reasons a customer can receive, instead of Vec<FreeReason>. PENDING, NO_CREDIT and HOLD_EXPIRED are not among them, so code that passes one of those no longer compiles. The reason a lookup reports back is unchanged: still FreeReason, including FreeReason::Unknown for a value this version does not know. Counts still take the filter and still ignore it, so nothing changes there.
* UsageCounts no longer has free_no_credit or free_hold_expired. Both had been returning 0 on every account before this release, and the API has now removed them from UsageCounts. Code that reads either one no longer compiles, so delete those reads. A lookup that was not charged, with no reason given, counts in free_other.

### Features

* the free_reason filter takes the reasons a customer receives ([07da17f](https://github.com/Checkslip/sdk-rust/commit/07da17f61cd3fb773bb7d517a487c9b521c8d9f5))
* UsageCounts no longer has free_no_credit or free_hold_expired ([4cf0c71](https://github.com/Checkslip/sdk-rust/commit/4cf0c71e8b460d80929b617e8330277342ff5d23))


### Bug Fixes

* **docs:** install with cargo add, not a pinned "0.2" ([33d1c51](https://github.com/Checkslip/sdk-rust/commit/33d1c516c06fa2ea1b68acd1fdc6979d754755d0))

## [0.3.1](https://github.com/Checkslip/sdk-rust/compare/v0.3.0...v0.3.1) (2026-11-10)


### Bug Fixes

* **docs:** say what an exhaustive match really needs ([7f0866f](https://github.com/Checkslip/sdk-rust/commit/7f0866fa3abebee3258280e95893ea624502265d))

## [0.3.0](https://github.com/Checkslip/sdk-rust/compare/v0.2.1...v0.3.0) (2026-11-09)


### ⚠ BREAKING CHANGES

* **audit:** the four request filters take a list instead of a single value. A call that passed one value now passes a list of one.

### Features

* **audit:** take several values per request filter ([b4d4f62](https://github.com/Checkslip/sdk-rust/commit/b4d4f62c2de8702ba8dea31606fac96abdd90f3f))

## [0.2.1](https://github.com/Checkslip/sdk-rust/compare/v0.2.0...v0.2.1) (2026-11-09)


### Bug Fixes

* **docs:** correct the install version and the Thai key wording ([0576788](https://github.com/Checkslip/sdk-rust/commit/0576788c253070a31acada894ed2f10db0229457))

## [0.2.0](https://github.com/Checkslip/sdk-rust/compare/v0.1.0...v0.2.0) (2026-11-09)


### ⚠ BREAKING CHANGES

* **usage:** free_no_credit and free_hold_expired are deprecated and will disappear. The API no longer counts either reason, so both are always 0 and both fields will be removed from the response. They are optional from this version, so code that reads them has to handle them being absent. The counts they used to carry are in free_other.

### Features

* **usage:** add free_other and retire two free counts ([2dddaaa](https://github.com/Checkslip/sdk-rust/commit/2dddaaa85ffc1558a7947c59643846120505a7f2))


### Bug Fixes

* **ci:** give the artifact step the tag it uses ([7f2e6bf](https://github.com/Checkslip/sdk-rust/commit/7f2e6bf76859e92bccbd540ec4da0d77acb13eb4))
* **ci:** send a User-Agent when asking crates.io about a version ([ab74da6](https://github.com/Checkslip/sdk-rust/commit/ab74da6743b11f32ef0c008dcd876b764a4c856a))

## 0.1.0 (2026-11-09)


### ⚠ BREAKING CHANGES

* **api:** nest resources to match the shared surface

### Features

* add the async Checkslip API client ([f58c792](https://github.com/Checkslip/sdk-rust/commit/f58c7928f3a95fcbf162e0d30a8fcf6776a3356d))


### Bug Fixes

* **ci:** bump the version constant and stop CI cancelling itself ([0517e1d](https://github.com/Checkslip/sdk-rust/commit/0517e1d8222467da597f9bfce0d81cf29af3f740))
* **ci:** call publish from the release job ([8bb84cb](https://github.com/Checkslip/sdk-rust/commit/8bb84cbbce2bec7fc63fa7e3e198ccc845408bad))
* **ci:** check the release pull request before it is merged ([76fac45](https://github.com/Checkslip/sdk-rust/commit/76fac451d8331684564028b1855c2ba5bacef47f))
* **ci:** publish through the release workflow, never directly ([14863ba](https://github.com/Checkslip/sdk-rust/commit/14863bacb1ffe4b8c7d64af91259ae77c6de577a))
* **ci:** stop the publish job waiting for itself ([95c898e](https://github.com/Checkslip/sdk-rust/commit/95c898e24cee0e600c7ed84fb1f40421db264f24))
* **deps:** record why RUSTSEC-2026-0009 does not apply ([d612c5c](https://github.com/Checkslip/sdk-rust/commit/d612c5ccf4c46d49bd9d18cb10723b1da6ca57b0))
* **webhooks:** gate verify behind the webhooks feature ([377234d](https://github.com/Checkslip/sdk-rust/commit/377234db96127abd269b63069db8058d90700536))


### Code Refactoring

* **api:** nest resources to match the shared surface ([c2384d2](https://github.com/Checkslip/sdk-rust/commit/c2384d22114c842ca6ac0eec0fb5368b1b0d33e6))

## [Unreleased]

## [0.1.0] - 2026-11-09

### Added

- Async `Client` and `ClientBuilder` on reqwest, with rustls by default and
  `native-tls` as an option.
- Slip lookups by QR text or by image (`multipart/form-data`), lookup history,
  counts and CSV exports.
- Billing (balance, ledger, packages, orders, receipts, billing info), usage
  and supported banks.
- Webhook endpoints (create, list, get, update, delete, recover, rotate secret,
  send test), deliveries (list, get, redeliver) and event types.
- Audit events, the API request log and evidence exports.
- `Error` with `Api`, `RateLimited`, `Http`, `Decode` and `InvalidArgument`
  variants, built from RFC 9457 problem responses.
- Retries with exponential backoff and jitter: 429 after `Retry-After`;
  network errors and 5xx only for idempotent methods or requests that carry
  an `Idempotency-Key`.
- An `Idempotency-Key` on every lookup, generated when you do not set one.
- `list_all` methods that follow `next_cursor` and return a `Stream`.
- `webhooks` feature: Standard Webhooks signature checks and typed
  `slip.checked` / `webhook.test` payloads.
- `blocking` feature: a blocking client for code without an async runtime.
- Forward compatible enums with an `Unknown(String)` variant.

[Unreleased]: https://github.com/Checkslip/sdk-rust/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Checkslip/sdk-rust/releases/tag/v0.1.0
