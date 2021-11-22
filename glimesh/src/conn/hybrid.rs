//! Hybrid connection that can use a different connection for subscriptions vs querues & mutations.
//!
//! You will usually pretty much immediately turn the connection into a Client.
//! E.g.
//! ```rust
//! use glimesh::{hybrid, ws, http, Auth};
//! let auth = Auth::client_credentials("<GLIMESH_CLIENT_ID>", "<GLIMESH_CLIENT_SECRET>");
//! let query = http::Connection::new(auth.clone());
//! let sub = ws::Connection::new(auth);
//! let client = hybrid::Connection::new(query, sub).into_client();
//! ```

use crate::{
    conn::{MutationConn, QueryConn, SubscriptionConn},
    subscription::Subscription,
    Client,
};
use graphql_client::GraphQLQuery;
use std::fmt::Debug;

/// A hybrid connection that allows combining two different connections into one effective
/// connection, for example for using an http client for querying & mutating, and a websocket client
/// for subscriptions.
pub struct Connection<C, S> {
    query_conn: C,
    subscription_conn: S,
}

impl<C, S> Connection<C, S> {
    /// Create a new hybrid connection. The first connection will be used for query and mutate
    /// operations, and the second only for subscriptions.
    pub fn new(query_conn: C, subscription_conn: S) -> Self {
        Self {
            query_conn,
            subscription_conn,
        }
    }

    /// Create a client with reference to this connection
    pub fn as_client(&self) -> Client<&Self> {
        Client::new(self)
    }

    /// Convert this connection into a client
    pub fn into_client(self) -> Client<Self> {
        Client::new(self)
    }
}

impl<C, S> Clone for Connection<C, S>
where
    C: Clone,
    S: Clone,
{
    fn clone(&self) -> Self {
        Self {
            query_conn: self.query_conn.clone(),
            subscription_conn: self.subscription_conn.clone(),
        }
    }
}

impl<C, S> Connection<C, S>
where
    C: Clone,
    S: Clone,
{
    /// Create a client with a clone of this connection
    pub fn to_client(&self) -> Client<Self> {
        Client::new(self.clone())
    }
}

impl<C, S> Debug for Connection<C, S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client").finish_non_exhaustive()
    }
}

#[async_trait]
impl<C, S> QueryConn for Connection<C, S>
where
    C: QueryConn + Send + Sync,
    S: Send + Sync,
{
    type Error = C::Error;

    async fn query<Q>(&self, variables: Q::Variables) -> Result<Q::ResponseData, Self::Error>
    where
        Q: GraphQLQuery,
        Q::Variables: Send + Sync,
    {
        self.query_conn.query::<Q>(variables).await
    }
}

#[async_trait]
impl<C, S> MutationConn for Connection<C, S>
where
    C: MutationConn + Send + Sync,
    S: Send + Sync,
{
    type Error = C::Error;

    async fn mutate<Q>(&self, variables: Q::Variables) -> Result<Q::ResponseData, Self::Error>
    where
        Q: GraphQLQuery,
        Q::Variables: Send + Sync,
    {
        self.query_conn.mutate::<Q>(variables).await
    }
}

#[async_trait]
impl<C, S> SubscriptionConn for Connection<C, S>
where
    S: SubscriptionConn + Send + Sync,
    C: Send + Sync,
{
    type Error = S::Error;

    async fn subscribe<Q>(
        &self,
        variables: Q::Variables,
    ) -> Result<Subscription<Q::ResponseData>, Self::Error>
    where
        Q: GraphQLQuery,
        Q::Variables: Send + Sync,
    {
        self.subscription_conn.subscribe::<Q>(variables).await
    }
}

/// Type alias for a hybrid backed client
pub type HybridClient<C, S> = Client<Connection<C, S>>;
