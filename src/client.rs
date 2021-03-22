use crate::{
    conn::{MutationConn, QueryConn, SubscriptionConn},
    error::Error,
};
use futures::{channel::mpsc, Stream};
use graphql_client::GraphQLQuery;

/// Glimesh client.
/// The client is generic over its connection/transport, meaning it can be used with http
/// or websockets (or any future transport Glimesh might support).
///
/// To create a client you first create the underlying conneciton/transport,
/// such as [`crate::http::Connection`].
pub struct Client<T> {
    conn: T,
}

impl<T> Client<T> {
    /// Create a new client from a connection. Prefer calling `into_client` on the connection itself.
    pub fn new(conn: T) -> Self {
        Self { conn }
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
    pub async fn query<Q>(&self, variables: Q::Variables) -> Result<Q::ResponseData, Error>
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
    pub async fn mutate<Q>(&self, variables: Q::Variables) -> Result<Q::ResponseData, Error>
    where
        Q: GraphQLQuery,
        Q::Variables: Send + Sync,
    {
        self.conn.mutate::<Q>(variables).await
    }
}

pub struct SubscribeOpts {
    /// How manay messages to buffer on the subscroption before potentially 'blocking' other subscriptions.
    pub buffer_size: usize,
}

impl Default for SubscribeOpts {
    fn default() -> Self {
        Self { buffer_size: 10 }
    }
}

impl SubscribeOpts {
    pub fn with_buffer_size(mut self, buffer_size: usize) -> Self {
        self.buffer_size = buffer_size;
        self
    }
}

impl<T> Client<T>
where
    T: SubscriptionConn,
{
    /// Subscribe to a graphql subcription using the underlying connection.
    /// This method is a shortcut for calling [`Self::subscribe_with_opts`] with default options.
    ///
    /// # Errors
    /// See [`SubscriptionConn::subscribe`] for error information
    pub async fn subscribe<Q>(
        &self,
        variables: Q::Variables,
    ) -> Result<impl Stream<Item = Q::ResponseData>, Error>
    where
        Q: GraphQLQuery,
        Q::Variables: Send + Sync,
    {
        self.subscribe_with_opts::<Q>(variables, Default::default())
            .await
    }

    /// Subscribe to a graphql subcription using the underlying connection.
    ///
    /// # Errors
    /// See [`SubscriptionConn::subscribe`] for error information
    pub async fn subscribe_with_opts<Q>(
        &self,
        variables: Q::Variables,
        opts: SubscribeOpts,
    ) -> Result<impl Stream<Item = Q::ResponseData>, Error>
    where
        Q: GraphQLQuery,
        Q::Variables: Send + Sync,
    {
        let (sender, receiver) = mpsc::channel(opts.buffer_size);
        self.conn.subscribe::<Q>(variables, sender).await?;
        Ok(receiver)
    }
}
