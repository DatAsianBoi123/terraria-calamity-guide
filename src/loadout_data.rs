use core::fmt::{self, Debug};
use std::{collections::HashMap, fmt::{Display, Formatter}, fs::File, io::BufReader, iter};

use convert_case::{Casing, Case};
use multimap::MultiMap;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use poise::{ChoiceParameter, serenity_prelude::{CreateEmbed, User, Color, Timestamp, CreateEmbedAuthor, CreateEmbedFooter}};
use rocket::futures::future;
use serde::Deserialize;
use sqlx::{PgPool, prelude::FromRow, postgres::{PgHasArrayType, PgTypeInfo}};
use crate::{str, bulleted_array, bulleted};
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

    pub async fn reset(pool: &PgPool) {
        sqlx::query("TRUNCATE stage_data, extra_loadout_data, loadouts RESTART IDENTITY CASCADE")
            .execute(pool).await.expect("valid query");
    }

    pub async fn save(&self, pool: &PgPool) {
        let queries = self.loadouts.iter().enumerate()
            .flat_map(|(stage_i, (stage, stage_data))| {
                let stage_id = *stage as i16;

                let stage_data_query = sqlx::query("INSERT INTO stage_data(stage, health_potion, powerups) VALUES ($1, $2, $3)")
                    .bind(stage_id)
                    .bind(stage_data.potion)
                    .bind(stage_data.powerups.as_deref())
                    .execute(pool);

                let loadout_queries = stage_data.loadouts.iter().enumerate()
                    .flat_map(move |(loadout_i, (class, loadout))| {
                        let id = (stage_i * stage_data.loadouts.len() + loadout_i) as i32;
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

                        iter::once(loadout_query)
                            .chain(extra_queries)
                    });

                iter::once(stage_data_query)
                    .chain(loadout_queries)
            });

        future::join_all(queries).await;
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
            .field("<:armor:1129548766857404576> Armor", &loadout.armor, true)
            .field("<:weapons:1129556916805304410> Weapons", bulleted_array(&loadout.weapons), true)
            .field("<:equipment:1129549501712048178> Equipment", bulleted(&loadout.equipment), true)
            .color(Color::DARK_RED)
            .footer(CreateEmbedFooter::new("Loadouts by GitGudWO").icon_url("https://yt3.googleusercontent.com/lFmtL3AfqsklQGMSPcYf1JUwEZYji5rpq3qPtv1tOGGwvsg4AAT7yffTTN1Co74mbrZ4-M6Lnw=s176-c-k-c0x00ffffff-no-rj"))
            .timestamp(Timestamp::now());

        if !loadout.extra.is_empty() {
            embed = embed
                .field("** **", "** **", false) // force next field to be on next row
                .fields(loadout.extra.iter().map(|(title, list)| (title, bulleted(list), true)));
        }
        embed = embed
            .field("** **", "** **", false)
            .field("<:healing_potion:1129549725331370075> Healing Potion", self.potion.to_string(), true);
        if let Some(powerups) = &self.powerups {
            embed = embed.field("<:powerups:1129550131000254614> Permanent Powerups", bulleted(powerups), true);
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

impl PgHasArrayType for Powerup {
    fn array_type_info() -> sqlx::postgres::PgTypeInfo {
        PgTypeInfo::with_name("_powerup")
    }
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

impl PgHasArrayType for PotionType {
    fn array_type_info() -> PgTypeInfo {
        PgTypeInfo::with_name("_health_potion")
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

impl Default for Stage {
    fn default() -> Self {
        Self::PreBoss
    }
}

impl Stage {
    pub fn img(&self) -> String {
        use Stage::*;

        let s = match self {
            PreBoss => "https://terraria.wiki.gg/images/a/a1/Map_Icon_Eye_of_Cthulhu_%28first_form%29.png",
            PreEvil => "https://i.imgur.com/YozyRaq.png",
            PreSkeletron => "https://terraria.wiki.gg/images/f/f4/Map_Icon_Skeletron.png",
            PreMech => "https://terraria.wiki.gg/images/6/6f/Map_Icon_The_Destroyer.png",
            PreWall => "https://terraria.wiki.gg/images/d/d4/Map_Icon_Wall_of_Flesh.png",
            PrePlantera => "https://i.imgur.com/JPIVa0l.png",
            PreGolem => "https://terraria.wiki.gg/images/b/b7/Map_Icon_Golem.png",
            PreCultist => "https://terraria.wiki.gg/images/6/68/Map_Icon_Lunatic_Cultist.png",
            PreMoonLord => "https://terraria.wiki.gg/images/8/82/Map_Icon_Moon_Lord.png",
            PreProvidence => "https://calamitymod.wiki.gg/images/f/fb/Providence_map.png",
            PrePolterghast => "https://calamitymod.wiki.gg/images/f/fc/Necroplasm_map.png",
            PreDevourer => "https://calamitymod.wiki.gg/images/f/fb/Devourer_of_Gods_map.png",
            PreYharon => "https://calamitymod.wiki.gg/images/7/70/Yharon_map.png",
            PreDraedon => "https://i.imgur.com/KirWaB3.png",
            Endgame => "https://calamitymod.wiki.gg/images/c/cb/Terminus.png",
        };
        s.to_string()
    }
}

#[derive(Clone, Copy, Deserialize, Hash, PartialEq, Eq, ChoiceParameter, FromPrimitive)]
pub enum CalamityClass {
    Melee,
    Ranger,
    Mage,
    Summoner,
    Rogue,
}

impl CalamityClass {
    pub fn emoji(&self) -> String {
        match self {
            Self::Melee => str!("<:melee:1152482097911574571>"),
            Self::Ranger => str!("<:ranger:1152484385359142995>"),
            Self::Mage => str!("<:mage:1152485318021361716>"),
            Self::Summoner => str!("<:summoner:1152485908105396277>"),
            Self::Rogue => str!("<:rogue:1152486391503126568>"),
        }
    }
}

#[derive(Deserialize, Clone)]
pub struct Loadout {
    pub armor: String,
    pub weapons: [String; 4],
    pub equipment: Vec<String>,
    pub extra: LinkedHashMap<String, Vec<String>>,
}

