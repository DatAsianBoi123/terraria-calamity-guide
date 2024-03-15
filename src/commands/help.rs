use poise::{command, serenity_prelude::{Color, Timestamp, CreateEmbed, CreateEmbedFooter, CreateEmbedAuthor}, CreateReply};

use crate::{Context, Result};

#[command(slash_command, description_localized("en-US", "Displays the help page"))]
pub async fn help(ctx: Context<'_>) -> Result {
    let current_user = ctx.serenity_context().cache.current_user().clone();
    ctx.send(CreateReply::default()
        .embed(CreateEmbed::new()
            .title("Help")
            .thumbnail(current_user.avatar_url().unwrap_or_default())
            .description("This bot is designed to help you on your next Calamity playthrough by showing you different loadouts from \
                         various stages of progression. Additionally, you will also be given information on what permanent upgrades \
                         and healing potions are available to you at that stage of the game.\n\
                         Weapons in **bold** are the recommended weapons to use.\n\
                         Weapons in *italics* are support items.\n\
                         Weapons and equipment marked with an asterisk (*) should be used together.")
            .footer(CreateEmbedFooter::new("Loadouts by GitGudWO").icon_url("https://yt3.googleusercontent.com/lFmtL3AfqsklQGMSPcYf1JUwEZYji5rpq3qPtv1tOGGwvsg4AAT7yffTTN1Co74mbrZ4-M6Lnw=s176-c-k-c0x00ffffff-no-rj"))
            .color(Color::DARK_GREEN)
            .timestamp(Timestamp::now())
        )
    ).await?;

    Ok(())
}

