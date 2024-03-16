use poise::{command, serenity_prelude::{Timestamp, Color, CreateEmbed}, CreateReply};
use crate::{Context, PoiseResult};

#[command(slash_command, description_localized("en-US", "Pings the bot"))]
pub async fn ping(ctx: Context<'_>) -> PoiseResult {
    if let Context::Application(ctx) = ctx {
        let now = Timestamp::now();
        let latency = now.timestamp_millis() - ctx.created_at().with_timezone(&now.timezone()).timestamp_millis();
        ctx.send(CreateReply::default()
            .embed(CreateEmbed::new()
                .title("Pong! :ping_pong:")
                .color(Color::BLUE)
                .field("Latency", format!("{latency}ms"), false)
                .timestamp(now))).await?;
    }
    Ok(())
}

