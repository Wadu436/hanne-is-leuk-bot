// TODO: command to add new exam for a user
// TODO: command to list a user's exams
// TODO: command to delete an exam for a user
// TODO: add a name for each exam? could serve as pkey per user (also add to db)

use chrono::NaiveDate;
use poise::Context;

use crate::{database::DbExam, Data, Error};

/// Add a new exam
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn exam(
    ctx: Context<'_, Data, Error>,
    #[description = "Which user the exam is taken by"] user: serenity::model::user::User,
    #[description = "What day the exam is. (format: \"YYYY-MM-DD\")"] day: String,
    #[description = "The name of the exam."] name: Option<String>,
) -> Result<(), Error> {
    let guild_id = ctx.guild_id().ok_or("Not running in a guild")?;
    let database = &ctx.data().database;
    let scheduler = &ctx.data().scheduler;
    let name = name.unwrap_or("".to_string()).trim().to_string();

    let day = if let Ok(day) = NaiveDate::parse_from_str(&day, "%Y-%m-%d") {
        day
    } else {
        ctx.say(format!("Invalid date: {}", day)).await?;
        return Ok(());
    };
    let exam = DbExam {
        exam_id: 0,
        day,
        exam_name: name.clone(),
        guild_id,
        user_id: user.id,
    };

    // Insert into db and add to scheduler
    let exam_id = database.insert_exam(exam.clone()).await?;
    scheduler.add_exam(exam_id).await?;

    if name.is_empty() {
        ctx.say(format!(
            "Added new exam for user {} on {}: \"{}\"",
            user.nick_in(&ctx, guild_id)
                .await
                .unwrap_or_else(|| user.name),
            day,
            name
        ))
        .await?;
    } else {
        ctx.say(format!(
            "Added new exam for user {} on {}",
            user.nick_in(&ctx, guild_id)
                .await
                .unwrap_or_else(|| user.name),
            day
        ))
        .await?;
    }

    Ok(())
}
