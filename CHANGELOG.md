# Changelog

## Unreleased

### Added

-   Websocket based connection

### Breaking Changes

-   The `Error` type has been split up into more specific error types dependent on the connection used.
-   Client is now generic over the error type of query/mutate/subscribe
-   Auth related methods now use `AuthError`
-   Http connection method now use `HttpConnectionError`

## v0.2.0 (2021-03-23)

### Added

-   Client credentials authentication
-   Access token refreshing

### Breaking Changes

-   `AccessTokenWithRefresh` auth variant was renamed to `RefreshableAccessToken` and can now only be constructed with the `Auth::refreshable_access_token` method.
-   Added new error and auth variants

## v0.1.0 (2021-03-22)

### Added

-   Initial implementation with an HTTP based connnection.
