use crate::conn::{MutationConn, QueryConn, SubscriptionConn};
use futures::stream::BoxStream;
use graphql_client::GraphQLQuery;
use std::{fmt::Debug, marker::PhantomData};

/// Glimesh client.
/// The client is generic over its connection/transport, meaning it can be used with http
/// or websockets (or any future transport Glimesh might support).
///
/// To create a client you first create the underlying connection/transport,
/// such as [`crate::http::Connection`].
pub struct Client<T, E> {
    conn: T,
    _err_type: PhantomData<E>,
}

impl<T, E> Client<T, E> {
    /// Create a new client from a connection. Prefer calling `into_client` on the connection itself.
    pub fn new(conn: T) -> Self {
        Self {
            conn,
            _err_type: PhantomData,
        }
    }
}

impl<T: Clone, E> Clone for Client<T, E> {
    fn clone(&self) -> Self {
        Self {
            conn: self.conn.clone(),
            _err_type: PhantomData,
        }
    }
}

impl<T: Debug, E> Debug for Client<T, E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Client").field("conn", &self.conn).finish()
    }
}

impl<T, E> Client<T, E>
where
    T: QueryConn<Error = E>,
{
    /// Perform a graphql query using the underlying connection.
    ///
    /// # Errors
    /// See [`QueryConn::query`] for error information
    pub async fn query<Q>(&self, variables: Q::Variables) -> Result<Q::ResponseData, E>
    where
        Q: GraphQLQuery,
        Q::Variables: Send + Sync,
    {
        self.conn.query::<Q>(variables).await
    }
}

impl<T, E> Client<T, E>
where
    T: MutationConn<Error = E>,
{
    /// Perform a graphql mutation using the underlying connection
    ///
    /// # Errors
    /// See [`MutationConn::mutate`] for error information
    pub async fn mutate<Q>(&self, variables: Q::Variables) -> Result<Q::ResponseData, E>
    where
        Q: GraphQLQuery,
        Q::Variables: Send + Sync,
    {
        self.conn.mutate::<Q>(variables).await
    }
}

impl<T, E> Client<T, E>
where
    T: SubscriptionConn<Error = E>,
{
    /// Subscribe to a graphql subcription using the underlying connection.
    ///
    /// # Errors
    /// See [`SubscriptionConn::subscribe`] for error information
    pub async fn subscribe<Q>(
        &self,
        variables: Q::Variables,
    ) -> Result<BoxStream<'_, Q::ResponseData>, E>
    where
        Q: GraphQLQuery,
        Q::Variables: Send + Sync,
    {
        self.conn.subscribe::<Q>(variables).await
    }
}
