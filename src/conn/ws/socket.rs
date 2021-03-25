use super::config::Config;
use crate::{
    entities::ws::{Empty, PhxReply, ReceivePhoenixMessage, SendPhoenixMessage},
    Auth, WebsocketConnectionError,
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

pub(super) struct Socket {
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
    pub fn new(auth: Auth, config: Config) -> Self {
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

    pub fn client(&self) -> SocketClient {
        SocketClient {
            join_ref: self.join_ref,
            outgoing_messages: self.outgoing_messages.0.clone(),
            incoming_messages: self.incoming_messages.0.clone(),
            request_timeout: self.config.request_timeout,
            cancellation_token: self.cancellation_token.clone(),
        }
    }

    pub async fn connect(&mut self) -> Result<(), WebsocketConnectionError> {
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

    pub fn stay_conected(mut self) {
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

    async fn wait(&mut self) -> Result<(), WebsocketConnectionError> {
        let handle = self
            .handle
            .take()
            .ok_or(WebsocketConnectionError::SocketClosed)?;
        self.outgoing_messages.1.replace(handle.await?);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub(super) struct SocketClient {
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

    pub fn close(self) {
        self.cancellation_token.cancel();
    }
}
