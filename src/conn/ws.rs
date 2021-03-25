//! Connection over WebSockets.
//!
//! You will usually pretty much immediately turn the connection into a Client.
//! E.g.
//! ```rust
//! use glimesh::{ws::Connection, Auth};
//! let client = Connection::connect(Auth::client_id("<GLIMESH_CLIENT_ID>")).await.into_client();
//! ```

mod config;
mod connection;
mod socket;

pub use connection::{Connection, ConnectionBuilder};
