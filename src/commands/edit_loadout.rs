use poise::command;

use crate::{Context, PoiseResult, loadout_data::{Stage, CalamityClass, LoadoutHeader, EditLoadoutError}, Loadouts, str};

#[command(
    slash_command,
    rename = "editloadout",
    subcommands("armor", "weapons", "equipment", "replace_extra", "add_extra"),
    owners_only,
    default_member_permissions = "MANAGE_GUILD",
)]
pub async fn edit_loadout(_: Context<'_>) -> PoiseResult {
    Ok(())
}

#[command(slash_command)]
pub async fn armor(ctx: Context<'_>, stage: Stage, class: CalamityClass, armor: String) -> PoiseResult {
    ctx.defer_ephemeral().await?;

    let message = edit(ctx, stage, class, LoadoutHeader::Armor(armor)).await
        .map_or_else(|err| str!(err), |_| str!("Successfully edited armor"));
    ctx.say(message).await?;

    Ok(())
}

#[command(slash_command)]
pub async fn weapons(ctx: Context<'_>, stage: Stage, class: CalamityClass, weapons: String) -> PoiseResult {
    ctx.defer_ephemeral().await?;

    let message = match weapons.split(',').map(|str| str.to_owned()).collect::<Vec<_>>().try_into() {
        Ok(weapons) => {
            edit(ctx, stage, class, LoadoutHeader::Weapons(weapons)).await.map_or_else(|err| str!(err), |_| str!("Successfully edited weapons"))
        },
        Err(_) => str!("Weapons must contain 4 elements"),
    };
    ctx.say(message).await?;

    Ok(())
}

#[command(slash_command)]
pub async fn equipment(ctx: Context<'_>, stage: Stage, class: CalamityClass, equipment: String) -> PoiseResult {
    ctx.defer_ephemeral().await?;

    let message = edit(ctx, stage, class, LoadoutHeader::Equipment(equipment.split(',').map(|str| str.to_owned()).collect())).await
        .map_or_else(|err| str!(err), |_| str!("Successfully edited equipment"));
    ctx.say(message).await?;

    Ok(())
}

#[command(slash_command, rename = "replaceextra")]
pub async fn replace_extra(ctx: Context<'_>, stage: Stage, class: CalamityClass, label: String, values: String) -> PoiseResult {
    ctx.defer_ephemeral().await?;

    let mut lock = ctx.serenity_context().data.write().await;
    let mut loadout_data = lock.get_mut::<Loadouts>().expect("loadouts exist").write().await;

    let message = loadout_data.set_extra(&ctx.data().pool, stage, class, label, values.split(',').map(|str| str.to_owned()).collect()).await
        .map_or_else(|err| str!(err), |_| str!("Successfully replaced extra label"));
    ctx.say(message).await?;

    Ok(())
}

#[command(slash_command, rename = "addextra")]
pub async fn add_extra(ctx: Context<'_>, stage: Stage, class: CalamityClass, label: String, values: String) -> PoiseResult {
    ctx.defer_ephemeral().await?;

    let mut lock = ctx.serenity_context().data.write().await;
    let mut loadout_data = lock.get_mut::<Loadouts>().expect("loadouts exist").write().await;

    let message = loadout_data.set_extra(&ctx.data().pool, stage, class, label, values.split(',').map(|str| str.to_owned()).collect()).await
        .map_or_else(|err| str!(err), |_| str!("Successfully replaced extra label"));
    ctx.say(message).await?;

    Ok(())
}

async fn edit(ctx: Context<'_>, stage: Stage, class: CalamityClass, header: LoadoutHeader) -> Result<(), EditLoadoutError> {
    let mut lock = ctx.serenity_context().data.write().await;
    let mut loadout_data = lock.get_mut::<Loadouts>().expect("loadouts exist").write().await;
    loadout_data.edit(&ctx.data().pool, stage, class, header).await
}

