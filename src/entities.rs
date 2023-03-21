#[cfg(feature = "websocket")]
pub(crate) mod ws {
    use serde::{de::DeserializeOwned, Deserialize, Serialize};
    use serde_tuple::{Deserialize_tuple, Serialize_tuple};
    use uuid::Uuid;

    #[derive(Debug, Clone, Serialize_tuple)]
    pub(crate) struct SendPhoenixMessage<T: Serialize> {
        pub(crate) join_ref: Uuid,
        pub(crate) msg_ref: Uuid,
        pub(crate) topic: String,
        pub(crate) event: String,
        pub(crate) payload: T,
    }

    #[derive(Debug, Clone, Deserialize_tuple)]
    pub(crate) struct ReceivePhoenixMessage<T: DeserializeOwned> {
        #[allow(unused)]
        pub(crate) join_ref: Option<Uuid>,
        pub(crate) msg_ref: Option<Uuid>,
        pub(crate) topic: String,
        pub(crate) event: String,
        pub(crate) payload: T,
    }
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub(crate) struct PhxReply<T> {
        pub(crate) response: T,
        pub(crate) status: String,
    }
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub(crate) struct Empty {}

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub(crate) struct EventSubscription {
        pub result: serde_json::Value,
        pub subscription_id: String,
    }

    #[derive(Debug, Clone, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub(crate) struct DocumentSubscribeResponse {
        pub subscription_id: String,
    }
}
