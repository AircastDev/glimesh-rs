//! Connection over http using reqwest.
//!
//! You will usually pretty much immediately turn the connection into a Client.
//! E.g.
//! ```rust
//! use glimesh::{http::Connection, Auth};
//! let client = Connection::new(Auth::client_id("<GLIMESH_CLIENT_ID>")).into_client();
//! ```

use crate::{Auth, Client, GlimeshError, HttpConnectionError, MutationConn, QueryConn};
use reqwest::{header, RequestBuilder};
use std::{sync::Arc, time::Duration};

#[derive(Debug)]
struct Config {
    user_agent: String,
    timeout: Duration,
    api_url: String,
    auth: Option<Auth>,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            user_agent: format!("Glimesh Rust / {}", env!("CARGO_PKG_VERSION")),
            timeout: Duration::from_secs(30),
            api_url: String::from("https://glimesh.tv/api/graph"),
            auth: None,
        }
    }
}

/// Configure and build an http [`Connection`].
///
/// ## Usage:
/// ```rust
/// let connection = Connection::builder()
///     .user_agent("My App / 0.1.0")
///     .finish();
/// ```
#[derive(Debug, Default)]
pub struct ConnectionBuilder {
    config: Config,
    http: Option<reqwest::Client>,
}

impl ConnectionBuilder {
    /// Finish building the http connection from the set options.
    ///
    /// # Panics
    /// This function panics if (when an http client hasn't already been provided) the TLS backend
    /// cannot be initialized, or the resolver cannot load the system configuration.
    pub fn finish(self) -> Connection {
        Connection {
            http: self.http.unwrap_or_else(move || {
                reqwest::Client::builder()
                    .user_agent(self.config.user_agent)
                    .timeout(self.config.timeout)
                    .build()
                    .expect("failed to create http client")
            }),
            auth: self.config.auth.map(Arc::new),
            api_url: Arc::new(self.config.api_url),
        }
    }

    /// Set the user agent the http client will identify itself as.
    ///
    /// This defaults to `Glimesh Rust / x.x.x` where `x.x.x` is the version of this package.
    ///
    /// This has no effect if a custom http client is passed in
    pub fn user_agent(mut self, value: impl Into<String>) -> Self {
        self.config.user_agent = value.into();
        self
    }

    /// Set the timeout for requests made to glimesh.
    ///
    /// The default is 30 seconds
    ///
    /// This has no effect if a custom http client is passed in
    pub fn timeout(mut self, value: Duration) -> Self {
        self.config.timeout = value;
        self
    }

    /// Set the base api url used for request.
    /// Useful if running Glimesh locally, or using the old api for example.
    ///
    /// Defaults to `https://glimesh.tv/api/graph`
    pub fn api_url(mut self, value: impl Into<String>) -> Self {
        self.config.api_url = value.into();
        self
    }

    /// Set the auth method used in requests.
    pub fn auth(mut self, auth: Auth) -> Self {
        self.config.auth = Some(auth);
        self
    }

    /// Use an existing http client.
    pub fn http_client(mut self, http: reqwest::Client) -> Self {
        self.http = Some(http);
        self
    }
}

/// Connect to glimesh over http(s).
#[derive(Debug, Clone)]
pub struct Connection {
    http: reqwest::Client,
    auth: Option<Arc<Auth>>,
    api_url: Arc<String>,
}

impl Connection {
    /// Create a [`ConnectionBuilder`] to configure various options.
    pub fn builder() -> ConnectionBuilder {
        ConnectionBuilder::default()
    }

    /// Create a connection with the default options.
    pub fn new(auth: Auth) -> Self {
        ConnectionBuilder::default().auth(auth).finish()
    }

    /// Create a client with reference to this connection
    pub fn as_client(&self) -> Client<&Self> {
        Client::new(self)
    }

    /// Create a client with a clone of this connection
    pub fn to_client(&self) -> HttpClient {
        Client::new(self.clone())
    }

    /// Convert this connection into a client
    pub fn into_client(self) -> HttpClient {
        Client::new(self)
    }

    /// Create a copy of this connection with a difference auth method.
    pub fn clone_with_auth(&self, auth: Auth) -> Self {
        Self {
            api_url: self.api_url.clone(),
            http: self.http.clone(),
            auth: Some(Arc::new(auth)),
        }
    }

    async fn request<Q>(
        &self,
        variables: Q::Variables,
    ) -> Result<Q::ResponseData, HttpConnectionError>
    where
        Q: graphql_client::GraphQLQuery,
    {
        let req = self
            .http
            .post(self.api_url.as_ref())
            .json(&Q::build_query(variables));

        let res = self
            .apply_auth(req)
            .await?
            .send()
            .await
            .map_err(anyhow::Error::from)?;

        if !res.status().is_success() {
            return Err(HttpConnectionError::BadStatus(res.status().as_u16()));
        }

        let res: graphql_client::Response<Q::ResponseData> =
            res.json().await.map_err(anyhow::Error::from)?;

        if let Some(errs) = res.errors {
            if !errs.is_empty() {
                return Err(GlimeshError::GraphqlErrors(errs).into());
            }
        }

        let data = res.data.ok_or(GlimeshError::NoData)?;
        Ok(data)
    }

    async fn apply_auth(&self, req: RequestBuilder) -> Result<RequestBuilder, HttpConnectionError> {
        match self.auth.as_ref().map(|a| a.as_ref()) {
            Some(Auth::ClientId(client_id)) => {
                Ok(req.header(header::AUTHORIZATION, format!("Client-ID {}", client_id)))
            }
            Some(Auth::AccessToken(access_token)) => Ok(req.bearer_auth(access_token)),
            Some(Auth::RefreshableAccessToken(token)) => {
                let tokens = token.access_token().await?;
                Ok(req.bearer_auth(tokens.access_token))
            }
            Some(Auth::ClientCredentials(client_credentials)) => {
                let tokens = client_credentials.access_token().await?;
                Ok(req.bearer_auth(tokens.access_token))
            }
            None => Ok(req),
        }
    }
}

#[async_trait]
impl QueryConn for Connection {
    type Error = HttpConnectionError;

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
    type Error = HttpConnectionError;

    async fn mutate<Q>(&self, variables: Q::Variables) -> Result<Q::ResponseData, Self::Error>
    where
        Q: graphql_client::GraphQLQuery,
        Q::Variables: Send + Sync,
    {
        self.request::<Q>(variables).await
    }
}

/// Type alias for a http backed client
pub type HttpClient = Client<Connection>;
