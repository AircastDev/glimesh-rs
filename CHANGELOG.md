# Changelog

## Unreleased

### Fixed

-   Stop retrying subscriptions if the socket dies whilst doing so (#2)

## v0.5.2 (2021-04-21)

### Fixed

-   Retry resubscriptions as long as the socket is connected [#1](https://github.com/AircastDev/glimesh-rs/issues/1)

## v0.5.1 (2021-04-16)

### Added

-   Added method `into_connection` to turn a client back into it's underlying connection

### Fixed

-   Include `AuthError` type even when http feature is disabled

## v0.5.0 (2021-03-26)

### Added

-   Implement `Clone` + `Debug` for Client where the connection does also
-   Expose `glimesh_date` module

### Breaking Changes

-   `SubscriptionConn` no longer binds the returned stream to the lifetime of self.
-   `Client` updated to reflect the above change

## v0.4.2 (2021-03-26)

### Added

-   Type aliases for websocket & http clients.

### Fixed

-   Export `AccessToken` struct

## v0.4.1 (2021-03-25)

### Fixed

-   Require async-trait v0.1.43 or above as versions below don't work with the subscription trait due to lifetime issues.

## v0.4.0 (2021-03-25)

### Added

-   Subscription support on the websocket connection

### Breaking Changes

-   Subscription trait was changed to returning a boxed stream instead of taking a channel sender.
-   `subscribe_with_opts` was removed as its no longer needed

## v0.3.0 (2021-03-25)

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
