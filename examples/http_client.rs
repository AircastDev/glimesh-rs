use glimesh::{http::Connection, Auth};
use graphql_client::GraphQLQuery;
use std::env;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "examples/graphql/schema.json",
    query_path = "examples/graphql/user_details.graphql",
    response_derives = "Debug"
)]
pub struct UserDetailsQuery;

#[tokio::main]
async fn main() -> Result<(), glimesh::HttpConnectionError> {
    let client_id = env::var("CLIENT_ID").expect("Missing CLIENT_ID env var");

    let auth = Auth::client_id(client_id);
    let conn = Connection::new(auth);
    let client = conn.into_client();

    let res = client
        .query::<UserDetailsQuery>(user_details_query::Variables {
            username: "James".into(),
        })
        .await?;

    let user = res.user;
    println!("User details: {:#?}", user);
    Ok(())
}
