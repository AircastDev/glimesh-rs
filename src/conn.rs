use crate::subscription::Subscription;
use graphql_client::GraphQLQuery;

#[cfg(feature = "http")]
pub mod http;

#[cfg(feature = "websocket")]
pub mod ws;

pub mod hybrid;

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
    /// and then any messages will be sent on the returned stream
    ///
    /// This subscription is resiliant against errors on the underlying connection, such that
    /// for example, if the websocket connection fails, when/if it succesfully reconnects, the
    /// subscriptions will be replayed on the new connection. This is currently unobservable,
    /// please open an issue if you need to be able to detect when this happens.
    ///
    /// # Errors
    /// This function may error if there was a problem with the underlying connection such as
    /// a dns resolution error, or the websocket is disconnected, or if the api returned an error
    /// or the api response failed to decode.
    async fn subscribe<Q>(
        &self,
        variables: Q::Variables,
    ) -> Result<Subscription<Q::ResponseData>, Self::Error>
    where
        Q: GraphQLQuery,
        Q::Variables: Send + Sync;
}
