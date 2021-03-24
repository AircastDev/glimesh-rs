//! Connection over WebSockets.
//!
//! You will usually pretty much immediately turn the connection into a Client.
//! E.g.
//! ```rust
//! use glimesh::{ws::Connection, Auth};
//! let client = Connection::connect(Auth::client_id("<GLIMESH_CLIENT_ID>")).await.into_client();
//! ```

use crate::{
    entities::ws::{Empty, PhxReply, ReceivePhoenixMessage, SendPhoenixMessage},
    Auth, Client, GlimeshError, MutationConn, QueryConn, WebsocketConnectionError,
};
use async_tungstenite::{tokio::connect_async, tungstenite::Message};
use backoff::{backoff::Backoff, ExponentialBackoff};
use futures::{future::BoxFuture, SinkExt};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use std::{fmt::Debug, time::Duration};
use tokio::{
    select,
    sync::{broadcast, mpsc},
    task,
    time::{sleep, timeout},
};
use tokio_stream::{wrappers::BroadcastStream, StreamExt};
use tokio_util::sync::CancellationToken;
use tracing::Instrument;
use uuid::Uuid;

#[derive(Debug)]
struct Config {
    api_url: String,
    version: String,
    outgoing_capacity: usize,
    incoming_capacity: usize,
    ping_interval: Duration,
    request_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            api_url: String::from("wss://glimesh.tv/api/socket/websocket"),
            version: String::from("2.0.0"),
            outgoing_capacity: 100,
            incoming_capacity: 10_000,
            ping_interval: Duration::from_secs(30),
            request_timeout: Duration::from_secs(30),
        }
    }
}

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

struct Socket {
    auth: Auth,
    config: Config,
    join_ref: Uuid,
    outgoing_messages: (mpsc::Sender<Message>, Option<mpsc::Receiver<Message>>),
    incoming_messages: (
        broadcast::Sender<ReceivePhoenixMessage<Value>>,
        broadcast::Receiver<ReceivePhoenixMessage<Value>>,
    ),
    cancellation_token: CancellationToken,
    handle: Option<BoxFuture<'static, Result<mpsc::Receiver<Message>, WebsocketConnectionError>>>,
}

impl Debug for Socket {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Socket")
            .field("auth", &self.auth)
            .field("config", &self.config)
            .field("join_ref", &self.join_ref)
            .finish()
    }
}

impl Socket {
    fn new(auth: Auth, config: Config) -> Self {
        let (outgoing_messages_sender, outgoing_messages_receiver) =
            mpsc::channel(config.outgoing_capacity);
        let incoming_messages = broadcast::channel(config.incoming_capacity);

        Self {
            auth,
            config,
            join_ref: Uuid::new_v4(),
            outgoing_messages: (outgoing_messages_sender, Some(outgoing_messages_receiver)),
            incoming_messages,
            cancellation_token: CancellationToken::new(),
            handle: None,
        }
    }

    fn client(&self) -> SocketClient {
        SocketClient {
            join_ref: self.join_ref,
            outgoing_messages: self.outgoing_messages.0.clone(),
            incoming_messages: self.incoming_messages.0.clone(),
            request_timeout: self.config.request_timeout,
            cancellation_token: self.cancellation_token.clone(),
        }
    }

    async fn connect(&mut self) -> Result<(), WebsocketConnectionError> {
        let mut query = vec![("vsn", self.config.version.clone())];
        match &self.auth {
            Auth::ClientId(client_id) => query.push(("client_id", client_id.clone())),
            Auth::AccessToken(token) => query.push(("token", token.clone())),
            Auth::RefreshableAccessToken(token) => {
                let access_token = token.access_token().await?;
                query.push(("token", access_token.access_token));
            }
            Auth::ClientCredentials(client_credentials) => {
                let access_token = client_credentials.access_token().await?;
                query.push(("token", access_token.access_token));
            }
        }

        let query_str = serde_urlencoded::to_string(query.as_slice())?;
        let connection_url = format!("{}?{}", self.config.api_url, query_str);

        let (ws_stream, _) = connect_async(&connection_url).await?;
        let (mut ws_tx, mut ws_rx) = futures::StreamExt::split(ws_stream);

        let cancellation_token = self.cancellation_token.child_token();

        let outgoing_messages_handle = {
            let mut outgoing_messages_receiver = self
                .outgoing_messages
                .1
                .take()
                .ok_or(WebsocketConnectionError::AlreadyConnected)?;
            let cancellation_token = cancellation_token.clone();
            task::spawn(async move {
                loop {
                    select! {
                        _ = cancellation_token.cancelled() => {
                            tracing::trace!("received cancellation signal");
                            break;
                        }
                        msg = outgoing_messages_receiver.recv() => {
                            match msg {
                                Some(msg) => {
                                    tracing::trace!(?msg, "sending message");
                                    if let Err(err) = ws_tx.send(msg).await {
                                        tracing::error!(?err, "failed to send message on the socket");
                                        cancellation_token.cancel();
                                        break;
                                    }
                                }
                                None => {
                                    tracing::trace!("all senders were dropped");
                                    cancellation_token.cancel();
                                    break;
                                }
                            }
                        }
                    }
                }

                outgoing_messages_receiver
            })
            .instrument(tracing::trace_span!("outgoing_messages"))
        };

        let incoming_messages_handle = {
            let cancellation_token = cancellation_token.clone();
            let incoming_messages_sender = self.incoming_messages.0.clone();
            task::spawn(async move {
                loop {
                    select! {
                        _ = cancellation_token.cancelled() => {
                            tracing::trace!("received cancellation signal");
                            break;
                        }
                        msg = ws_rx.next() => {
                            match msg {
                                Some(Ok(Message::Text(text))) => {
                                    match serde_json::from_str::<ReceivePhoenixMessage<Value>>(&text) {
                                        Ok(msg) => {
                                            if msg.event == "phx_error" {
                                                tracing::error!(?msg.payload, "error on socket");
                                                cancellation_token.cancel();
                                                break;
                                            }

                                            tracing::trace!(?msg, "incoming message");
                                            if let Err(err) = incoming_messages_sender.send(msg) {
                                                tracing::error!(?text, ?err, "failed to broadcast incoming message");
                                            }
                                        }
                                        Err(err) => {
                                            tracing::error!(?text, ?err, "failed to deserialize glimesh message");
                                        }
                                    }
                                }
                                Some(Ok(Message::Close(reason))) => {
                                    tracing::error!(?reason, "socket closed");
                                    cancellation_token.cancel();
                                    break;
                                }
                                Some(Ok(frame)) => {
                                    tracing::error!(?frame, "unexpected frame type");
                                    cancellation_token.cancel();
                                    break;
                                }
                                Some(Err(err)) => {
                                    tracing::error!(?err, "socket error");
                                    cancellation_token.cancel();
                                    break;
                                }
                                None => {
                                    // The socket must have errored in the previous
                                    // iteration so we should never really get here
                                    tracing::error!("no more socket messages");
                                    cancellation_token.cancel();
                                    break;
                                }
                            }
                        }
                    }
                }
            })
            .instrument(tracing::trace_span!("incoming_messages"))
        };

        let socket_client = self.client();
        if let Err(err) = socket_client
            .request::<_, Empty>("__absinthe__:control".into(), "phx_join".into(), Empty {})
            .await
        {
            tracing::error!(?err, "join request failed");
            cancellation_token.cancel();
            return Err(err);
        }

        let pinger_handle = {
            let ping_interval = Duration::from_secs(30);
            let cancellation_token = cancellation_token.clone();
            task::spawn(async move {
                loop {
                    select! {
                        _ = cancellation_token.cancelled() => {
                            tracing::trace!("received cancellation signal");
                            break;
                        }
                        _ = sleep(ping_interval) => {
                            if let Err(err) = socket_client.request::<_, Empty>(
                                "phoenix".into(),
                                "heartbeat".into(),
                                Empty {},
                            )
                            .await {
                                tracing::error!(?err, "failed to send ping");
                                cancellation_token.cancel();
                                break;
                            }
                        }
                    };
                }
            })
            .instrument(tracing::trace_span!("pinger"))
        };

        self.handle.replace(Box::pin(async move {
            incoming_messages_handle
                .await
                .map_err(anyhow::Error::from)?;
            pinger_handle.await.map_err(anyhow::Error::from)?;
            let outgoing_messages_receiver = outgoing_messages_handle
                .await
                .map_err(anyhow::Error::from)?;
            Ok::<_, WebsocketConnectionError>(outgoing_messages_receiver)
        }));

        Ok(())
    }

    async fn wait(&mut self) -> Result<(), WebsocketConnectionError> {
        let handle = self
            .handle
            .take()
            .ok_or(WebsocketConnectionError::SocketClosed)?;
        self.outgoing_messages.1.replace(handle.await?);
        Ok(())
    }

    fn stay_conected(mut self) {
        task::spawn(async move {
            loop {
                if let Err(err) = self.wait().await {
                    tracing::error!(?err, "irrecoverable connecton error");
                    // TODO: some way of bubbling this up to the consumer
                    break;
                }

                if self.cancellation_token.is_cancelled() {
                    break;
                }

                let mut backoff = ExponentialBackoff::default();
                while let Err(err) = self.connect().await {
                    match backoff.next_backoff() {
                        Some(backoff_time) => {
                            tracing::error!(
                                ?err,
                                "failed to reconnect, retrying in {:?}",
                                backoff_time
                            );
                            sleep(backoff_time).await;
                        }
                        None => {
                            tracing::error!(?err, "failed to reconnect, after many attempts");
                            // TODO: some way of bubbling this up to the consumer
                            break;
                        }
                    }
                }
            }
        });
    }
}

#[derive(Debug, Clone)]
struct SocketClient {
    join_ref: Uuid,
    outgoing_messages: mpsc::Sender<Message>,
    incoming_messages: broadcast::Sender<ReceivePhoenixMessage<Value>>,
    request_timeout: Duration,
    cancellation_token: CancellationToken,
}

impl SocketClient {
    pub async fn request<T, U>(
        &self,
        topic: String,
        event: String,
        payload: T,
    ) -> Result<PhxReply<U>, WebsocketConnectionError>
    where
        T: Serialize,
        U: DeserializeOwned,
    {
        let msg_ref = Uuid::new_v4();
        let msg = serde_json::to_string(&SendPhoenixMessage {
            join_ref: self.join_ref,
            msg_ref,
            topic: topic.into(),
            event: event.into(),
            payload,
        })?;
        self.outgoing_messages.send(msg.into()).await?;

        timeout(
            self.request_timeout,
            BroadcastStream::new(self.incoming_messages.subscribe())
                .filter_map(|msg| match msg {
                    Ok(msg) => {
                        if msg.msg_ref == Some(msg_ref) {
                            Some(
                                serde_json::from_value::<PhxReply<U>>(msg.payload)
                                    .map_err(WebsocketConnectionError::from),
                            )
                        } else {
                            None
                        }
                    }
                    Err(_) => None,
                })
                .take(1)
                .next(),
        )
        .await?
        .ok_or(WebsocketConnectionError::SocketClosed)
        .and_then(|r| r) // .flatten() is unstable
    }

    fn close(self) {
        self.cancellation_token.cancel();
    }
}
