use poise::{command, serenity_prelude::{Timestamp, Color}};
use crate::{Context, Result};

#[command(slash_command, description_localized("en-US", "Pings the bot"))]
pub async fn ping(ctx: Context<'_>) -> Result {
    if let Context::Application(ctx) = ctx {
        let now = Timestamp::now();
        let latency = now.timestamp_millis() - ctx.created_at().with_timezone(&now.timezone()).timestamp_millis();
        ctx.send(|f| f
                 .embed(|embed| embed
                        .title("Pong! :ping_pong:")
                        .color(Color::BLUE)
                        .field("Latency", format!("{latency}ms"), false)
                        .timestamp(now))).await?;
    }
    Ok(())
}

