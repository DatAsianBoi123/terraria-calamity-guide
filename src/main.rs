use std::{env, sync::Arc};

use data::LoadoutData;
use poise::{samples::register_globally, FrameworkOptions, command, serenity_prelude::{Timestamp, Color}};
use serenity::prelude::GatewayIntents;
use dotenv::dotenv;

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
            let loadout = stage_data.loadouts.get(&class).expect("class exists");

            macro_rules! loadout_msg {
                ($creator: expr) => {
                    $creator.embed(|embed| {
                        embed
                            .title(format!("{class} - {stage}"))
                            .author(|a| a.name("datasianboi123").icon_url("https://tinyurl.com/5n7ny7es"))
                            .thumbnail(stage.img())
                            .field("<:armor:1129548766857404576> Armor", &loadout.armor, true)
                            .field("<:weapons:1129556916805304410> Weapons", bulleted_array(&loadout.weapons), true)
                            .field("<:equipment:1129549501712048178> Equipment", bulleted(&loadout.equipment), true)
                            .color(Color::DARK_RED)
                            .footer(|f| f.text("Loadouts by GitGudWO"))
                            .timestamp(Timestamp::now());
                        let mut extra_iter = loadout.extra.iter().peekable();
                        let mut first = true;
                        while let Some((title, list)) = extra_iter.next() {
                            embed.field(title, bulleted(list), !first || extra_iter.peek().is_some());
                            first = false;
                        }
                        embed.field("<:healing_potion:1129549725331370075> Healing Potion", stage_data.potion.to_string(), true);
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

#[tokio::main]
async fn main() {
    dotenv().ok();

    let framework = poise::Framework::builder()
        .options(FrameworkOptions {
            commands: vec![
                ping(),
                view_loadout(),
            ],
            ..Default::default()
        })
        .token(env::var("token").expect("token"))
        .intents(GatewayIntents::GUILDS)
        .setup(|ctx, ready, framework| {
            Box::pin(async move {
                register_globally(ctx, &framework.options().commands).await?;
                println!("registered commands");
                println!("ready! logged in as {}", ready.user.tag());
                Ok(data::load_data())
            })
        });

    framework.run().await.unwrap();
}

