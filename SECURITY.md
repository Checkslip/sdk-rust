# Security policy

## Reporting a problem

Email security@checkslip.com. Please do not open a public GitHub issue for
anything that could put Checkslip users at risk.

Include what you found, the crate version, and the steps or code that show
it. We reply within three working days and keep you posted until it is fixed.

If the report involves an API key or a webhook signing secret, do not send the
real value. Revoke it in the dashboard and tell us which key it was by its ID.

## Supported versions

Security fixes go into the latest 0.x release.

## Handling secrets with this crate

- The client never logs your API key. `Debug` output of `Client`,
  `ClientBuilder`, `SigningSecret` and `webhook::Webhook` shows a redacted value.
- Pass keys and webhook secrets in through the environment or a secret store.
  Do not commit them.
- Check webhook signatures on the raw request body and refuse messages that
  fail, using `checkslip::webhook::Webhook::verify`.
