use std::{collections::{HashMap, HashSet}, vec::Vec, convert::Into};

use multimap::MultiMap;
use num_traits::{FromPrimitive, ToPrimitive};
use poise::serenity_prelude::{UserId, User};
use sqlx::{PgPool, types::{BigDecimal, chrono::{NaiveDateTime, Utc}}};
use tracing::info;

use crate::loadout_data::{CalamityClass, Stage};

type RawPlaythrough = (BigDecimal, i16, Option<NaiveDateTime>);

type RawPlayer = (BigDecimal, BigDecimal, i16);

pub struct InPlaythroughError;

pub enum FinishPlaythroughError {
    NotInPlaythrough,
    NotOwner,
}

pub enum StartPlaythroughError {
    NotInPlaythrough,
    NotOwner,
    AlreadyStarted,
}

pub enum JoinPlayerError {
    AlreadyInPlaythrough,
    PlayerNotInPlaythrough,
    PlayerNotOwner,
}

pub enum KickError {
    NotOwner,
    PlayerNotInPlaythrough,
    NotInPlaythrough,
    OwnerOfPlaythrough,
}

pub enum LeaveError {
    NotInPlaythrough,
    OwnerOfPlaythrough,
}

pub enum ProgressError {
    NotInPlaythrough,
    NotOwner,
    LastStage,
}

pub struct PlaythroughData {
    pub active_playthroughs: HashMap<UserId, Playthrough>,
    pub all_users: HashSet<UserId>,
}

impl PlaythroughData {
    pub async fn create(&mut self, owner: &User, class: CalamityClass, pool: &PgPool) -> Result<&Playthrough, InPlaythroughError> {
        if self.all_users.contains(&owner.id) { return Err(InPlaythroughError) }

        let owner_id = BigDecimal::from(owner.id.get());

        sqlx::query("INSERT INTO playthroughs(owner, stage) VALUES ($1, $2)")
            .bind(owner_id.clone())
            .bind(Stage::default() as i16)
            .execute(pool).await.expect("valid query");

        sqlx::query("INSERT INTO playthrough_players(playthrough_owner, user_id, class) VALUES ($1, $2, $3)")
            .bind(owner_id.clone())
            .bind(owner_id)
            .bind(class as i16)
            .execute(pool).await.expect("value query");

        let playthrough = Playthrough {
            owner: owner.id,
            players: vec![Player { user_id: owner.id, class }],
            stage: Default::default(),
            started: None,
        };

        let owner_id = playthrough.owner;
        self.active_playthroughs.insert(owner_id, playthrough);
        self.all_users.insert(owner_id);

        info!("{} ({}) created a playthrough", owner.name, owner.id);

        Ok(self.active_playthroughs.get(&owner.id).expect("playthrough exists"))
    }

    pub async fn end(&mut self, owner: &User, pool: &PgPool) -> Result<Playthrough, FinishPlaythroughError> {
        if !self.active_playthroughs.contains_key(&owner.id) {
            return if self.all_users.contains(&owner.id) {
                Err(FinishPlaythroughError::NotOwner)
            } else {
                Err(FinishPlaythroughError::NotInPlaythrough)
            }
        }

        sqlx::query("DELETE FROM playthroughs WHERE owner = $1")
            .bind(BigDecimal::from(owner.id.get()))
            .execute(pool).await.expect("query works");

        let playthrough = self.active_playthroughs.remove(&owner.id).expect("owner is in playthrough");
        playthrough.players.iter().for_each(|player| {
            self.all_users.remove(&player.user_id);
        });

        Ok(playthrough)
    }

    pub async fn start(&mut self, owner: &User, pool: &PgPool) -> Result<(), StartPlaythroughError> {
        let playthrough = match self.active_playthroughs.get_mut(&owner.id) {
            Some(playthrough) => Ok(playthrough),
            None if self.all_users.contains(&owner.id) => Err(StartPlaythroughError::NotOwner),
            None => Err(StartPlaythroughError::NotInPlaythrough),
        }?;
        if playthrough.started.is_some() {
            return Err(StartPlaythroughError::AlreadyStarted)
        }

        let now = Utc::now().naive_utc();
        sqlx::query("UPDATE playthroughs SET started = $1 WHERE owner = $2")
            .bind(now)
            .bind(BigDecimal::from(owner.id.get()))
            .execute(pool).await.expect("query works");

        playthrough.started = Some(now);

        Ok(())
    }

    pub async fn join_player(&mut self, owner: &User, player: Player, pool: &PgPool) -> Result<(), JoinPlayerError> {
        let owner_id = owner.id;
        if self.all_users.contains(&player.user_id) { return Err(JoinPlayerError::AlreadyInPlaythrough) }
        if !self.all_users.contains(&owner_id) { return Err(JoinPlayerError::PlayerNotInPlaythrough) };

        let playthrough = self.active_playthroughs.get_mut(&owner_id).ok_or(JoinPlayerError::PlayerNotOwner)?;

        sqlx::query("INSERT INTO playthrough_players(playthrough_owner, user_id, class) VALUES ($1, $2, $3)")
            .bind(BigDecimal::from(owner_id.get()))
            .bind(BigDecimal::from(player.user_id.get()))
            .bind(player.class as i16)
            .execute(pool).await.expect("query is valid");

        self.all_users.insert(player.user_id);
        playthrough.players.push(player);

        Ok(())
    }

    pub async fn kick(&mut self, owner: &User, player: &User, pool: &PgPool) -> Result<(), KickError> {
        if !self.all_users.contains(&owner.id) { return Err(KickError::NotInPlaythrough) }
        if !self.active_playthroughs.contains_key(&owner.id) { return Err(KickError::NotOwner) };

        self.leave(player, pool).await
            .and(Ok(()))
            .map_err(|err| {
                match err {
                    LeaveError::NotInPlaythrough => KickError::PlayerNotInPlaythrough,
                    LeaveError::OwnerOfPlaythrough => KickError::OwnerOfPlaythrough,
                }
            })
    }

    pub async fn leave(&mut self, player: &User, pool: &PgPool) -> Result<&Playthrough, LeaveError> {
        if !self.all_users.contains(&player.id) { return Err(LeaveError::NotInPlaythrough) }
        if self.active_playthroughs.contains_key(&player.id) { return Err(LeaveError::OwnerOfPlaythrough) }

        self.all_users.remove(&player.id);

        let (player_id, playthrough) = self.active_playthroughs.values_mut()
            .find_map(|playthrough| {
                let players = &mut playthrough.players;
                let (i, player_id) = players.iter().enumerate().find_map(|(i, p)| (p.user_id == player.id).then_some((i, p.user_id)))?;
                players.remove(i);
                Some((player_id, playthrough))
            }).expect("player is in a playthrough");

        sqlx::query("DELETE FROM playthrough_players WHERE user_id = $1")
            .bind(BigDecimal::from(player_id.get()))
            .execute(pool).await.expect("query works");

        Ok(playthrough)
    }

    pub async fn progress(&mut self, owner: &User, stage: Option<Stage>, pool: &PgPool) -> Result<&Playthrough, ProgressError> {
        if !self.all_users.contains(&owner.id) { return Err(ProgressError::NotInPlaythrough) }

        let playthrough = self.active_playthroughs.get_mut(&owner.id).ok_or(ProgressError::NotOwner)?;

        let new_stage = stage.or_else(|| {
            let stage_index = playthrough.stage as usize;
            FromPrimitive::from_usize(stage_index + 1)
        }).ok_or(ProgressError::LastStage)?;

        sqlx::query("UPDATE playthroughs SET stage = $1 WHERE owner = $2")
            .bind(new_stage as i16)
            .bind(BigDecimal::from(owner.id.get()))
            .execute(pool).await.expect("query works");

        playthrough.stage = new_stage;

        Ok(playthrough)
    }

    pub async fn load(pool: &PgPool) -> PlaythroughData {
        let playthrough_data = sqlx::query_as("SELECT * FROM playthroughs")
            .fetch_all(pool);

        let players = sqlx::query_as("SELECT * FROM playthrough_players")
            .fetch_all(pool);

        let (playthrough_data, players): (Vec<RawPlaythrough>, Vec<RawPlayer>) = tokio::try_join!(playthrough_data, players).expect("queries work");

        let all_users: HashSet<UserId> = players.iter().map(|player| Into::<Player>::into(player).user_id).collect();

        let players: MultiMap<BigDecimal, Player> = players.into_iter()
            .map(|player| -> (BigDecimal, Player) { (player.1.clone(), player.into()) })
            .collect();

        let mut playthroughs = HashMap::with_capacity(playthrough_data.len());

        for (owner_id, stage, started) in playthrough_data {
            let players = players.get_vec(&owner_id).expect("valid playthrough id").clone();
            let owner_id = owner_id.to_u64().expect("owner snowflake is a valid u64");
            let stage = FromPrimitive::from_i16(stage).expect("stage is a valid stage");
            playthroughs.insert(UserId::new(owner_id), Playthrough { owner: UserId::new(owner_id), players, stage, started });
        }

        PlaythroughData {
            active_playthroughs: playthroughs,
            all_users,
        }
    }
}

pub struct Playthrough {
    pub owner: UserId,
    pub players: Vec<Player>,
    pub stage: Stage,
    pub started: Option<NaiveDateTime>,
}

#[derive(Clone)]
pub struct Player {
    pub user_id: UserId,
    pub class: CalamityClass,
}

impl From<RawPlayer> for Player {
    fn from(value: RawPlayer) -> Self {
        Self::from(&value)
    }
}

impl From<&RawPlayer> for Player {
    fn from(value: &RawPlayer) -> Self {
        Self {
            user_id: UserId::new(value.0.to_u64().expect("user id is a valid u64")),
            class: FromPrimitive::from_i16(value.2).expect("class id is a valid class"),
        }
    }
}

