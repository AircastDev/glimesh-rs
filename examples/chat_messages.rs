use glimesh::{ws::Connection, Auth};
use graphql_client::GraphQLQuery;
use std::env;
use tokio_stream::StreamExt;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "examples/graphql/schema.json",
    query_path = "examples/graphql/channel_details.graphql",
    response_derives = "Debug"
)]
pub struct ChannelDetailsQuery;

#[derive(GraphQLQuery)]
#[graphql(
    schema_path = "examples/graphql/schema.json",
    query_path = "examples/graphql/chat_messages.graphql",
    response_derives = "Debug"
)]
pub struct ChatMessagesSubscription;

#[tokio::main]
async fn main() -> Result<(), glimesh::WebsocketConnectionError> {
    let client_id = env::var("CLIENT_ID").expect("Missing CLIENT_ID env var");

    let auth = Auth::client_id(client_id);
    let conn = Connection::connect(auth).await?;
    let client = conn.into_client();

    let res = client
        .query::<ChannelDetailsQuery>(channel_details_query::Variables {
            username: "JamesTest".into(),
        })
        .await?;

    let channel_id = res.channel.unwrap().id.unwrap();
    let mut messages = client
        .subscribe::<ChatMessagesSubscription>(chat_messages_subscription::Variables {
            id: channel_id.clone(),
        })
        .await?;

    println!("Subscribed to chat messages on channel #{}", channel_id);
    while let Some(msg) = messages.next().await {
        let chat_message = msg.chat_message.unwrap();
        println!(
            "[{}]: {}",
            chat_message.user.displayname.unwrap(),
            chat_message.message.unwrap()
        );
    }

    Ok(())
}
