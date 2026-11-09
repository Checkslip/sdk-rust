# Changelog

All notable changes to this crate are listed here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the crate uses
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
