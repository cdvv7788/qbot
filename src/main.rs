use futures_util::StreamExt;
use std::{env, error::Error, sync::Arc};
use twilight_cache_inmemory::{InMemoryCache, ResourceType};
use twilight_gateway::{Cluster, Event};
use twilight_http::Client as HttpClient;
use twilight_model::gateway::Intents;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let token = env::var("DISCORD_TOKEN")?;

    let (cluster, mut events) = Cluster::new(
        token.to_owned(),
        Intents::MESSAGE_CONTENT | Intents::GUILD_MESSAGES,
    )
    .await?;
    let cluster = Arc::new(cluster);

    // Start up the cluster
    let cluster_spawn = Arc::clone(&cluster);

    // Start all shards in the cluster in the background
    tokio::spawn(async move {
        cluster_spawn.up().await;
    });

    // HTTP is separated from the gateway, so create a new client
    let http = Arc::new(HttpClient::new(token));

    // Since we care only about new messages, make the cache only
    // cache new messages
    let cache = InMemoryCache::builder()
        .resource_types(ResourceType::MESSAGE)
        .build();

    // Process each event as they come in
    while let Some((shard_id, event)) = events.next().await {
        // Update the cache with the event
        cache.update(&event);

        tokio::spawn(handle_event(shard_id, event, Arc::clone(&http)));
    }
    Ok(())
}

async fn handle_event(
    shard_id: u64,
    event: Event,
    http: Arc<HttpClient>,
) -> Result<(), Box<dyn Error + Send + Sync>> {
    match event {
        Event::MessageCreate(msg) if msg.content == "!ping" => {
            http.create_message(msg.channel_id)
                .content("Pong!")?
                .exec()
                .await?;
        }
        Event::MessageCreate(msg) => {
            println!("message was {:?}", msg)
        }
        Event::ShardConnected(_) => {
            println!("Connected on shard {shard_id}")
        }
        // Other events here
        new_event => {
            println!("New Event: {:?}", new_event);
        }
    }
    Ok(())
}
