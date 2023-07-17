use std::{sync::Arc, path::PathBuf};

use data::LoadoutData;
use poise::{samples::register_globally, FrameworkOptions, command, serenity_prelude::{Timestamp, Color, Activity, OnlineStatus}};
use serenity::prelude::GatewayIntents;

use shuttle_poise::ShuttlePoise;
use shuttle_secrets::SecretStore;

use tracing::info;

use crate::data::{CalamityClass, Stage};

mod data;

type Error = Box<dyn std::error::Error + Send + Sync>;
type Result = std::result::Result<(), Error>;
type Context<'a> = poise::Context<'a, LoadoutData, Error>;

#[macro_export]
macro_rules! str {
    ($s: literal) => {
        $s.to_string()
    };
}

#[macro_export]
macro_rules! str_slice {
    [$($s: literal),*] => {
        [
            $($s.to_string(),)*
        ]
    }
}

#[macro_export]
macro_rules! str_vec {
    [$($s: literal),*] => {
        vec![$($s.to_string(),)*]
    }
}

#[command(slash_command, description_localized("en-US", "Pings the bot"))]
async fn ping(ctx: Context<'_>) -> Result {
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

#[command(slash_command, description_localized("en-US", "Displays the help page"))]
async fn help(ctx: Context<'_>) -> Result {
    ctx.send(|m| {
        m.embed(|e| {
            e.title("Help")
                .thumbnail(ctx.serenity_context().cache.current_user().avatar_url().unwrap_or(String::new()))
                .description("This bot is designed to help you on your next Calamity playthrough by showing you different loadouts from \
                             various stages of progression. Additionally, you will also be given information on what permanent upgrades \
                             and healing potions are available to you at that stage of the game.\n\
                             Weapons in **bold** are the recommended weapons to use.\n\
                             Weapons in *italics* are support items.\n\
                             Weapons and equipment marked with an asterisk (*) should be used together.")
                .footer(|f| f.text("Loadouts by GitGudWO").icon_url("https://yt3.googleusercontent.com/lFmtL3AfqsklQGMSPcYf1JUwEZYji5rpq3qPtv1tOGGwvsg4AAT7yffTTN1Co74mbrZ4-M6Lnw=s176-c-k-c0x00ffffff-no-rj"))
                .color(Color::DARK_GREEN)
                .timestamp(Timestamp::now())
        })
    }).await?;

    Ok(())
}

#[command(slash_command, rename = "sendrules", required_permissions = "MANAGE_GUILD")]
async fn send_rules(ctx: Context<'_>) -> Result {
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

#[command(
    slash_command,
    description_localized("en-US", "Views the recommended loadout during a specific stage of progression"),
    rename = "viewloadout"
)]
async fn view_loadout(
    ctx: Context<'_>,
    #[description = "The class"] class: CalamityClass,
    #[description = "The stage of the game"] stage: Option<Stage>,
) -> Result {
    let stage = stage.unwrap_or(Stage::PreBoss);
    if let Context::Application(ctx) = ctx {
        fn bulleted<S>(vec: &Vec<S>) -> String
        where
            S: ToString,
        {
            if vec.len() == 1 { return vec[0].to_string() }
            vec.iter()
                .map(|e| str!("- ") + &e.to_string())
                .fold(String::new(), |prev, curr| prev + "\n" + &curr)
        }
        fn bulleted_array<S>(array: &[S]) -> String
        where
            S: ToString,
        {
            if array.len() == 1 { return array[0].to_string() }
            array.iter()
                .map(|e| str!("- ") + &e.to_string())
                .fold(String::new(), |prev, curr| prev + "\n" + &curr)
        }

        let stage_data = Arc::new(ctx.data().get(&stage).expect("stage exists"));

        {
            let stage_data = stage_data.clone();
            let loadout = if let Some(l) = stage_data.loadouts.get(&class) { l } else {
                ctx.say("This class is currently under construction!").await?;
                return Ok(())
            };

            macro_rules! loadout_msg {
                ($creator: expr) => {
                    $creator.embed(|embed| {
                        embed
                            .title(format!("{class} - {stage}"))
                            .author(|a| a.name(&ctx.author().name).icon_url(ctx.author().avatar_url().unwrap_or(String::new())))
                            .thumbnail(stage.img())
                            .field("<:armor:1129548766857404576> Armor", &loadout.armor, true)
                            .field("<:weapons:1129556916805304410> Weapons", bulleted_array(&loadout.weapons), true)
                            .field("<:equipment:1129549501712048178> Equipment", bulleted(&loadout.equipment), true)
                            .field("** **", "** **", false) // force next field to be on next row
                            .fields(loadout.extra.iter().map(|(title, list)| (title, bulleted(list), true)))
                            .field("** **", "** **", false)
                            .field("<:healing_potion:1129549725331370075> Healing Potion", stage_data.potion.to_string(), true)
                            .color(Color::DARK_RED)
                            .footer(|f| f.text("Loadouts by GitGudWO").icon_url("https://yt3.googleusercontent.com/lFmtL3AfqsklQGMSPcYf1JUwEZYji5rpq3qPtv1tOGGwvsg4AAT7yffTTN1Co74mbrZ4-M6Lnw=s176-c-k-c0x00ffffff-no-rj"))
                            .timestamp(Timestamp::now());
                        if let Some(powerups) = &stage_data.powerups {
                            embed.field("<:powerups:1129550131000254614> Permanent Powerups", bulleted(powerups), true);
                        }
                        embed
                    })
                }
            }

            ctx.send(|f| loadout_msg!(f)).await?;
        }
    }
    Ok(())
}

#[shuttle_runtime::main]
async fn poise(
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
    #[shuttle_static_folder::StaticFolder] static_folder: PathBuf,
) -> ShuttlePoise<LoadoutData, Error> {
    let token = secret_store.get("TOKEN").expect("TOKEN not found");

    let framework = poise::Framework::builder()
        .options(FrameworkOptions {
            commands: vec![
                ping(),
                view_loadout(),
                help(),
                send_rules(),
            ],
            ..Default::default()
        })
        .token(token)
        .intents(GatewayIntents::GUILDS)
        .setup(|ctx, ready, framework| {
            Box::pin(async move {
                register_globally(ctx, &framework.options().commands).await?;
                ctx.set_presence(Some(Activity::playing("TModLoader")), OnlineStatus::Online).await;
                info!("ready! logged in as {}", ready.user.tag());
                Ok(data::load_data(static_folder))
            })
        }).build().await.map_err(shuttle_runtime::CustomError::new)?;

    Ok(framework.into())
}

