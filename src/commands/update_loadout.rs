use poise::command;

use crate::{Context, PoiseResult, loadout_data::{Stage, CalamityClass, LoadoutHeader}, Loadouts};

#[command(
    slash_command,
    rename = "editloadout",
    subcommands("armor", "weapons", "equipment"),
    owners_only,
    default_member_permissions = "MANAGE_GUILD",
)]
pub async fn edit_loadout(_: Context<'_>) -> PoiseResult {
    Ok(())
}

#[command(slash_command)]
pub async fn armor(ctx: Context<'_>, stage: Stage, class: CalamityClass, armor: String) -> PoiseResult {
    ctx.defer_ephemeral().await?;

    edit(ctx, stage, class, LoadoutHeader::Armor(armor)).await;
    Ok(())
}

#[command(slash_command)]
pub async fn weapons(ctx: Context<'_>, stage: Stage, class: CalamityClass, weapons: Vec<String>) -> PoiseResult {
    ctx.defer_ephemeral().await?;

    if let Ok(weapons) = weapons.try_into() {
        edit(ctx, stage, class, LoadoutHeader::Weapons(weapons)).await;
    } else {
        ctx.say("Must only contain 4 weapons").await?;
    }
    Ok(())
}

#[command(slash_command)]
pub async fn equipment(ctx: Context<'_>, stage: Stage, class: CalamityClass, equipment: Vec<String>) -> PoiseResult {
    ctx.defer_ephemeral().await?;

    edit(ctx, stage, class, LoadoutHeader::Equipment(equipment)).await;
    Ok(())
}

async fn edit(ctx: Context<'_>, stage: Stage, class: CalamityClass, header: LoadoutHeader) -> Option<()> {
    let mut lock = ctx.serenity_context().data.write().await;
    let mut loadout_data = lock.get_mut::<Loadouts>().expect("loadouts exist").write().await;
    loadout_data.edit(&ctx.data().pool, stage, class, header).await;
    ctx.say("Successfully updated loadout").await.ok().and(Some(()))
}

