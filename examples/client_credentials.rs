use glimesh::{http::Connection, Auth};
use graphql_client::GraphQLQuery;
use std::env;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "examples/graphql/schema.json",
    query_path = "examples/graphql/current_user.graphql",
    response_derives = "Debug"
)]
pub struct CurrentUserQuery;

#[tokio::main]
async fn main() -> Result<(), glimesh::Error> {
    let client_id = env::var("CLIENT_ID").expect("Missing CLIENT_ID env var");
    let client_secret = env::var("CLIENT_SECRET").expect("Missing CLIENT_SECRET env var");

    let auth = Auth::client_credentials(client_id, client_secret);
    let conn = Connection::new(auth);
    let client = conn.into_client();

    let res = client
        .query::<CurrentUserQuery>(current_user_query::Variables)
        .await?;

    println!("I am: {:#?}", res.myself);

    Ok(())
}
