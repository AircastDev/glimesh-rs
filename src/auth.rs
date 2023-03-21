use crate::AuthError;
use chrono::{DateTime, Duration, Utc};
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_stream::wrappers::ReceiverStream;

const EXPIRY_BUFFER: i64 = 5 * 60;

/// Authentication method.
/// The Glimesh API requires an authentication method to be used.
/// The most basic is the ClientId method, which gives you read only access to the api.
#[derive(Debug, Clone)]
pub enum Auth {
    /// Use Client-ID authentication.
    /// When using this method, you can only 'read' from the API.
    ClientId(String),

    /// Use Bearer authentication.
    /// The supplied access token is assumed to be valid.
    /// If you would like the client to handle token refreshing, use [`Auth::RefreshableAccessToken`] instead.
    AccessToken(String),

    /// Use Bearer authentication.
    /// This will use the provided refresh token to refresh the access token when/if it expires.
    RefreshableAccessToken(RefreshableAccessToken),

    /// Use Bearer authentication via the client_credentials flow.
    ///
    /// This allows you to log in to the api as the account that created the developer application.
    ClientCredentials(ClientCredentials),
}

impl Auth {
    /// Use Client-ID authentication.
    /// When using this method, you can only 'read' from the API.
    pub fn client_id(client_id: impl Into<String>) -> Self {
        Self::ClientId(client_id.into())
    }

    /// Use Bearer authentication.
    /// The supplied access token is assumed to be valid.
    /// If you would like the client to handle token refreshing, use [`Auth::RefreshableAccessToken`] instead.
    pub fn access_token(access_token: impl Into<String>) -> Self {
        Self::AccessToken(access_token.into())
    }

    /// Use Bearer authentication. This will use the provided refresh token to refresh the access
    /// token when/if it expires.
    ///
    /// You can listen for updates to the token using the Stream returned as the second part of the
    /// tuple.
    ///
    /// ## Example
    ///
    /// ```rust
    /// let (auth, token_receiver) = Auth::refreshable_access_token(
    ///     "<client_id>",
    ///     "<client_secret>",
    ///     "<redirect_uri>",
    ///     AccessToken { .. },
    /// );
    ///
    /// tokio::spawn(async move {
    ///     while let Some(token) = token_receiver.next().await {
    ///         println!("new token = {:#?}", token);
    ///     }
    /// });
    /// ```
    ///
    /// # Panics
    /// This function panics if the TLS backend cannot be initialized, or the resolver cannot load
    /// the system configuration.
    pub fn refreshable_access_token(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        redirect_uri: impl Into<String>,
        access_token: AccessToken,
    ) -> (Self, impl Stream<Item = AccessToken>) {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to create http client");

        let (token_sender, token_receiver) = mpsc::channel(1);

        (
            Self::RefreshableAccessToken(RefreshableAccessToken {
                client: ClientConfig {
                    client_id: client_id.into(),
                    client_secret: client_secret.into(),
                    redirect_uri: Some(redirect_uri.into()),
                },
                access_token: Arc::new(RwLock::new(access_token)),
                http,
                token_sender,
            }),
            ReceiverStream::new(token_receiver),
        )
    }

    /// Use Bearer authentication via the client_credentials flow.
    ///
    /// This allows you to log in to the api as the account that created the developer application.
    ///
    /// # Panics
    /// This function panics if the TLS backend cannot be initialized, or the resolver cannot load
    /// the system configuration.
    pub fn client_credentials(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
    ) -> Self {
        Self::client_credentials_with_scopes(client_id, client_secret, vec![])
    }

    /// Use Bearer authentication via the client_credentials flow, specifying the scopes for the
    /// token.
    ///
    /// This allows you to log in to the api as the account that created the developer application.
    ///
    /// # Panics
    /// This function panics if the TLS backend cannot be initialized, or the resolver cannot load
    /// the system configuration.
    pub fn client_credentials_with_scopes(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        scopes: Vec<String>,
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to create http client");

        Self::ClientCredentials(ClientCredentials {
            client: ClientConfig {
                client_id: client_id.into(),
                client_secret: client_secret.into(),
                redirect_uri: None,
            },
            scopes,
            access_token: Default::default(),
            http,
        })
    }
}

/// Stored information about the access token
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessToken {
    /// Glimesh access token
    pub access_token: String,

    /// Glimesh refresh token
    pub refresh_token: Option<String>,

    /// Time the token was created
    #[serde(with = "glimesh_protocol::date")]
    pub created_at: DateTime<Utc>,

    /// Seconds after the created_at time when the token expires.
    pub expires_in: i64,

    /// Space separated list of scopes the token has permission for.
    pub scope: Option<String>,

    /// Type of the token, usually `bearer`
    pub token_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Glimesh client id, used to obtain an access token
    pub client_id: String,

    /// Glimesh client secret, used to obtain an access token
    pub client_secret: String,

    /// The same redirect uri that the token was obtained with. Can be None for client_credentials.
    pub redirect_uri: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RefreshableAccessToken {
    client: ClientConfig,
    access_token: Arc<RwLock<AccessToken>>,
    http: reqwest::Client,
    token_sender: mpsc::Sender<AccessToken>,
}

impl RefreshableAccessToken {
    /// Refresh the access token using the refresh token
    pub async fn refresh(&self) -> Result<AccessToken, AuthError> {
        let refresh_token = self
            .access_token
            .read()
            .await
            .refresh_token
            .clone()
            .ok_or(AuthError::MissingRefreshToken)?;
        let res = self
            .http
            .post("https://glimesh.tv/api/oauth/token")
            .form(&[
                ("grant_type", "refresh_token"),
                ("client_id", &self.client.client_id),
                ("client_secret", &self.client.client_secret),
                (
                    "redirect_uri",
                    self.client
                        .redirect_uri
                        .as_ref()
                        .ok_or(AuthError::MissingRedirect)?,
                ),
                ("refresh_token", &refresh_token),
            ])
            .send()
            .await
            .map_err(anyhow::Error::from)?;

        if !res.status().is_success() {
            return Err(AuthError::BadStatus(res.status().as_u16()));
        }

        let new_token: AccessToken = res.json().await.map_err(anyhow::Error::from)?;

        {
            let mut access_token = self.access_token.write().await;
            *access_token = new_token.clone();
        }

        if let Err(err) = self.token_sender.send(new_token.clone()).await {
            tracing::warn!(?err, "failed to send refresh access token to receiver");
        }

        Ok(new_token)
    }

    /// Obtain an access token, will refresh the token if near expiry.
    pub async fn access_token(&self) -> Result<AccessToken, AuthError> {
        let token = self.access_token.read().await.clone();
        let expires_at = token.created_at + Duration::seconds(token.expires_in);
        if (expires_at - Duration::seconds(EXPIRY_BUFFER)) > Utc::now() {
            Ok(token)
        } else {
            self.refresh().await
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClientCredentials {
    client: ClientConfig,
    access_token: Arc<RwLock<Option<AccessToken>>>,
    scopes: Vec<String>,
    http: reqwest::Client,
}

impl ClientCredentials {
    /// Refresh or obtain a new access token using the client credentials
    pub async fn refresh(&self) -> Result<AccessToken, AuthError> {
        let res = self
            .http
            .post("https://glimesh.tv/api/oauth/token")
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", &self.client.client_id),
                ("client_secret", &self.client.client_secret),
                ("scope", &self.scopes.join(" ")),
            ])
            .send()
            .await
            .map_err(anyhow::Error::from)?;

        if !res.status().is_success() {
            return Err(AuthError::BadStatus(res.status().as_u16()));
        }

        let new_token: AccessToken = res.json().await.map_err(anyhow::Error::from)?;

        {
            let mut access_token = self.access_token.write().await;
            access_token.replace(new_token.clone());
        }

        Ok(new_token)
    }

    /// Obtain an access token, will refresh the token if near expiry.
    pub async fn access_token(&self) -> Result<AccessToken, AuthError> {
        let access_token = self.access_token.read().await.clone();
        if let Some(token) = access_token {
            let expires_at = token.created_at + Duration::seconds(token.expires_in);
            if (expires_at - Duration::seconds(EXPIRY_BUFFER)) > Utc::now() {
                return Ok(token);
            }
        }

        self.refresh().await
    }
}
