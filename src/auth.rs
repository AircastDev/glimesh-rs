use std::time::SystemTime;

/// Authentication method.
/// The Glimesh API requires an authentication method to be used.
/// The most basic is the ClientId method, which gives you read only access to the api.
#[derive(Debug)]
pub enum Auth {
    /// Use Client-ID authentication.
    /// When using this method, you can only 'read' from the API.
    ClientId(String),

    /// Use Bearer authentication.
    /// The supplied access token is assumed to be valid.
    /// If you would like the client to handle token refreshing, use AccessTokenWithRefresh instead.
    AccessToken(String),

    /// Use Bearer authentication.
    /// This will use the provided refresh token to refresh the access token when/if it expires.
    ///
    /// If `expires_at` is supplied, the client will preemptively refresh the token when nearing expiry.
    /// If it is None, then the client will only refresh when the API returns an authentication error.
    AccessTokenWithRefresh {
        /// Glimesh access token
        access_token: String,

        /// Glimesh refresh token
        refresh_token: String,

        /// Time the glimesh access token expires. Optional.
        expires_at: Option<SystemTime>,
    },
}

impl Auth {
    /// Use Client-ID authentication.
    /// When using this method, you can only 'read' from the API.
    pub fn client_id(client_id: impl Into<String>) -> Self {
        Self::ClientId(client_id.into())
    }

    /// Use Bearer authentication.
    /// The supplied access token is assumed to be valid.
    /// If you would like the client to handle token refreshing, use AccessTokenWithRefresh instead.
    pub fn access_token(access_token: impl Into<String>) -> Self {
        Self::AccessToken(access_token.into())
    }
}
