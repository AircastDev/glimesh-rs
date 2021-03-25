use super::{
    config::Config,
    socket::{Socket, SocketClient},
};
use crate::{Auth, Client, GlimeshError, MutationConn, QueryConn, WebsocketConnectionError};
use std::{fmt::Debug, time::Duration};

/// Configure and build a websocket [`Connection`].
///
/// ## Usage:
/// ```rust
/// let connection = Connection::builder()
///     .api_url("ws://localhost:8080/api/socket/websocket")
///     .build();
/// ```
#[derive(Debug, Default)]
pub struct ConnectionBuilder {
    config: Config,
}

impl ConnectionBuilder {
    /// Build the websocket connection from the set options and connect to the socket.
    pub async fn connect(self, auth: Auth) -> Result<Connection, WebsocketConnectionError> {
        let mut socket = Socket::new(auth, self.config);
        socket.connect().await?;
        let socket_client = socket.client();
        socket.stay_conected();

        Ok(Connection {
            socket: socket_client,
        })
    }

    /// Set the base api url used for request.
    /// Useful if running Glimesh locally for example.
    ///
    /// Defaults to `wss://glimesh.tv/api/socket/websocket`
    pub fn api_url(mut self, value: impl Into<String>) -> Self {
        self.config.api_url = value.into();
        self
    }

    /// Set the version passed as query param `vsn`.
    ///
    /// This defaults to `2.0.0` and is all glimesh supports at the time of writing.
    pub fn version(mut self, value: impl Into<String>) -> Self {
        self.config.version = value.into();
        self
    }

    /// Number of messages to buffer before sending messages blocks the sender.
    ///
    /// This defaults to 100
    pub fn outgoing_capacity(mut self, value: usize) -> Self {
        self.config.outgoing_capacity = value;
        self
    }

    /// Number of messages buffered before older messages a dropped if they aren't being handled in time.
    ///
    /// This defaults to 10_000 to allow for bursts of messages.
    pub fn incoming_capacity(mut self, value: usize) -> Self {
        self.config.incoming_capacity = value;
        self
    }

    /// Number of seconds between each socket ping
    ///
    /// This defaults to 30 seconds.
    pub fn ping_interval(mut self, value: Duration) -> Self {
        self.config.ping_interval = value;
        self
    }

    /// How long to wait for a response to a request over the socket before timing out.
    ///
    /// This defaults to 30 seconds.
    pub fn request_timeout(mut self, value: Duration) -> Self {
        self.config.request_timeout = value;
        self
    }
}

/// Connect to glimesh over websockets
#[derive(Debug, Clone)]
pub struct Connection {
    socket: SocketClient,
}

impl Connection {
    /// Create a [`ConnectionBuilder`] to configure various options.
    pub fn builder() -> ConnectionBuilder {
        ConnectionBuilder::default()
    }

    /// Create a connection with the default options.
    pub async fn connect(auth: Auth) -> Result<Self, WebsocketConnectionError> {
        ConnectionBuilder::default().connect(auth).await
    }

    /// Create a client with reference to this connection
    pub fn as_client(&self) -> Client<&Self, WebsocketConnectionError> {
        Client::new(self)
    }

    /// Create a client with a clone of this connection
    pub fn to_client(&self) -> Client<Self, WebsocketConnectionError> {
        Client::new(self.clone())
    }

    /// Convert this connection into a client
    pub fn into_client(self) -> Client<Self, WebsocketConnectionError> {
        Client::new(self)
    }

    /// Terminate the socket connection
    pub fn close(self) {
        self.socket.close();
    }

    async fn request<Q>(
        &self,
        variables: Q::Variables,
    ) -> Result<Q::ResponseData, WebsocketConnectionError>
    where
        Q: graphql_client::GraphQLQuery,
    {
        let reply = self
            .socket
            .request(
                "__absinthe__:control".into(),
                "doc".into(),
                Q::build_query(variables),
            )
            .await?;

        let res: graphql_client::Response<Q::ResponseData> = reply.response;

        if let Some(errs) = res.errors {
            if errs.len() > 0 {
                return Err(GlimeshError::GraphqlErrors(errs).into());
            }
        }

        let data = res.data.ok_or_else(|| GlimeshError::NoData)?;
        Ok(data)
    }
}

#[async_trait]
impl QueryConn for Connection {
    type Error = WebsocketConnectionError;

    async fn query<Q>(&self, variables: Q::Variables) -> Result<Q::ResponseData, Self::Error>
    where
        Q: graphql_client::GraphQLQuery,
        Q::Variables: Send + Sync,
    {
        self.request::<Q>(variables).await
    }
}

#[async_trait]
impl MutationConn for Connection {
    type Error = WebsocketConnectionError;

    async fn mutate<Q>(&self, variables: Q::Variables) -> Result<Q::ResponseData, Self::Error>
    where
        Q: graphql_client::GraphQLQuery,
        Q::Variables: Send + Sync,
    {
        self.request::<Q>(variables).await
    }
}
