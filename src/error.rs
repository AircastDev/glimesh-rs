/// Errors that can occur when interacting with the Glimesh API.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A list of errors returned by graphql.
    /// This is guaranteed to always have at least one error.
    #[error("Graphql error(s): {0:?}")]
    GraphqlErrors(Vec<graphql_client::Error>),

    /// The graphql response contained no errors, but null data.
    /// Its likely this will never really happen if the API is behaving.
    #[error("Missing response data")]
    NoData,

    /// A request returned a bad status code. This may happen if the auth credentials
    /// were wrong for example.
    #[error("{0} response from api")]
    BadStatus(u16),

    /// Some other error occurred.
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}
