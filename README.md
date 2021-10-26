# glimesh-rs

![Crates.io](https://img.shields.io/crates/l/glimesh) [![Crates.io](https://img.shields.io/crates/v/glimesh)](https://crates.io/crates/glimesh) [![Docs.rs](https://docs.rs/glimesh/badge.svg)](https://docs.rs/glimesh)

<!-- cargo-sync-readme start -->

A wrapper around [graphql_client](https://github.com/graphql-rust/graphql-client) for easier use with [Glimesh](https://glimesh.tv). This is currently a work in progress, and should be considered beta, but it is being used to power [Oaty](https://oaty.app) in production.

## Features

-   Queries
-   Mutations
-   Subscriptions
-   HTTP or Websocket connection
-   Automatic access token refreshing
-   Reconnect and resubscribe to subscriptions on socket failure

## Example

More examples can be found in the `glimesh/examples/` directory.

```rust
#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "examples/graphql/schema.json",
    query_path = "examples/graphql/user_details.graphql",
    response_derives = "Debug"
)]
pub struct UserDetailsQuery;

let auth = Auth::client_id(client_id);
let conn = Connection::new(auth);
let client = conn.into_client();

let res = client
    .query::<UserDetailsQuery>(
        user_details_query::Variables {
            username: "James".into(),
        }
    )
    .await?;

let user = res.user;
println!("User details: {:#?}", user);
```

## License

Licensed under either of

-   Apache License, Version 2.0
    ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
-   MIT license
    ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

<!-- cargo-sync-readme end -->
