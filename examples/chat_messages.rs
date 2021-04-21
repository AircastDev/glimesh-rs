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
    tracing_subscriber::fmt()
        .with_env_filter("info,glimesh=debug")
        .init();

    let client_id = env::var("CLIENT_ID").expect("Missing CLIENT_ID env var");
    let api_url = env::var("API_URL")
        .unwrap_or_else(|_| String::from("wss://glimesh.tv/api/socket/websocket"));
    let channel_name = env::var("CHANNEL_NAME").unwrap_or_else(|_| String::from("JamesTest"));

    let auth = Auth::client_id(client_id);
    let conn = Connection::builder().api_url(api_url).connect(auth).await?;
    let client = conn.into_client();

    let res = client
        .query::<ChannelDetailsQuery>(channel_details_query::Variables {
            username: channel_name,
        })
        .await?;

    let channel_id = res.channel.unwrap().id.unwrap();
    let mut messages = client
        .subscribe::<ChatMessagesSubscription>(chat_messages_subscription::Variables {
            id: channel_id.clone(),
        })
        .await?;

    // `messages` can be sent to a different thread if desired
    tokio::spawn(async move {
        tracing::info!("Subscribed to chat messages on channel #{}", channel_id);
        while let Some(msg) = messages.next().await {
            let chat_message = msg.chat_message.unwrap();
            tracing::info!(
                "[{}]: {}",
                chat_message.user.displayname.unwrap(),
                chat_message.message.unwrap()
            );
        }
    })
    .await
    .unwrap();

    Ok(())
}
