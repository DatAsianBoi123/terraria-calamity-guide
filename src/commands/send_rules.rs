use poise::command;
use serenity::utils::Color;

use crate::{Context, Result};

#[command(slash_command, rename = "sendrules", required_permissions = "MANAGE_GUILD")]
pub async fn send_rules(ctx: Context<'_>) -> Result {
    ctx.channel_id().send_message(ctx, |m| {
        m.embed(|embed| {
            embed
                .title("Rules")
                .description("**1. Be Respectful**\
                             Just be respectful to other guild members.\n\
                             **2. Limit Profanity**\
                             Don't spam swear words.\n\
                             **3. Avoid NSFW Content**\
                             Do not post any sort of NSFW or controversial topic in any channel.\n\
                             **4. No Spamming**\
                             Do not spam random messages.\n\
                             **5. No Advertising**\
                             Do not advertise anything in any channel.\n\
                             **6. English Only**\
                             Having every message be in English helps us moderate better.")
                .color(Color::from_rgb(255, 255, 255))
        })
    }).await?;
    ctx.send(|m| m.content("Done!").ephemeral(true)).await?;
    Ok(())
}
