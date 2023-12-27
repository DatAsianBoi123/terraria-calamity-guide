#![deny(unused_crate_dependencies)]
use shuttle_poise as _;
use tokio::sync::RwLock;

use std::{fs::{self, File}, net::SocketAddr, sync::Arc};

use commands::{report::report, db::db};
use issue::Issues;
use loadout_data::LoadoutData;
use poise::{samples::register_globally, FrameworkOptions, serenity_prelude::{Activity, OnlineStatus, GuildId, ChannelId, GuildChannel, Interaction, ComponentType, InteractionResponseType, GatewayIntents, TypeMapKey}, Event, FrameworkContext, FrameworkBuilder};
use rocket::{fs::{FileServer, relative}, routes};

use shuttle_rocket::RocketService;
use shuttle_runtime::{CustomError, Service};
use shuttle_secrets::SecretStore;

use sqlx::{PgPool, Executor};
use tracing::info;

use crate::{commands::{ping::ping, help::help, view_loadout::view_loadout, playthrough::playthrough}, playthrough_data::PlaythroughData, route::invite};

mod loadout_data;
mod playthrough_data;
mod commands;
mod issue;
mod route;

#[macro_export]
macro_rules! str {
    ($s: expr) => {
        $s.to_string()
    };
}

pub type Error = Box<dyn std::error::Error + Send + Sync>;
pub type Result = std::result::Result<(), Error>;
pub type Context<'a> = poise::Context<'a, Data, Error>;

pub struct Data {
    loadouts: LoadoutData,
    pool: PgPool,
    issue_channel: GuildChannel,
}

pub struct Playthroughs;

impl TypeMapKey for Playthroughs {
    type Value = Arc<RwLock<PlaythroughData>>;
}

pub struct IssueData;

impl TypeMapKey for IssueData {
    type Value = Arc<RwLock<Issues>>;
}

struct PoiseRocketService {
    pub poise: FrameworkBuilder<Data, Box<(dyn std::error::Error + Send + Sync + 'static)>>,
    pub rocket: RocketService,
}

#[shuttle_runtime::async_trait]
impl Service for PoiseRocketService {
    async fn bind(mut self, addr: SocketAddr) -> std::result::Result<(), shuttle_runtime::Error> {
        let binder = self.rocket.bind(addr);

        tokio::select! {
            _ = self.poise.run() => {},
            _ = binder => {},
        }

        Ok(())
    }
}

#[shuttle_runtime::main]
async fn poise(
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
    #[shuttle_shared_db::Postgres(
        local_uri = "postgres://DatAsianBoi123:{secrets.NEON_PASS}@ep-rough-star-70439200.us-east-2.aws.neon.tech/neondb"
    )] pool: PgPool,
) -> std::result::Result<PoiseRocketService, shuttle_runtime::Error> {
    let token = secret_store.get("TOKEN").expect("TOKEN not found");

    let schema = fs::read_to_string("static/schema.sql").expect("file exists");
    pool.execute(&schema[..]).await.map_err(CustomError::new)?;

    let framework = poise::Framework::builder()
        .options(FrameworkOptions {
            commands: vec![
                ping(),
                view_loadout(),
                help(),
                playthrough(),
                report(),
                db(),
            ],
            event_handler: |ctx, event, framework, data| {
                Box::pin(event_handler(ctx, event, framework, data))
            },
            ..Default::default()
        })
        .token(token)
        .intents(GatewayIntents::GUILDS)
        .setup(|ctx, ready, framework| {
            Box::pin(async move {
                register_globally(ctx, &framework.options().commands).await?;
                ctx.set_presence(Some(Activity::playing("TModLoader")), OnlineStatus::Online).await;

                let mut data_lock = ctx.data.write().await;
                let playthroughs = PlaythroughData::load(&pool).await;
                let issues = Issues::load(&ctx.http, &pool).await;
                data_lock.insert::<Playthroughs>(Arc::new(RwLock::new(playthroughs)));
                data_lock.insert::<IssueData>(Arc::new(RwLock::new(issues)));

                let guild_id: u64 = secret_store.get("ISSUE_GUILD").and_then(|id| id.parse().ok()).expect("issue guild should be valid and exists");
                let guild_id = GuildId::from(guild_id);

                let channel_id: u64 = secret_store.get("ISSUE_CHANNEL").and_then(|id| id.parse().ok()).expect("issue channel should be valid and exists");
                let channel_id = ChannelId::from(channel_id);

                let channels = guild_id.channels(&ctx.http).await?;
                let issue_channel = channels.get(&channel_id).expect("channel exists");

                let all_guilds = ctx.cache.guild_count();
                let playthroughs = data_lock.get::<Playthroughs>().expect("playthroughs exist").clone();
                let playthroughs = playthroughs.read().await;
                let issues = data_lock.get::<IssueData>().expect("issues exist").clone();
                let issues = issues.read().await;
                info!("loaded {} playthroughs", playthroughs.active_playthroughs.len());
                info!("loaded {} issues", issues.issues.len());
                info!("helping playthroughs in {} guilds", all_guilds);
                info!("ready! logged in as {}", ready.user.tag());
                Ok(Data {
                    loadouts: loadout_data::load_data(File::open("static/loadout_data.json").expect("file exists")),
                    pool,
                    issue_channel: issue_channel.clone(),
                })
            })
        });


    let rocket = rocket::build()
        .mount("/", FileServer::from(relative!("static/public")))
        .mount("/", routes![invite])
        .into();

    Ok(PoiseRocketService { poise: framework, rocket })
}

async fn event_handler(ctx: &poise::serenity_prelude::Context, event: &Event<'_>, _framework: FrameworkContext<'_, Data, Error>, data: &Data) -> Result {
    match event {
        Event::InteractionCreate {
            interaction: Interaction::MessageComponent(interaction)
        } if interaction.data.component_type == ComponentType::Button && interaction.data.custom_id.starts_with("r-") => {
            if let Some(member) = &interaction.member {
                if let Ok(permissions) = member.permissions(&ctx.cache) {
                    if !permissions.administrator() { return Ok(()); }
                    let id: i32 = interaction.data.custom_id[2..].parse().expect("issue id is a number");

                    let data_read = ctx.data.read().await;
                    let issues = data_read.get::<IssueData>().expect("data exists").clone();
                    let mut issue_lock = issues.write().await;
                    let issue = issue_lock.resolve(id, &data.pool).await.expect("issue exists");

                    interaction.create_interaction_response(&ctx.http, |r| {
                        r.kind(InteractionResponseType::UpdateMessage)
                            .interaction_response_data(|edit| edit.set_embed(issue.create_resolved_embed()).components(|c| c))
                    }).await?;
                }
            }
        }
        _ => {},
    }

    Ok(())
}

pub fn ordered_list<S>(vec: &[S]) -> String
where
    S: ToString,
{
    vec.iter()
        .map(|e| str!("1. ") + &e.to_string())
        .fold(String::new(), |prev, curr| prev + "\n" + &curr)
}

pub fn bulleted<S>(vec: &Vec<S>) -> String
where
    S: ToString,
{
    if vec.len() == 1 { return vec[0].to_string() }
    vec.iter()
        .map(|e| str!("- ") + &e.to_string())
        .fold(String::new(), |prev, curr| prev + "\n" + &curr)
}

pub fn bulleted_iter<S, I>(iter: I) -> String
where
    S: ToString,
    I: Iterator<Item = S>,
{
    iter
        .map(|e| str!("- ") + &e.to_string())
        .fold(String::new(), |prev, curr| prev + "\n" + &curr)
}

pub fn bulleted_array<S>(array: &[S]) -> String
where
    S: ToString,
{
    if array.len() == 1 { return array[0].to_string() }
    array.iter()
        .map(|e| str!("- ") + &e.to_string())
        .fold(String::new(), |prev, curr| prev + "\n" + &curr)
}

