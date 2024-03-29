//! A wrapper around [graphql_client](https://github.com/graphql-rust/graphql-client) for easier use with [Glimesh](https://glimesh.tv). This is currently a work in progress, and should be considered beta, but it is being used to power [Oaty](https://oaty.app) in production.
//!
//! ## Features
//!
//! -   Queries
//! -   Mutations
//! -   Subscriptions
//! -   HTTP or Websocket connection
//! -   Automatic access token refreshing
//! -   Reconnect and resubscribe to subscriptions on socket failure
//!
//! ## Example
//!
//! More examples can be found in the `examples/` directory.
//!
//! ```rust
//! # use glimesh::{http::Connection, Auth, HttpConnectionError};
//! # use graphql_client::GraphQLQuery;
//! # use std::env;
//! #
//! #[derive(GraphQLQuery)]
//! #[graphql(
//!     schema_path = "examples/graphql/schema.json",
//!     query_path = "examples/graphql/user_details.graphql",
//!     response_derives = "Debug"
//! )]
//! pub struct UserDetailsQuery;
//! #
//! # #[tokio::main]
//! # async fn main() -> Result<(), HttpConnectionError> {
//! #    let client_id = env::var("CLIENT_ID").expect("Missing CLIENT_ID env var");
//!
//! let auth = Auth::client_id(client_id);
//! let conn = Connection::new(auth);
//! let client = conn.into_client();
//!
//! let res = client
//!     .query::<UserDetailsQuery>(
//!         user_details_query::Variables {
//!             username: "James".into(),
//!         }
//!     )
//!     .await?;
//!
//! let user = res.user;
//! println!("User details: {:#?}", user);
//! #    Ok(())
//! # }
//! ```
//!
//! ## License
//!
//! Licensed under either of
//!
//! -   Apache License, Version 2.0
//!     ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
//! -   MIT license
//!     ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
//!
//! at your option.
//!
//! ## Contribution
//!
//! Unless you explicitly state otherwise, any contribution intentionally submitted
//! for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
//! dual licensed as above, without any additional terms or conditions.

#![deny(missing_docs)]

#[macro_use]
extern crate async_trait;

mod auth;
mod client;
mod conn;
mod entities;
mod error;
mod subscription;

pub use auth::{AccessToken, Auth};
pub use client::Client;
pub use conn::*;
pub use error::*;
pub use subscription::*;
