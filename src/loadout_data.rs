use core::fmt;
use std::{collections::HashMap, io::BufReader, fs::File, fmt::Display};

use convert_case::{Casing, Case};
use num_derive::FromPrimitive;
use poise::{ChoiceParameter, serenity_prelude::{CreateEmbed, User, Color, Timestamp, CreateEmbedAuthor, CreateEmbedFooter}};
use serde::Deserialize;
use crate::{str, bulleted_array, bulleted};
use linked_hash_map::LinkedHashMap;

pub type LoadoutData = HashMap<Stage, StageData>;

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

#[derive(Deserialize, Debug)]
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

#[derive(Deserialize, Debug)]
pub enum PotionType {
    Lesser,
    Normal,
    Greater,
    Super,
    Supreme,
    Omega,
}

impl Display for PotionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt::Debug::fmt(&self, f)
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

pub fn load_data(loadouts: File) -> LoadoutData {
    serde_json::from_reader(BufReader::new(loadouts)).expect("valid json")
}

