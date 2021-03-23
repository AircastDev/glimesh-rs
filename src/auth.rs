use crate::Error;
use chrono::{DateTime, Duration, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

    /// Use Bearer authentication.
    /// This will use the provided refresh token to refresh the access token when/if it expires.
    ///
    /// # Panics
    /// This function panics if the TLS backend cannot be initialized, or the resolver cannot load the system configuration.
    pub fn refreshable_access_token(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        redirect_uri: impl Into<String>,
        access_token: AccessToken,
    ) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to create http client");

        Self::RefreshableAccessToken(RefreshableAccessToken {
            client: ClientConfig {
                client_id: client_id.into(),
                client_secret: client_secret.into(),
                redirect_uri: Some(redirect_uri.into()),
            },
            access_token: Arc::new(RwLock::new(access_token)),
            http,
        })
    }

    /// Use Bearer authentication via the client_credentials flow.
    ///
    /// This allows you to log in to the api as the account that created the developer application.
    ///
    /// # Panics
    /// This function panics if the TLS backend cannot be initialized, or the resolver cannot load the system configuration.
    pub fn client_credentials(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
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
            access_token: Default::default(),
            http,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessToken {
    /// Glimesh access token
    pub access_token: String,

    /// Glimesh refresh token
    pub refresh_token: Option<String>,

    /// Time the token was created
    #[serde(with = "crate::entities::glimesh_date")]
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
}

impl RefreshableAccessToken {
    /// Refresh the access token using the refresh token
    pub async fn refresh(&self) -> Result<AccessToken, Error> {
        let refresh_token = self
            .access_token
            .read()
            .refresh_token
            .clone()
            .ok_or(Error::MissingRefreshToken)?;
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
                        .ok_or(Error::MissingRedirect)?,
                ),
                ("refresh_token", &refresh_token),
            ])
            .send()
            .await
            .map_err(anyhow::Error::from)?;

        if !res.status().is_success() {
            return Err(Error::BadStatus(res.status().as_u16()));
        }

        let new_token: AccessToken = res.json().await.map_err(anyhow::Error::from)?;

        {
            let mut access_token = self.access_token.write();
            *access_token = new_token.clone();
        }

        Ok(new_token)
    }

    /// Obtain an access token, will refresh the token if near expiry.
    pub async fn access_token(&self) -> Result<AccessToken, Error> {
        let token = self.access_token.read().clone();
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
    http: reqwest::Client,
}

impl ClientCredentials {
    /// Refresh or obtain a new access token using the client credentials
    pub async fn refresh(&self) -> Result<AccessToken, Error> {
        let res = self
            .http
            .post("https://glimesh.tv/api/oauth/token")
            .form(&[
                ("grant_type", "client_credentials"),
                ("client_id", &self.client.client_id),
                ("client_secret", &self.client.client_secret),
            ])
            .send()
            .await
            .map_err(anyhow::Error::from)?;

        if !res.status().is_success() {
            return Err(Error::BadStatus(res.status().as_u16()));
        }

        let new_token: AccessToken = res.json().await.map_err(anyhow::Error::from)?;

        {
            let mut access_token = self.access_token.write();
            access_token.replace(new_token.clone());
        }

        Ok(new_token)
    }

    /// Obtain an access token, will refresh the token if near expiry.
    pub async fn access_token(&self) -> Result<AccessToken, Error> {
        let access_token = self.access_token.read().clone();
        if let Some(token) = access_token {
            let expires_at = token.created_at + Duration::seconds(token.expires_in);
            if (expires_at - Duration::seconds(EXPIRY_BUFFER)) > Utc::now() {
                return Ok(token);
            }
        }

        self.refresh().await
    }
}
