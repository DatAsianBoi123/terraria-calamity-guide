use std::{collections::{HashMap, HashSet}, vec::Vec, convert::Into};

use num_traits::{ToPrimitive, FromPrimitive};
use poise::serenity_prelude::{UserId, User};
use sqlx::{PgPool, types::{BigDecimal, chrono::{NaiveDateTime, Utc}}, postgres::{PgTypeInfo, PgHasArrayType}};
use tracing::info;

use crate::loadout_data::{CalamityClass, Stage};

type RawPlaythrough = (BigDecimal, Vec<RawPlayer>, i16, Option<NaiveDateTime>);

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

        let playthrough = Playthrough {
            owner: owner.id,
            players: vec![Player { id: owner.id, class }],
            stage: Stage::PreBoss,
            started: None,
        };
        let playthrough_data: RawPlaythrough = playthrough.raw_data();

        sqlx::query("INSERT INTO playthroughs(owner, players, stage) VALUES ($1, ARRAY[$2], $3)")
            .bind(playthrough_data.0)
            .bind(&playthrough_data.1[0])
            .bind(playthrough_data.2)
            .execute(pool).await.expect("valid query");

        let owner_id = playthrough.owner;
        self.active_playthroughs.insert(owner_id, playthrough);
        self.all_users.insert(owner_id);

        info!("{} ({}) created a playthrough", owner.name, owner.id);

        Ok(self.active_playthroughs.get(&owner.id).expect("playthrough exists"))
    }

    pub async fn end(&mut self, owner: &User, pool: &PgPool) -> Result<(), FinishPlaythroughError> {
        if !self.active_playthroughs.contains_key(&owner.id) {
            return if self.all_users.contains(&owner.id) {
                Err(FinishPlaythroughError::NotOwner)
            } else {
                Err(FinishPlaythroughError::NotInPlaythrough)
            }
        }

        sqlx::query("DELETE FROM playthroughs WHERE owner = $1")
            .bind(BigDecimal::from_u64(owner.id.0).expect("big decimal"))
            .execute(pool).await.expect("query works");

        // make sure id increments correctly
        sqlx::query("SELECT setval(pg_get_serial_sequence('playthroughs', 'id'), COALESCE(max(id) + 1, 1), false) FROM playthroughs")
            .execute(pool).await.expect("query works");

        let playthrough = self.active_playthroughs.remove(&owner.id).expect("owner is in playthrough");
        for player in playthrough.players {
            self.all_users.remove(&player.id);
        }

        Ok(())
    }

    pub async fn start(&mut self, owner: &User, pool: &PgPool) -> Result<(), StartPlaythroughError> {
        let playthrough = match self.active_playthroughs.get_mut(&owner.id) {
            Some(playthrough) => Ok(playthrough),
            None => {
                if self.all_users.contains(&owner.id) {
                    Err(StartPlaythroughError::NotOwner)
                } else {
                    Err(StartPlaythroughError::NotInPlaythrough)
                }
            },
        }?;
        if playthrough.started.is_some() {
            return Err(StartPlaythroughError::AlreadyStarted)
        }

        let now = Utc::now().naive_utc();
        sqlx::query("UPDATE playthroughs SET started = $1 WHERE owner = $2")
            .bind(now)
            .bind(BigDecimal::from_u64(owner.id.0).expect("owner id is a valid big decimal"))
            .execute(pool).await.expect("query works");

        playthrough.started = Some(now);

        Ok(())
    }

    pub async fn join_player(&mut self, owner: &User, player: Player, pool: &PgPool) -> Result<(), JoinPlayerError> {
        if !self.active_playthroughs.contains_key(&owner.id) {
            return if self.all_users.contains(&owner.id) {
                Err(JoinPlayerError::PlayerNotOwner)
            } else {
                Err(JoinPlayerError::PlayerNotInPlaythrough)
            }
        }
        if self.all_users.contains(&player.id) { return Err(JoinPlayerError::AlreadyInPlaythrough) }

        sqlx::query("UPDATE playthroughs SET players = players || $1 WHERE owner = $2")
            .bind(player.sql_type())
            .bind(BigDecimal::from_u64(owner.id.0).expect("id is a big decimal"))
            .execute(pool).await.expect("query is valid");

        self.all_users.insert(player.id);
        self.active_playthroughs.entry(owner.id).and_modify(|playthrough| playthrough.players.push(player));

        Ok(())
    }

    pub async fn kick(&mut self, owner: &User, player: &User, pool: &PgPool) -> Result<(), KickError> {
        if !self.all_users.contains(&owner.id) { return Err(KickError::NotInPlaythrough) }

        match self.active_playthroughs.get_mut(&owner.id) {
            Some(_) => {
                self.leave(player, pool).await
                    .and(Ok(()))
                    .map_err(|err| {
                        match err {
                            LeaveError::NotInPlaythrough => KickError::PlayerNotInPlaythrough,
                            LeaveError::OwnerOfPlaythrough => KickError::OwnerOfPlaythrough,
                        }
                })
            },
            None => Err(KickError::NotOwner),
        }
    }

    pub async fn leave(&mut self, player: &User, pool: &PgPool) -> Result<&Playthrough, LeaveError> {
        if !self.all_users.contains(&player.id) { return Err(LeaveError::NotInPlaythrough) }
        if self.active_playthroughs.contains_key(&player.id) { return Err(LeaveError::OwnerOfPlaythrough) }

        self.all_users.remove(&player.id);
        let mut playthroughs = None;
        self.active_playthroughs.iter_mut()
            .try_for_each(|(_, p)| {
                let old_len = p.players.len();
                p.players.retain(|p| p.id != player.id);
                if p.players.len() != old_len {
                    // found player to delete
                    playthroughs = Some(p);
                    Ok(())
                } else { Err(()) }
            }).expect("player is in a playthrough");
        let playthroughs = playthroughs.expect("playthrough was found");
        let players = &playthroughs.players;

        let player_params = players.iter().enumerate().map(|(i, _)| format!("${}", i + 1)).collect::<Vec<String>>().join(" ");
        let query_str = format!("UPDATE playthroughs SET players = ARRAY[{}]", player_params);
        let mut query = sqlx::query(&query_str);

        for player in players {
            query = query.bind(player.sql_type());
        }

        query.execute(pool).await.expect("query is valid");

        Ok(playthroughs)
    }

    pub async fn progress(&mut self, owner: &User, stage: Option<Stage>, pool: &PgPool) -> Result<&Playthrough, ProgressError> {
        if !self.all_users.contains(&owner.id) { return Err(ProgressError::NotInPlaythrough) }

        match self.active_playthroughs.get_mut(&owner.id) {
            Some(playthrough) => {
                let new_stage = stage.or_else(|| {
                    let stage_index = playthrough.stage as usize;
                    FromPrimitive::from_usize(stage_index + 1)
                }).ok_or(ProgressError::LastStage)?;

                sqlx::query("UPDATE playthroughs SET stage = $1 WHERE owner = $2")
                    .bind(new_stage as i16)
                    .bind(BigDecimal::from_u64(owner.id.0).expect("owner id is big decimal"))
                    .execute(pool).await.expect("query works");

                playthrough.stage = new_stage;

                Ok(playthrough)
            },
            None => Err(ProgressError::NotOwner),
        }
    }

    pub async fn load(pool: &PgPool) -> PlaythroughData {
        let playthrough_data: Vec<RawPlaythrough> = sqlx::query_as("SELECT owner, players, stage, started FROM playthroughs")
            .fetch_all(pool).await.expect("can select playthroughs");

        let mut playthroughs = HashMap::with_capacity(playthrough_data.len());
        let mut all_users = HashSet::new();

        for (owner_id, raw_players, stage, started) in playthrough_data {
            let mut players = Vec::with_capacity(raw_players.len());
            for player in raw_players {
                let player_id = player.id.to_u64().expect("player id is valid u64");
                players.push(Player {
                    id: UserId(player_id),
                    class: FromPrimitive::from_i16(player.class).expect("player class is a valid class"),
                });
                all_users.insert(UserId(player_id));
            }
            let owner_id = owner_id.to_u64().expect("owner snowflake is a valid u64");
            let stage = FromPrimitive::from_i16(stage).expect("stage is a valid stage");
            playthroughs.insert(UserId(owner_id), Playthrough { owner: UserId(owner_id), players, stage, started });
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

impl Playthrough {
    fn raw_data(&self) -> RawPlaythrough {
        let owner = BigDecimal::from_u64(self.owner.0).expect("id is a valid big decimal");
        (owner, self.players.iter().map(Player::sql_type).collect(), self.stage as i16, self.started)
    }
}

pub struct Player {
    pub id: UserId,
    pub class: CalamityClass,
}

impl Player {
    pub fn raw_data(&self) -> (BigDecimal, i16) {
        (BigDecimal::from_u64(self.id.0).expect("id is a valid big decimal"), self.class as i16)
    }

    fn sql_type(&self) -> RawPlayer {
        self.raw_data().into()
    }
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "player")]
struct RawPlayer {
    id: BigDecimal,
    class: i16,
}

impl PgHasArrayType for RawPlayer {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("_player")
    }
}

impl From<(BigDecimal, i16)> for RawPlayer {
    fn from(value: (BigDecimal, i16)) -> Self {
        RawPlayer { id: value.0, class: value.1 }
    }
}

