use std::{vec, result::Result as StdResult, convert::Into};

use poise::{command, serenity_prelude::{User, Color, Timestamp, CacheHttp, CreateEmbed, CreateMessage, CreateEmbedFooter}, ChoiceParameter, CreateReply};
use rocket::futures::future;
use sqlx::types::chrono::Utc;

use crate::{
    Context,
    PoiseResult,
    loadout_data::{CalamityClass, Stage, LoadoutData},
    playthrough_data::{FinishPlaythroughError, Player, JoinPlayerError, LeaveError, StartPlaythroughError, KickError, ProgressError, Playthrough},
    str,
    bulleted,
    ordered_list,
    Playthroughs,
};

#[command(
    slash_command,
    subcommands("list", "view", "create", "end", "start", "join", "kick", "leave", "progress"),
    description_localized("en-US", "All playthrough related commands"),
)]
pub async fn playthrough(_: Context<'_>) -> PoiseResult {
    Ok(())
}

#[command(slash_command, owners_only, ephemeral)]
async fn list(ctx: Context<'_>) -> PoiseResult {
    let data_lock = ctx.serenity_context().data.read().await;
    let read_lock = data_lock.get::<Playthroughs>().expect("get data").clone();
    let playthroughs = &read_lock.read().await.active_playthroughs;
    let owners = future::join_all(playthroughs.iter()
        .map(|(owner, playthrough)| async {
            let owner = owner.to_user(&ctx).await.expect("user exists");
            format!("{} ({} total players) - {}", owner.name, playthrough.players.len(), playthrough.stage.name())
        })).await;
    ctx.say(ordered_list(&owners)).await?;
    Ok(())
}

#[command(slash_command, description_localized("en-US", "Views the playthrough that you or another player is in"))]
async fn view(
    ctx: Context<'_>,
    #[description = "The user's playthrough to check"] #[rename = "user"] other: Option<User>
) -> PoiseResult {
    let user = other.as_ref().unwrap_or(ctx.author());
    let data_lock = ctx.serenity_context().data.read().await;
    let data_lock = data_lock.get::<Playthroughs>().expect("work").clone();
    let data_lock = &data_lock.read().await;
    if !data_lock.all_users.contains(&user.id) {
        ctx.say(format!("{} not currently in a playthrough", other.map(|_| "That user is").unwrap_or("You are"))).await?;
        return Ok(());
    }
    let playthrough = data_lock.active_playthroughs.get(&user.id)
        .or_else(|| {
            data_lock.active_playthroughs.iter()
                .find_map(|(_, playthrough)| playthrough.players.iter().find(|player| player.user_id == user.id).and(Some(playthrough)))
        }).expect("found playthrough player is in");

    let owner = playthrough.owner.to_user(ctx).await.expect("owner is a user");
    let player_list = future::join_all(playthrough.players.iter()
        .map(|p| async move { format!("{} - {}{}", p.user_id.to_user(ctx).await.expect("player is user").name, p.class.name(), p.class.emoji()) })).await;

    let current_user = ctx.serenity_context().cache.current_user().clone();

    ctx.send(CreateReply::default()
        .embed(CreateEmbed::new()
            .title(format!("{}'s Playthrough", owner.name))
            .thumbnail(current_user.avatar_url().unwrap_or_default())
            .field("Players", bulleted(&player_list).to_string(), false)
            .field("Date Started", match playthrough.started {
                Some(date) => format!("<t:{}:D>", date.timestamp()),
                None => str!("Playthrough hasn't started yet"),
            }, true)
            .field("Game Stage", playthrough.stage.name(), true)
            .color(Color::FOOYOO)
            .footer(CreateEmbedFooter::new("Loadouts by GitGudWO").icon_url("https://yt3.googleusercontent.com/lFmtL3AfqsklQGMSPcYf1JUwEZYji5rpq3qPtv1tOGGwvsg4AAT7yffTTN1Co74mbrZ4-M6Lnw=s176-c-k-c0x00ffffff-no-rj"))
            .timestamp(Timestamp::now())
        )
    ).await?;

    Ok(())
}

#[command(slash_command, description_localized("en-US", "Creates a new playthrough"))]
async fn create(ctx: Context<'_>, #[description = "The class you're playing in this playthrough"] class: CalamityClass) -> PoiseResult {
    ctx.defer().await?;

    let data_lock = ctx.serenity_context().data.read().await;
    let data_lock = data_lock.get::<Playthroughs>().expect("work").clone();
    let mut write_lock = data_lock.write().await;
    let create_res = write_lock.create(
        ctx.author(),
        class,
        &ctx.data().pool,
    ).await;
    if create_res.is_ok() {
        ctx.say("Successfully created a new playthrough").await?;
    } else {
        ctx.say("You are already in a playthrough!").await?;
    }

    Ok(())
}

#[command(slash_command, description_localized("en-US", "Ends the playthrough you're in"))]
async fn end(ctx: Context<'_>) -> PoiseResult {
    ctx.defer().await?;

    let data_lock = ctx.serenity_context().data.read().await;
    let data_lock = data_lock.get::<Playthroughs>().expect("work").clone();
    let mut write_lock = data_lock.write().await;
    let finish_res = write_lock.end(ctx.author(), &ctx.data().pool).await;
    match finish_res {
        Ok(playthrough) => {
            let time_spent = playthrough.started
                .map(|started| {
                    macro_rules! find_highest {
                        (($f: expr, $t: literal), ($l: expr, $n: literal)) => {{
                            if $f > 0 {
                                format!("{} {}", $f, $t)
                            } else {
                                format!("{} {}", $l, $n)
                            }
                        }};
                        (($f: expr, $t: literal), ($l: expr, $n: literal), $(($a: expr, $o: expr)),+) => {{
                            if $f > 0 {
                                format!("{} {}", $f, $t)
                            }
                            $(
                                else if $a > 0 {
                                    format!("{} {}", $a, $o)
                                }
                             )*
                            else {
                                format!("{} {}", $l, $n)
                            }
                        }};
                    }

                    let duration = Utc::now().naive_utc() - started;
                    find_highest!(
                        (duration.num_days(), "days"),
                        (duration.num_seconds(), "seconds"),
                        (duration.num_hours(), "hours"),
                        (duration.num_minutes(), "minutes")
                    )
                })
                .unwrap_or(str!("Playthrough never started"));
            ctx.say(format!("Successfully ended your playthrough\nTotal playthrough time: {time_spent}")).await?
        },
        Err(FinishPlaythroughError::NotOwner) => ctx.say("You are not the owner of the playthrough you are in").await?,
        Err(FinishPlaythroughError::NotInPlaythrough) => ctx.say("You are not in a playthrough").await?,
    };

    Ok(())
}

#[command(slash_command, description_localized("en-US", "Starts your created playthrough"))]
async fn start(ctx: Context<'_>) -> PoiseResult {
    ctx.defer().await?;

    let data_lock = ctx.serenity_context().data.read().await;
    let data_lock = data_lock.get::<Playthroughs>().expect("work").clone();
    let mut write_lock = data_lock.write().await;
    let start_res = write_lock.start(ctx.author(), &ctx.data().pool).await;

    match start_res {
        Ok(()) => {
            let playthrough = write_lock.active_playthroughs.get(&ctx.author().id).expect("thing exists");
            let dm_results = resend_loadouts(ctx, playthrough, &ctx.data().loadouts).await;
            let error_futures = dm_results.into_iter().map(|(user, dm_res)| async move {
                if dm_res.is_err() {
                    ctx.say(format!("{user}, I can't DM you! Please enable DMs if you want me to automatically send you loadouts!")).await
                        .expect("can message");
                }
            });
            future::join_all(error_futures).await;
            ctx.say("Successfully started your playthrough!").await?
        },
        Err(StartPlaythroughError::NotOwner) => ctx.say("You are not the owner of the playthrough you are in").await?,
        Err(StartPlaythroughError::AlreadyStarted) => ctx.say("This playthrough has already started").await?,
        Err(StartPlaythroughError::NotInPlaythrough) => ctx.say("You are not in a playthrough").await?,
    };

    Ok(())
}

#[command(slash_command, description_localized("en-US", "Joins another player's playthrough"))]
async fn join(
    ctx: Context<'_>,
    #[description = "The owner of the playthrough"] owner: User,
    #[description = "The class you want to play in this playthrough"] class: CalamityClass,
) -> PoiseResult {
    ctx.defer().await?;

    let data_lock = ctx.serenity_context().data.read().await;
    let data_lock = data_lock.get::<Playthroughs>().expect("work").clone();
    let mut write_lock = data_lock.write().await;
    let add_res = write_lock.join_player(
        &owner,
        Player { user_id: ctx.author().id, class },
        &ctx.data().pool,
    ).await;
    match add_res {
        Ok(()) => ctx.say(format!("Successfully joined {}'s playthrough", owner)).await?,
        Err(JoinPlayerError::PlayerNotInPlaythrough) => ctx.say("That player is not in a playthrough").await?,
        Err(JoinPlayerError::PlayerNotOwner) => ctx.say("That player is not the owner of the playthrough they are in").await?,
        Err(JoinPlayerError::AlreadyInPlaythrough) => ctx.say("You are already in a playthrough").await?,
    };

    Ok(())
}

#[command(slash_command, description_localized("en-US", "Kicks another player from your playthrough"))]
async fn kick(ctx: Context<'_>, #[description = "The player you want to kick"] player: User) -> PoiseResult {
    ctx.defer().await?;

    let data_lock = ctx.serenity_context().data.read().await;
    let data_lock = data_lock.get::<Playthroughs>().expect("work").clone();
    let mut write_lock = data_lock.write().await;
    let kick_res = write_lock.kick(ctx.author(), &player, &ctx.data().pool).await;
    match kick_res {
        Ok(()) => ctx.say(format!("Successfully kicked {} from your playthrough", player)).await?,
        Err(KickError::NotOwner) => ctx.say("You are not the owner of the playthrough you are in").await?,
        Err(KickError::PlayerNotInPlaythrough) => ctx.say("That player is not in a playthrough").await?,
        Err(KickError::NotInPlaythrough) => ctx.say("You are not in a playthrough").await?,
        Err(KickError::OwnerOfPlaythrough) => ctx.say("You cannot kick the owner of the playthrough").await?,
    };

    Ok(())
}

#[command(slash_command, description_localized("en-US", "Leaves the playthrough you are in"))]
async fn leave(ctx: Context<'_>) -> PoiseResult {
    ctx.defer().await?;

    let data_lock = ctx.serenity_context().data.read().await;
    let data_lock = data_lock.get::<Playthroughs>().expect("work").clone();
    let mut write_lock = data_lock.write().await;
    let leave_res = write_lock.leave(ctx.author(), &ctx.data().pool).await;
    match leave_res {
        Ok(playthrough) => ctx.say(format!("Successfully left <@{}>'s playthrough", playthrough.owner)).await?,
        Err(LeaveError::NotInPlaythrough) => ctx.say("You are not in a playthrough").await?,
        Err(LeaveError::OwnerOfPlaythrough) => ctx.say("You cannot leave the playthrough you are an owner of").await?,
    };

    Ok(())
}

#[command(slash_command, description_localized("en-US", "Changes your progression stage in the playthrough"))]
async fn progress(
    ctx: Context<'_>,
    #[description = "The new stage to progress to. Leaving this blank advances the stage by 1"] stage: Option<Stage>,
) -> PoiseResult {
    ctx.defer().await?;

    let data_lock = ctx.serenity_context().data.read().await;
    let data_lock = data_lock.get::<Playthroughs>().expect("work").clone();
    let mut write_lock = data_lock.write().await;
    let progress_res = write_lock.progress(ctx.author(), stage, &ctx.data().pool).await;
    match progress_res {
        Ok(playthrough) => {
            if playthrough.started.is_some() {
                resend_loadouts(ctx, playthrough, &ctx.data().loadouts).await;
            }
            let progress_str = format!("Progressed to stage `{}`", playthrough.stage.name());
            if playthrough.started.is_some() {
                ctx.say(progress_str).await?
            } else {
                ctx.say(format!("{progress_str}\n \
                        Note: You have not started your playthrough yet! This bot will only automatically send loadouts when the playthrough \
                        has started.\n \
                        Hint: Start a playthrough with `/playthrough start`")).await?
            }
        },
        Err(ProgressError::NotInPlaythrough) => ctx.say("You are not in a playthrough").await?,
        Err(ProgressError::NotOwner) => ctx.say("You are not the owner of the playthrough you are in").await?,
        Err(ProgressError::LastStage) => ctx.say("You are already on the last stage of the game").await?,
    };

    Ok(())
}

async fn resend_loadouts(http: impl CacheHttp, playthrough: &Playthrough, loadouts: &LoadoutData) -> Vec<(User, StdResult<(), poise::serenity_prelude::Error>)> {
    let dm_futures = playthrough.players.iter().map(|player| {
        let http = http.http();
        async move {
            let user = player.user_id.to_user(&http).await.expect("player id is a user");
            let stage_data = loadouts.get(&playthrough.stage).expect("loadout exists");
            let dm_res = user.direct_message(&http, CreateMessage::new()
                .embed(stage_data.create_embed(&user, player.class, playthrough.stage))).await.map(|_| ());
            (user, dm_res)
        }
    });
    future::join_all(dm_futures).await
}

