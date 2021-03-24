use futures::channel::mpsc;
use graphql_client::GraphQLQuery;

#[cfg(feature = "http")]
pub mod http;

#[cfg(feature = "websocket")]
pub mod ws;

/// Connections that implement this support graphql queries.
#[async_trait]
pub trait QueryConn {
    /// Error type representing any errors that can occurr when querying
    type Error;

    /// Send a graphql query over this connection.
    ///
    /// # Errors
    /// This function may error if there was a problem with the underlying connection such as
    /// a dns resolution error, or the websocket is disconnected, or if the api returned an error
    /// or the api response failed to decode.
    async fn query<Q>(&self, variables: Q::Variables) -> Result<Q::ResponseData, Self::Error>
    where
        Q: GraphQLQuery,
        Q::Variables: Send + Sync;
}

/// Connections that implement this support graphql mutations.
#[async_trait]
pub trait MutationConn {
    /// Error type representing any errors that can occurr when mutating
    type Error;

    /// Send a graphql mutation over this connection.
    ///
    /// # Errors
    /// This function may error if there was a problem with the underlying connection such as
    /// a dns resolution error, or the websocket is disconnected, or if the api returned an error
    /// or the api response failed to decode.
    async fn mutate<Q>(&self, variables: Q::Variables) -> Result<Q::ResponseData, Self::Error>
    where
        Q: GraphQLQuery,
        Q::Variables: Send + Sync;
}

/// Connections that implement this support graphql subscriptions.
#[async_trait]
pub trait SubscriptionConn {
    /// Error type representing any errors that can occurr when subscribing
    type Error;

    /// Send a graphql subscription over this connection.
    /// The future will resolve when the subscription has been established,
    /// and then any messages will be sent to the passed in mpsc Sender.
    ///
    /// # Errors
    /// This function may error if there was a problem with the underlying connection such as
    /// a dns resolution error, or the websocket is disconnected, or if the api returned an error
    /// or the api response failed to decode.
    async fn subscribe<Q>(
        &self,
        variables: Q::Variables,
        sender: mpsc::Sender<Q::ResponseData>,
    ) -> Result<(), Self::Error>
    where
        Q: GraphQLQuery,
        Q::Variables: Send + Sync;
}
