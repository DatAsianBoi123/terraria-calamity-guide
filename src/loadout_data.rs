use core::fmt::{self, Debug};
use std::{collections::HashMap, fmt::{Display, Formatter}, fs::File, io::BufReader, iter};

use convert_case::{Casing, Case};
use futures::future::join_all;
use multimap::MultiMap;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use poise::{ChoiceParameter, serenity_prelude::{CreateEmbed, User, Color, Timestamp, CreateEmbedAuthor, CreateEmbedFooter}};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, prelude::FromRow};
use thiserror::Error;
use crate::{bulleted, str};
use linked_hash_map::LinkedHashMap;

#[derive(FromRow)]
struct RawLoadout {
    id: i32,
    class: i16,
    stage: i16,
    armor: String,
    weapons: [String; 4],
    equipment: Vec<String>,
}

#[derive(FromRow)]
struct RawStageData {
    stage: i16,
    health_potion: PotionType,
    powerups: Option<Vec<Powerup>>,
}

#[derive(FromRow)]
struct RawExtraLoadoutData {
    loadout_id: i32,
    label: String,
    data: Vec<String>,
}

pub struct LoadoutData {
    loadouts: HashMap<Stage, StageData>,
}

impl LoadoutData {
    pub fn get_loadout(&self, stage: Stage, class: CalamityClass) -> Option<&Loadout> {
        self.loadouts.get(&stage)?.loadouts.get(&class)
    }

    pub fn get_stage(&self, stage: Stage) -> Option<&StageData> {
        self.loadouts.get(&stage)
    }

    pub async fn edit(&mut self, pool: &PgPool, stage: Stage, class: CalamityClass, header: LoadoutHeader) -> Result<(), EditLoadoutError> {
        let loadout = self.loadouts.get_mut(&stage)
            .and_then(|stage_data| stage_data.loadouts.get_mut(&class))
            .ok_or(LoadoutNotFoundError { stage, class })?;
        let id = loadout.id.expect("loadout has an id");
        let query = match header {
            LoadoutHeader::Armor(armor) => {
                loadout.armor = armor;

                sqlx::query("UPDATE loadouts SET armor = $1 WHERE id = $2")
                    .bind(&loadout.armor)
                    .bind(id)
            },
            LoadoutHeader::Weapons(weapons) => {
                loadout.weapons = weapons;

                sqlx::query("UPDATE loadouts SET weapons = $1 WHERE id = $2")
                    .bind(&loadout.weapons)
                    .bind(id)
            },
            LoadoutHeader::Equipment(equipment) => {
                loadout.equipment = equipment;

                sqlx::query("UPDATE loadouts SET equipment = $1 WHERE id = $2")
                    .bind(&loadout.equipment)
                    .bind(id)
            },
        };

        query.execute(pool).await.expect("valid query");
        Ok(())
    }

    pub async fn set_extra(
        &mut self,
        pool: &PgPool,
        stage: Stage,
        class: CalamityClass,
        label: String,
        values: Vec<String>,
    ) -> Result<(), SetExtraError> {
        let loadout = self.loadouts.get_mut(&stage)
            .and_then(|stage_data| stage_data.loadouts.get_mut(&class))
            .ok_or(LoadoutNotFoundError { stage, class })?;
        if !loadout.extra.contains_key(&label) { return Err(SetExtraError::LabelNotFound(label)); }

        loadout.extra.entry(label.clone()).and_modify(|old_data| *old_data = values.clone());

        sqlx::query("UPDATE extra_loadout_data SET data = $1 WHERE label = $2 AND loadout_id = $3")
            .bind(values)
            .bind(label)
            .bind(loadout.id.expect("loadout has id"))
            .execute(pool).await.expect("query is valid");

        Ok(())
    }

    pub async fn add_extra(
        &mut self,
        pool: &PgPool,
        stage: Stage,
        class: CalamityClass,
        label: String,
        values: Vec<String>,
    ) -> Result<(), AddExtraError> {
        let loadout = self.loadouts.get_mut(&stage)
            .and_then(|stage_data| stage_data.loadouts.get_mut(&class))
            .ok_or(LoadoutNotFoundError { stage, class })?;
        if loadout.extra.contains_key(&label) { return Err(AddExtraError::LabelAlreadyExists(label)); }

        loadout.extra.insert(label.clone(), values.clone());

        sqlx::query("INSERT INTO extra_loadout_data(loadout_id, label, data) VALUES ($1, $2, $3)")
            .bind(loadout.id.expect("loadout has an id"))
            .bind(label)
            .bind(values)
            .execute(pool).await.expect("query is valid");

        Ok(())
    }

    pub async fn reset(pool: &PgPool) {
        sqlx::query("TRUNCATE stage_data, extra_loadout_data, loadouts RESTART IDENTITY CASCADE")
            .execute(pool).await.expect("valid query");
    }

    pub async fn save(&self, pool: &PgPool) {
        // HACK: literally cannot find a better way to do this
        let queries = self.loadouts.iter().enumerate()
            .map(|(stage_i, (stage, stage_data))| {
                let stage_id = *stage as i16;

                let stage_data_query = sqlx::query("INSERT INTO stage_data(stage, health_potion, powerups) VALUES ($1, $2, $3)")
                    .bind(stage_id)
                    .bind(stage_data.potion)
                    .bind(stage_data.powerups.as_deref())
                    .execute(pool);

                let loadout_queries = stage_data.loadouts.iter().enumerate()
                    .map(move |(loadout_i, (class, loadout))| {
                        let id = loadout.id.unwrap_or((stage_i * stage_data.loadouts.len() + loadout_i) as i32);
                        let loadout_query = sqlx::query(
                            "INSERT INTO loadouts(id, class, stage, armor, weapons, equipment) VALUES ($1, $2, $3, $4, $5, $6)"
                        )
                            .bind(id)
                            .bind(*class as i16)
                            .bind(*stage as i16)
                            .bind(&loadout.armor)
                            .bind(&loadout.weapons)
                            .bind(&loadout.equipment)
                            .execute(pool);

                        let extra_queries = loadout.extra.iter()
                            .map(move |(label, data)| {
                                sqlx::query("INSERT INTO extra_loadout_data(loadout_id, label, data) VALUES ($1, $2, $3)")
                                    .bind(id)
                                    .bind(label)
                                    .bind(data)
                                    .execute(pool)
                            });

                        (iter::once(loadout_query), extra_queries)
                    });

                let queries = iter::once(stage_data_query)
                    .chain(loadout_queries.clone().flat_map(|(query, _)| query));
                (queries, loadout_queries.flat_map(|(_, extra)| extra))
            });

        join_all(queries.clone().flat_map(|(query, _)| query)).await;
        join_all(queries.flat_map(|(_, extra)| extra)).await;
    }

    pub fn from_file(loadouts: File) -> Option<LoadoutData> {
        serde_json::from_reader(BufReader::new(loadouts)).ok().map(|loadouts| LoadoutData { loadouts })
    }

    pub async fn load(pool: &PgPool) -> LoadoutData {
        let stage_data = sqlx::query_as("SELECT * FROM stage_data")
            .fetch_all(pool);

        let loadouts = sqlx::query_as("SELECT * FROM loadouts")
            .fetch_all(pool);

        let extra_loadout_data = sqlx::query_as("SELECT * FROM extra_loadout_data ORDER BY id")
            .fetch_all(pool);

        let (stage_data, loadouts, extra_loadout_data): (Vec<RawStageData>, Vec<RawLoadout>, Vec<RawExtraLoadoutData>) = tokio::try_join!(
            stage_data,
            loadouts,
            extra_loadout_data,
        ).expect("loadouts work");

        let extra_loadout_data: MultiMap<i32, RawExtraLoadoutData> = extra_loadout_data.into_iter()
            .map(|raw| (raw.loadout_id, raw))
            .collect();
        let mut extra_loadout_data: HashMap<i32, LinkedHashMap<String, Vec<String>>> = extra_loadout_data.into_iter()
            .map(|(stage, data)| (stage, data.into_iter().fold(LinkedHashMap::new(), |mut acc, raw| {
                acc.insert(raw.label, raw.data);
                acc
            })))
            .collect();

        let stage_data: HashMap<Stage, RawStageData> = stage_data.into_iter()
            .map(|raw| (FromPrimitive::from_i16(raw.stage).expect("stage num is valid stage"), raw))
            .collect();

        let loadouts: MultiMap<Stage, (CalamityClass, Loadout)> = loadouts.into_iter().map(|raw| {
            let stage = FromPrimitive::from_i16(raw.stage).expect("stage num is valid stage");
            let class = FromPrimitive::from_i16(raw.class).expect("class num is valid class");
            let extra = extra_loadout_data.remove(&raw.id).unwrap_or_default();
            let loadout = Loadout {
                id: Some(raw.id),
                armor: raw.armor,
                weapons: raw.weapons,
                equipment: raw.equipment,
                extra,
            };
            (stage, (class, loadout))
        }).collect();

        let loadout_data: HashMap<Stage, StageData> = loadouts.into_iter()
            .filter_map(|(stage, loadouts)| Some((stage, (stage_data.get(&stage)?, loadouts))))
            .map(|(stage, (stage_data, loadouts))| {
                (stage, StageData { potion: stage_data.health_potion, powerups: stage_data.powerups.clone(), loadouts: loadouts.into_iter().collect() })
            })
            .collect();

        LoadoutData { loadouts: loadout_data }
    }
}

#[derive(Deserialize)]
pub struct StageData {
    pub potion: PotionType,
    pub powerups: Option<Vec<Powerup>>,
    pub loadouts: HashMap<CalamityClass, Loadout>,
}

impl StageData {
    pub fn create_embed(&self, author: &User, class: CalamityClass, stage: Stage) -> CreateEmbed {
        let loadout = self.loadouts.get(&class).expect("loadout exists for stage");
        let mut embed = CreateEmbed::new();
        embed = embed
            .title(format!("{} - {}", class.name(), stage.name()))
            .author(CreateEmbedAuthor::new(&author.name).icon_url(author.avatar_url().unwrap_or_default()))
            .thumbnail(stage.img())
            .field("<:armor:1312528988786393088> Armor", &loadout.armor, true)
            .field("<:weapons:1312528868074328074> Weapons", bulleted(&loadout.weapons), true)
            .field("<:equipment:1312528964866150471> Equipment", bulleted(&loadout.equipment), true)
            .color(Color::DARK_RED)
            .footer(CreateEmbedFooter::new("Loadouts by GitGudWO").icon_url(crate::get_asset("gitgudpfp.jpg")))
            .timestamp(Timestamp::now());

        if !loadout.extra.is_empty() {
            embed = embed
                .field("** **", "** **", false) // force next field to be on next row
                .fields(loadout.extra.iter().map(|(title, list)| (title, bulleted(list), true)));
        }
        embed = embed
            .field("** **", "** **", false)
            .field("<:healing_potion:1312528931836002314> Healing Potion", self.potion.to_string(), true);
        if let Some(powerups) = &self.powerups {
            embed = embed.field("<:powerups:1312528902308102254> Permanent Powerups", bulleted(powerups), true);
        }
        embed
    }

}

#[derive(Clone, Copy, Deserialize, Debug, sqlx::Type)]
#[sqlx(type_name = "powerup")]
pub enum Powerup {
    LifeCrystal,
    LifeFruit,
    BloodOrange,
    MiracleFruit,
    Elderberry,
    Dragonfruit,

    ManaCrystal,
    CometShard,
    EtherealCore,
    PhantomHeart,

    MushroomPlasmaRoot,
    InfernalBlood,
    RedLightningContainer,

    ElectrolyteGelPack,
    StarlightFuelCell,
    Ectoheart,

    HermitBox,
    DemonHeart,
    CelestialOnion,
}

impl Display for Powerup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let str = match self {
            Self::LifeCrystal => str!("Life Crystal (15)"),
            Self::ManaCrystal => str!("Mana Crystal (9)"),
            Self::LifeFruit => str!("Life Fruit (20)"),
            Self::HermitBox => str!("Hermit's Box of One Hundred Medicines"),
            _ => format!("{self:?}").from_case(Case::Pascal).to_case(Case::Title),
        };
        write!(f, "{str}")
    }
}

#[derive(Clone, Copy, Deserialize, Debug, sqlx::Type)]
#[sqlx(type_name = "health_potion")]
pub enum PotionType {
    Lesser,
    Normal,
    Greater,
    Super,
    Supreme,
    Omega,
}

impl Display for PotionType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self, f)
    }
}

#[derive(Copy, Clone, Deserialize, Debug, Hash, PartialEq, Eq, ChoiceParameter, FromPrimitive)]
pub enum Stage {
    #[name = "Pre-Bosses"]
    PreBoss,
    #[name = "Pre-Hive Mind / Perforator"]
    PreEvil,
    #[name = "Pre-Skeletron"]
    PreSkeletron,
    #[name = "Pre-Wall of Flesh"]
    PreWall,
    #[name = "Pre-Mechanical Bosses"]
    PreMech,
    #[name = "Pre-Plantera / Calamitas"]
    PrePlantera,
    #[name = "Pre-Golem"]
    PreGolem,
    #[name = "Pre-Lunatic Cultist"]
    PreCultist,
    #[name = "Pre-Moon Lord"]
    PreMoonLord,
    #[name = "Pre-Providence"]
    PreProvidence,
    #[name = "Pre-Polterghast"]
    PrePolterghast,
    #[name = "Pre-Devourer of Gods"]
    PreDevourer,
    #[name = "Pre-Yharon"]
    PreYharon,
    #[name = "Pre-Draedon / Supreme Calamitas"]
    PreDraedon,
    Endgame,
}

impl Display for Stage {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

impl Default for Stage {
    fn default() -> Self {
        Self::PreBoss
    }
}

impl Stage {
    pub fn img(&self) -> Url {
        use Stage::*;

        match self {
            PreBoss => crate::get_asset("preboss.png"),
            PreEvil => crate::get_asset("preevil.png"),
            PreSkeletron => crate::get_asset("preskeletron.png"),
            PreMech => crate::get_asset("premech.png"),
            PreWall => crate::get_asset("prewall.png"),
            PrePlantera =>  crate::get_asset("preplantera.png"),
            PreGolem => crate::get_asset("pregolem.png"),
            PreCultist => crate::get_asset("precultist.png"),
            PreMoonLord => crate::get_asset("premoonlord.png"),
            PreProvidence => crate::get_asset("preprovidence.png"),
            PrePolterghast => crate::get_asset("prepolterghast.png"),
            PreDevourer => crate::get_asset("predevourer.png"),
            PreYharon => crate::get_asset("preyharon.png"),
            PreDraedon => crate::get_asset("predraedon.png"),
            Endgame => crate::get_asset("endgame.png"),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Hash, PartialEq, Eq, ChoiceParameter, FromPrimitive)]
pub enum CalamityClass {
    Melee,
    Ranger,
    Mage,
    Summoner,
    Rogue,
}

impl Display for CalamityClass {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl CalamityClass {
    pub fn emoji(&self) -> String {
        match self {
            Self::Melee => str!("<:melee:1312528694367092780>"),
            Self::Ranger => str!("<:ranger:1312528658895736893>"),
            Self::Mage => str!("<:mage:1312528590734098542>"),
            Self::Summoner => str!("<:summoner:1312527694172393563>"),
            Self::Rogue => str!("<:rogue:1312527650945896579>"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Loadout {
    #[serde(skip_serializing)]
    pub id: Option<i32>,
    pub armor: String,
    pub weapons: [String; 4],
    pub equipment: Vec<String>,
    pub extra: LinkedHashMap<String, Vec<String>>,
}

pub enum LoadoutHeader {
    Armor(String),
    Weapons([String; 4]),
    Equipment(Vec<String>),
}

#[derive(Error, Debug)]
pub enum EditLoadoutError {
    #[error(transparent)]
    LoadoutNotFound(#[from] LoadoutNotFoundError),
}

#[derive(Error, Debug)]
pub enum SetExtraError {
    #[error(transparent)]
    LoadoutNotFound(#[from] LoadoutNotFoundError),
    #[error("Label '{0}' was not found")]
    LabelNotFound(String),
}

#[derive(Error, Debug)]
pub enum AddExtraError {
    #[error(transparent)]
    LoadoutNotFound(#[from] LoadoutNotFoundError),
    #[error("label '{0}' already exists")]
    LabelAlreadyExists(String),
}

#[derive(Error, Debug)]
#[error("Loadout not found with stage {stage} and class {class}")]
pub struct LoadoutNotFoundError {
    pub stage: Stage,
    pub class: CalamityClass,
}

