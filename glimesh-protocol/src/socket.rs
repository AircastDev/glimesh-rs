use bimap::BiMap;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Value;
use serde_tuple::{Deserialize_tuple, Serialize_tuple};
use snafu::{ResultExt, Snafu};
use std::{collections::HashMap, hash::Hash};
use uuid::Uuid;

const TOPIC_PHOENIX: &str = "phoenix";
const TOPIC_ABSINTHE_CONTROL: &str = "__absinthe__:control";

#[derive(Debug, Clone, Serialize_tuple)]
struct SendPhoenixMessage<T: Serialize> {
    join_ref: Uuid,
    msg_ref: Uuid,
    topic: String,
    event: SendEvent,
    payload: T,
}

#[derive(Debug, Clone, Deserialize_tuple)]
struct ReceivePhoenixMessage<T: DeserializeOwned> {
    #[allow(unused)]
    join_ref: Option<Uuid>,
    msg_ref: Option<Uuid>,
    #[allow(unused)]
    topic: String,
    event: ReceiveEvent,
    payload: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Empty;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PhxReply<T> {
    response: T,
    status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DocumentSubscribeResponse {
    subscription_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SubscriptionEvent {
    result: serde_json::Value,
    subscription_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UnsubscribePayload {
    subscription_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SendEvent {
    PhxJoin,
    Heartbeat,
    Doc,
    Unsubscribe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ReceiveEvent {
    PhxReply,
    PhxError,

    #[serde(rename = "subscription:data")]
    SubscriptionData,
}

enum Request<T> {
    Join,
    Ping,
    Subscribe(T),
    Unsubscribe(T),
}

pub enum Event<T> {
    Joined,
    Pong,
    Document(T, Value),
}

pub enum SessionResult<T> {
    Event(Event<T>),
    Message(String),
}

impl<T> From<Event<T>> for SessionResult<T> {
    fn from(evt: Event<T>) -> Self {
        Self::Event(evt)
    }
}

struct PendingRequest<T> {
    msg_ref: Uuid,
    message: String,
    request: Request<T>,
}

pub struct SocketSession<T>
where
    T: Clone + Hash + Eq,
{
    join_ref: Uuid,
    pending_requests: Vec<PendingRequest<T>>,
    in_flight_requests: HashMap<Uuid, Request<T>>,
    subscriptions: BiMap<String, T>,
    joined: bool,
}

impl<T> SocketSession<T>
where
    T: Clone + Hash + Eq,
{
    pub fn new() -> Self {
        Self {
            join_ref: Uuid::new_v4(),
            pending_requests: Vec::new(),
            in_flight_requests: Default::default(),
            subscriptions: Default::default(),
            joined: false,
        }
    }

    fn send_request<V>(
        &mut self,
        request: Request<T>,
        message: SendPhoenixMessage<V>,
    ) -> Vec<SessionResult<T>>
    where
        V: Serialize,
    {
        let message_str = serde_json::to_string(&message).unwrap();

        if self.joined || matches!(request, Request::Join) {
            self.in_flight_requests.insert(message.msg_ref, request);
            vec![SessionResult::Message(message_str)]
        } else {
            self.pending_requests.push(PendingRequest {
                msg_ref: message.msg_ref,
                message: message_str,
                request,
            });
            vec![]
        }
    }

    pub fn join(&mut self) -> Vec<SessionResult<T>> {
        self.send_request(
            Request::Join,
            SendPhoenixMessage {
                join_ref: self.join_ref,
                msg_ref: Uuid::new_v4(),
                topic: TOPIC_ABSINTHE_CONTROL.into(),
                event: SendEvent::PhxJoin,
                payload: Empty,
            },
        )
    }

    pub fn ping(&mut self) -> Vec<SessionResult<T>> {
        self.send_request(
            Request::Ping,
            SendPhoenixMessage {
                join_ref: self.join_ref,
                msg_ref: Uuid::new_v4(),
                topic: TOPIC_PHOENIX.into(),
                event: SendEvent::Heartbeat,
                payload: Empty,
            },
        )
    }

    pub fn subscribe<B>(&mut self, reference: T, body: B) -> Vec<SessionResult<T>>
    where
        B: Serialize,
    {
        self.send_request(
            Request::Subscribe(reference),
            SendPhoenixMessage {
                join_ref: self.join_ref,
                msg_ref: Uuid::new_v4(),
                topic: TOPIC_ABSINTHE_CONTROL.into(),
                event: SendEvent::Doc,
                payload: body,
            },
        )
    }

    pub fn unsubscribe(&mut self, reference: T) -> Vec<SessionResult<T>> {
        match self.subscriptions.remove_by_right(&reference) {
            Some((subscription_id, _)) => self.send_request(
                Request::Unsubscribe(reference),
                SendPhoenixMessage {
                    join_ref: self.join_ref,
                    msg_ref: Uuid::new_v4(),
                    topic: TOPIC_ABSINTHE_CONTROL.into(),
                    event: SendEvent::Unsubscribe,
                    payload: UnsubscribePayload { subscription_id },
                },
            ),
            None => {
                vec![]
            }
        }
    }

    pub fn handle_message(
        &mut self,
        msg: &str,
    ) -> Result<Vec<SessionResult<T>>, HandleMessageError> {
        let message: ReceivePhoenixMessage<Value> =
            serde_json::from_str(msg).context(DeserializeSnafu {})?;

        let results = match message.event {
            ReceiveEvent::PhxReply => match message.msg_ref {
                Some(msg_ref) => match self.in_flight_requests.remove(&msg_ref) {
                    Some(Request::Join) => {
                        self.joined = true;
                        let mut results = Vec::with_capacity(self.pending_requests.len() + 1);
                        results.push(Event::Joined.into());
                        for pending in self.pending_requests.drain(..) {
                            self.in_flight_requests
                                .insert(pending.msg_ref, pending.request);
                            results.push(SessionResult::Message(pending.message));
                        }
                        results
                    }
                    Some(Request::Ping) => {
                        vec![Event::Pong.into()]
                    }
                    Some(Request::Subscribe(reference)) => {
                        let reply: PhxReply<DocumentSubscribeResponse> =
                            serde_json::from_value(message.payload).context(DeserializeSnafu {})?;
                        self.subscriptions
                            .insert(reply.response.subscription_id, reference);
                        vec![]
                    }
                    Some(Request::Unsubscribe(_)) => {
                        vec![]
                    }
                    None => {
                        tracing::warn!(?message, "received a reply to a request we didn't make");
                        vec![]
                    }
                },
                None => {
                    tracing::warn!(
                        ?message,
                        "received a reply that cannot be matched to a request"
                    );
                    vec![]
                }
            },
            ReceiveEvent::PhxError => {
                tracing::error!(?message, "got phx_error");
                // TODO
                vec![]
            }
            ReceiveEvent::SubscriptionData => {
                let data: SubscriptionEvent =
                    serde_json::from_value(message.payload).context(DeserializeSnafu {})?;
                match self.subscriptions.get_by_left(&data.subscription_id) {
                    Some(reference) => {
                        vec![Event::Document(reference.clone(), data.result).into()]
                    }
                    None => {
                        tracing::warn!(
                            ?data,
                            "received subscription data for a subscription we are not tracking"
                        );
                        vec![]
                    }
                }
            }
        };

        Ok(results)
    }
}

impl<T> Default for SocketSession<T>
where
    T: Clone + Hash + Eq,
{
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Snafu)]
pub enum HandleMessageError {
    DeserializeError { source: serde_json::Error },
}
