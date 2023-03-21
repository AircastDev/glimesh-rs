use crate::{
    conn::{MutationConn, QueryConn, SubscriptionConn},
    subscription::Subscription,
};
use graphql_client::GraphQLQuery;
use std::fmt::Debug;

/// Glimesh client.
/// The client is generic over its connection/transport, meaning it can be used with http
/// or websockets (or any future transport Glimesh might support).
///
/// To create a client you first create the underlying connection/transport,
/// such as [`crate::http::Connection`].
pub struct Client<T> {
    conn: T,
}

impl<T> Client<T> {
    /// Create a new client from a connection. Prefer calling `into_client` on the connection itself.
    pub fn new(conn: T) -> Self {
        Self { conn }
    }

    /// Turn this client back into its underlying connection
    pub fn into_connection(self) -> T {
        self.conn
    }
}

impl<T: Clone> Clone for Client<T> {
    fn clone(&self) -> Self {
        Self {
            conn: self.conn.clone(),
        }
    }
}

impl<T: Debug> Debug for Client<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client").field("conn", &self.conn).finish()
    }
}

impl<T> Client<T>
where
    T: QueryConn,
{
    /// Perform a graphql query using the underlying connection.
    ///
    /// # Errors
    /// See [`QueryConn::query`] for error information
    pub async fn query<Q>(&self, variables: Q::Variables) -> Result<Q::ResponseData, T::Error>
    where
        Q: GraphQLQuery,
        Q::Variables: Send + Sync,
    {
        self.conn.query::<Q>(variables).await
    }
}

impl<T> Client<T>
where
    T: MutationConn,
{
    /// Perform a graphql mutation using the underlying connection
    ///
    /// # Errors
    /// See [`MutationConn::mutate`] for error information
    pub async fn mutate<Q>(&self, variables: Q::Variables) -> Result<Q::ResponseData, T::Error>
    where
        Q: GraphQLQuery,
        Q::Variables: Send + Sync,
    {
        self.conn.mutate::<Q>(variables).await
    }
}

impl<T> Client<T>
where
    T: SubscriptionConn,
{
    /// Subscribe to a graphql subcription using the underlying connection.
    ///
    /// # Errors
    /// See [`SubscriptionConn::subscribe`] for error information
    pub async fn subscribe<'a, Q>(
        &self,
        variables: Q::Variables,
    ) -> Result<Subscription<Q::ResponseData>, T::Error>
    where
        Q: GraphQLQuery,
        Q::Variables: Send + Sync,
    {
        self.conn.subscribe::<Q>(variables).await
    }
}
