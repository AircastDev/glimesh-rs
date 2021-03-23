# Changelog

## Unreleased

### Added

-   Support for client credentials authentication

### Breaking Changes

-   `AccessTokenWithRefresh` auth variant was renamed to `RefreshableAccessToken` and can now only be constructed with the `Auth::refreshable_access_token` method.

## v0.1.0 (2021-03-22)

### Added

-   Initial implementation with an HTTP based connnection.
