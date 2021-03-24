/// Errors that can occur when interacting with the Glimesh API.
#[derive(Debug, thiserror::Error)]
pub enum GlimeshError {
    /// A list of errors returned by graphql.
    /// This is guaranteed to always have at least one error.
    #[error("Graphql error(s): {0:?}")]
    GraphqlErrors(Vec<graphql_client::Error>),

    /// The graphql response contained no errors, but null data.
    /// Its likely this will never really happen if the API is behaving.
    #[error("Missing response data")]
    NoData,
}

/// Errors that can occur obtaining and refreshing access tokens
#[cfg(feature = "http")]
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// A request returned a bad status code. This may happen if the auth credentials
    /// were wrong for example.
    #[error("{0} response from api")]
    BadStatus(u16),

    /// Attempted to refresh a token without having a redirect uri specified on the client.
    #[error("Missing redirect uri in client config")]
    MissingRedirect,

    /// Attempted to refresh a token but the refresh_token was None
    #[error("Missing refresh token")]
    MissingRefreshToken,

    /// Some other error occurred.
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

/// Errors that can occur using the http based connection
#[cfg(feature = "http")]
#[derive(Debug, thiserror::Error)]
pub enum HttpConnectionError {
    /// A glimesh error ocurred
    #[error(transparent)]
    Glimesh(#[from] GlimeshError),

    /// An auth error ocurred
    #[error(transparent)]
    Auth(#[from] AuthError),

    /// A request returned a bad status code.
    #[error("{0} response from api")]
    BadStatus(u16),

    /// Some other error occurred.
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}

/// Errors that can occur using the websocket based connection
#[cfg(feature = "websocket")]
#[derive(Debug, thiserror::Error)]
pub enum WebsocketConnectionError {
    /// A glimesh error ocurred
    #[error(transparent)]
    Glimesh(#[from] GlimeshError),

    /// An auth error ocurred
    #[error(transparent)]
    Auth(#[from] AuthError),

    /// Failed to serialize query param
    #[error(transparent)]
    QueryParamSerialization(#[from] serde_urlencoded::ser::Error),

    /// Failed to (de)serialize a json payload
    #[error(transparent)]
    SerdeJson(#[from] serde_json::Error),

    /// Websocket negotiating failure
    #[error(transparent)]
    Tungstenite(#[from] async_tungstenite::tungstenite::Error),

    /// Socket message sender channel likely closed.
    #[error("failed to send socket message over channel, {0}")]
    SendError(#[from] tokio::sync::mpsc::error::SendError<async_tungstenite::tungstenite::Message>),

    /// Request sent over the socket reached its timeout before hearing a response.
    #[error("request timed out")]
    Timeout(#[from] tokio::time::error::Elapsed),

    /// connect() was called on a socket before it released the resources used in the previous connection attempt.
    #[error("attempt to connect a socket that is already holding on to a connection")]
    AlreadyConnected,

    /// Attempted to send or receive on a closed connection
    #[error("attmpted to send or receive on a closed socket")]
    SocketClosed,

    /// Some other error occurred.
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}
