// exam crud command

use poise::Context;
use serenity::model::user::User;

use crate::{Data, Error};

/// Change the bot's settings for this server
#[poise::command(
    slash_command,
    subcommands("guild", "user"),
    guild_only,
    required_permissions = "ADMINISTRATOR"
)]
pub async fn exams(_ctx: Context<'_, Data, Error>) -> Result<(), Error> {
    Ok(())
}

/// List the exams in this guild
#[poise::command(slash_command, guild_only, required_permissions = "ADMINISTRATOR")]
pub async fn guild(ctx: Context<'_, Data, Error>) -> Result<(), Error> {
    let database = &ctx.data().database;
    let guild = ctx.guild().ok_or("Not running in a guild")?;
    let exams = database.get_guild_exams(guild.id).await?;

    let mut message = String::with_capacity(32 + 32 * exams.len());

    message.push_str(&format!("Exams in {}:\n", guild.name));
    for exam in exams {
        let user_name = exam.user_id.to_user(&ctx).await?.name;
        if !exam.exam_name.is_empty() {
            message.push_str(&format!(
                "\t{} - {} - {}\n",
                user_name, exam.day, exam.exam_name
            ));
        } else {
            message.push_str(&format!("\t{} - {}\n", user_name, exam.day));
        }
    }

    ctx.say(message).await?;

    Ok(())
}

/// List the exams for this user in this guild
#[poise::command(slash_command, guild_only, required_permissions = "ADMINISTRATOR")]
pub async fn user(
    ctx: Context<'_, Data, Error>,
    #[description = "User to look up the exams for"] user: User,
) -> Result<(), Error> {
    let database = &ctx.data().database;
    let guild = ctx.guild().ok_or("Not running in a guild")?;
    let exams = database.get_user_exams(guild.id, user.id).await?;

    let user_name = user.name;

    let mut message = String::with_capacity(32 + 32 * exams.len());

    message.push_str(&format!("Exams for {} in {}:\n", user_name, guild.name));
    for exam in exams {
        if !exam.exam_name.is_empty() {
            message.push_str(&format!("\t{} - {}\n", exam.day, exam.exam_name));
        } else {
            message.push_str(&format!("\t{}\n", exam.day));
        }
    }

    ctx.say(message).await?;

    Ok(())
}
