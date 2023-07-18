use std::sync::Arc;

use poise::{command, serenity_prelude::{Color, Timestamp}};

use crate::{Context, Result, data::{CalamityClass, Stage}, str};

#[command(
    slash_command,
    description_localized("en-US", "Views the recommended loadout during a specific stage of progression"),
    rename = "viewloadout"
)]
pub async fn view_loadout(
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

