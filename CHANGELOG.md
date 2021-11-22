# Changelog

## 0.8.0 (2021-11-22)

### Added

-   Added `HybridConnection` that allows you to combine two connections into one, using one as the
    subscriptions connection, and the other as the query & mutations connection. This allows you to
    have a single client that, for example, supports subscriptions and the new glimesh api at the
    same time (as the websocket based api query currently only supports the old glimesh api)
-   Auth is now `Clone`

### Breaking Changes

-   `Auth::refreshable_access_token` now returns `impl Stream<Item = AccessToken>` instead of a
    tokio watch receiver.

## 0.7.0 (2021-10-26)

### Added

-   Added http_client method to the http connection builder to allow reuse of http clients.
-   Added auth method to the http connection builder.
-   Added clone_with_auth method to http connection to allow reusing shared data whilst specifying a
    different auth method.
-   Added ability to unsubscribe from subscriptions.
-   Subscriptions will now be automatically unsubscribed from in the background when dropped.

### Breaking Changes

-   Default api url for the http connection changed to the new api url
    `https://glimesh.tv/api/graph`. You can still use the old api url by using the http connection
    builder and calling `.api_url("https://glimesh.tv/api")` on it. Note that some queries may be
    different on the new url, so you should redownload the schema and check your queries still work.
-   Removed `.build(auth)` in favour of `.auth(auth).finish()` for
    `glimesh::http::ConnectionBuilder` as auth is now optional (although in practise the api will
    fail without some form of auth).
-   Removed error type from `Client` as it was redundant information.
-   Crate now uses rust edition 2021
-   The `SubscriptionConn` trait (and by extension the subscribe method on the client) now returns a
    `glimesh::Subscription` instead of a BoxStream, although it still implements Stream with the
    same Item. This allows for unsubscribing via `Subscription::unsubscribe` or when dropped. The
    lifetime on `SubscriptionConn` has also been removed as a result.

## v0.6.1 (2021-08-25)

### Added

-   Added `client_credentials_with_scopes` method to `Auth` to allow specifying scopes for the token.

## v0.6.0 (2021-07-05)

### Added

-   New crate `glimesh_protocol`
    -   Used to store lower level structures for interacting with the glimesh api.
    -   These are abstracted away from specific transport and are designed to be able to be consumed without
        using the main glimesh crate or it's choice of transport

### Breaking Changes

-   The glimesh_date submodule has been moved to the new glimesh_protocol package as `glimesh_protocol::date`
-   Updated `graphql_client` to 0.10

## v0.5.3 (2021-04-22)

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
