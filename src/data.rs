use core::fmt;
use std::{collections::HashMap, io::BufReader, fs::File, fmt::Display, path::PathBuf};

use convert_case::{Casing, Case};
use poise::ChoiceParameter;
use serde::Deserialize;
use crate::str;

pub type LoadoutData = HashMap<Stage, StageData>;

#[derive(Deserialize)]
pub struct StageData {
    pub potion: PotionType,
    pub powerups: Option<Vec<Powerup>>,
    pub loadouts: HashMap<CalamityClass, Loadout>,
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

#[derive(Deserialize, Debug, Hash, PartialEq, Eq, ChoiceParameter)]
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

#[derive(Deserialize, Hash, PartialEq, Eq, ChoiceParameter)]
pub enum CalamityClass {
    Melee,
    Ranger,
    Mage,
    Summoner,
    Rogue,
}

#[derive(Deserialize, Clone)]
pub struct Loadout {
    pub armor: String,
    pub weapons: [String; 4],
    pub equipment: Vec<String>,
    pub extra: HashMap<String, Vec<String>>,
}

pub fn load_data(mut buf: PathBuf) -> LoadoutData {
    buf.push("loadout_data.json");
    serde_json::from_reader(BufReader::new(File::open(buf).expect("exists"))).expect("valid json")
}

