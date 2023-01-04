// exam crud command

use poise::{serenity_prelude::CacheHttp, Context};
use serenity::model::user::User;

use crate::{database::DbExam, Data, Error};

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

async fn format_exam_list<C: CacheHttp>(
    cache_http: C,
    exam: DbExam,
    user: bool,
    id: bool,
) -> Result<String, Error> {
    let mut message = String::with_capacity(32);

    if user {
        let user_name = exam.user_id.to_user(cache_http).await?.name;
        message.push_str(&format!("{} - ", user_name));
    }

    if !exam.exam_name.is_empty() {
        message.push_str(&format!("{} - {}", exam.day, exam.exam_name));
    } else {
        message.push_str(&format!("{}", exam.day));
    };

    if id {
        message.push_str(&format!(" (ID: {})", exam.exam_id));
    }

    Ok(message)
}

/// List the exams in this guild
#[poise::command(slash_command, guild_only, required_permissions = "ADMINISTRATOR")]
pub async fn guild(ctx: Context<'_, Data, Error>) -> Result<(), Error> {
    let database = &ctx.data().database;
    let guild = ctx.guild().ok_or("Not running in a guild")?;
    let mut exams = database.get_guild_exams(guild.id).await?;
    exams.sort_unstable_by(|a, b| a.day.cmp(&b.day));
    let exams = exams;

    let mut message = String::with_capacity(32 + 32 * exams.len());

    message.push_str(&format!("Exams in {}:\n", guild.name));
    for exam in exams {
        message.push_str(&format!(
            "\t{}\n",
            format_exam_list(&ctx, exam, true, true).await?
        ));
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
    let mut exams = database.get_user_exams(guild.id, user.id).await?;
    exams.sort_unstable_by(|a, b| a.day.cmp(&b.day));
    let exams = exams;

    let user_name = user.name;

    let mut message = String::with_capacity(32 + 32 * exams.len());

    message.push_str(&format!("Exams for {} in {}:\n", user_name, guild.name));
    for exam in exams {
        message.push_str(&format!(
            "\t{}\n",
            format_exam_list(&ctx, exam, false, true).await?
        ));
    }

    ctx.say(message).await?;

    Ok(())
}
