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

use crate::Client;
pub use connection::{Connection, ConnectionBuilder};

/// Type alias for a websocket backed client
pub type WebsocketClient = Client<Connection>;
