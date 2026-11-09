# Changelog

All notable changes to this crate are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
