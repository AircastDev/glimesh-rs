# glimesh-rs

![Crates.io](https://img.shields.io/crates/l/glimesh) ![Crates.io](https://img.shields.io/crates/v/glimesh)

A wrapper around [graphql_client](https://github.com/graphql-rust/graphql-client) for easier use with [Glimesh](https://glimesh.tv). This is currently a work in progress, and should be considered beta, but it is being used to power [Oaty](https://oaty.app) in production.

## Features

-   [x] Queries
-   [x] Mutations
-   [x] Http based connection
-   [ ] Subscriptions
-   [ ] Websocket based connection
-   [ ] Reconnect and resubscribe to subscriptions on socket failure
-   [ ] Batch subscriptions

## Example

More examples can be found in the `examples/` directory.

```rust
use glimesh::{http::Connection, Auth, Error};
use graphql_client::GraphQLQuery;
use std::env;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "examples/schema.json",
    query_path = "examples/user_details.graphql",
    response_derives = "Debug"
)]
pub struct UserDetailsQuery;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let client_id = env::var("CLIENT_ID").expect("Missing CLIENT_ID env var");

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

    Ok(())
}
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
