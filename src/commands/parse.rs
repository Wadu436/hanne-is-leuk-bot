use std::fmt::Display;

use poise::{
    serenity_prelude::{GuildId, User},
    Context,
};

use crate::{
    database::DbExam,
    schedule_parser::{self, ParseExam},
    Data, Error,
};

#[derive(Clone, Debug)]
pub struct ParseInteraction {
    user: User, // User that the exams are for
    guild_id: GuildId,
    exams: Vec<ParseExam>,
}

impl Display for ParseInteraction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.exams.len() > 0 {
            write!(
            f,
            "Parsed exams for user {}\n{}\n\nuse `/parse accept` to add these exams to the bot\nuse `/parse reject` to reject these exams and stop the parsing interaction\nuse `/parse remove <id>` to remove one of these exams from the parsed exams (for example, if it was parsed incorrectly)\nuse `/parse user <user>` to change who takes these exams",
            self.user.name,
            self.exams
                .iter()
                .enumerate()
                .map(|(i, exam)| format!("{}: {}", i + 1, exam))
                .collect::<Vec<_>>()
                .join("\n")
        )
        } else {
            write!(f, "No exams found in this message.")
        }
    }
}

#[poise::command(
    context_menu_command = "Parse message and add exams",
    required_permissions = "ADMINISTRATOR",
    guild_only
)]
pub async fn add_parse_menu(
    ctx: Context<'_, Data, Error>,
    msg: serenity::model::channel::Message,
) -> Result<(), Error> {
    let safe_content = msg.content_safe(&ctx);
    let (exams, warnings) = schedule_parser::parse(&safe_content)?;

    // ctx.author() is the person who invoked the command
    let interaction = ParseInteraction {
        user: msg.author.clone(),
        guild_id: ctx.guild_id().ok_or("Not running in a guild")?,
        exams: exams.clone(),
    };
    if exams.len() > 0 {
        let mut interactions = ctx.data().parse_interactions.lock().unwrap();
        interactions.insert(ctx.author().id, interaction.clone());
    }

    // TODO: make sure this doesn't become too big of a message
    ctx.defer_ephemeral().await?;

    let interaction_message = format!("{}", interaction);

    if warnings.len() > 0 {
        let mut warnings_message = String::new();
        for warning in warnings {
            let warning_message = warning.to_string();
            if (interaction_message.len() + warnings_message.len() + warning_message.len() + 80)
                > 2000
            {
                warnings_message +=
                    "More warnings were hidden as to not exceed the maximum message length\n";
                break;
            }
            warnings_message += warning_message.as_str();
            warnings_message += "\n";
        }
        ctx.say(format!("{}\n\n{}", warnings_message, interaction_message))
            .await?;
    } else {
        ctx.say(format!("{}", interaction_message)).await?;
    }

    // msg.reply(ctx, format!("{}", interaction)).await?;

    Ok(())
}

#[poise::command(
    slash_command,
    required_permissions = "ADMINISTRATOR",
    subcommands("accept", "reject", "remove", "user"),
    guild_only
)]
pub async fn parse(_ctx: Context<'_, Data, Error>) -> Result<(), Error> {
    Ok(())
}

/// Accept the parsed exams
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR", guild_only)]
pub async fn accept(ctx: Context<'_, Data, Error>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let interaction = {
        let mut interactions = ctx.data().parse_interactions.lock().unwrap();
        interactions.remove(&ctx.author().id)
    };

    let database = ctx.data().database.clone();
    let scheduler = ctx.data().scheduler.clone();

    if let Some(interaction) = interaction {
        let mut duplicates = 0;
        let mut inserted = 0;
        for exam in interaction.exams.iter() {
            // Insert into db and add to scheduler

            match database
                .insert_exam(DbExam {
                    day: exam.day,
                    exam_id: 0,
                    exam_name: exam.name.to_owned(),
                    guild_id: interaction.guild_id,
                    user_id: interaction.user.id,
                })
                .await
            {
                Ok(exam_id) => {
                    scheduler.add_exam(exam_id).await?;
                    inserted += 1;
                }
                Err(sqlx::Error::Database(err)) if err.message().contains("duplicate") => {
                    // Means the exam is already in the db, we can ignore :)
                    duplicates += 1;
                }
                err => {
                    err?;
                }
            }
        }
        let message = if duplicates > 0 {
            format!(
                "Inserted {} exams. {} duplicates already in the bot.",
                inserted, duplicates
            )
        } else {
            format!("Inserted {} exams.", inserted)
        };
        ctx.say(message).await?;
    } else {
        ctx.say("You don't have an ongoing 'parse' interaction. Use the context menu to start one first! (right click > Apps > Parse message and add exams)").await?;
    }

    Ok(())
}

/// Reject the parsed exams
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR", guild_only)]
pub async fn reject(ctx: Context<'_, Data, Error>) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let interaction = {
        let mut interactions = ctx.data().parse_interactions.lock().unwrap();
        interactions.remove(&ctx.author().id)
    };

    if let Some(_) = interaction {
        ctx.say("Rejected parse interaction.").await?;
    } else {
        ctx.say("You don't have an ongoing 'parse' interaction. Use the context menu to start one first! (right click > Apps > Parse message and add exams)").await?;
    }

    Ok(())
}

/// Remove one of the parsed exams
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR", guild_only)]
pub async fn remove(
    ctx: Context<'_, Data, Error>,
    #[description = "ID of the exam to remove"] id: usize,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let message = {
        let mut interactions = ctx.data().parse_interactions.lock().unwrap();
        let interaction = interactions.get_mut(&ctx.author().id);

        if let Some(interaction) = interaction {
            let num_exams = interaction.exams.len();
            if (id >= 1) && (id <= num_exams) {
                interaction.exams.remove(id - 1);
                format!("{}", interaction)
            } else {
                "Invalid index".to_owned()
            }
        } else {
            "You don't have an ongoing 'parse' interaction. Use the context menu to start one first! (right click > Apps > Parse message and add exams)".to_owned()
        }
    };

    ctx.say(message).await?;

    Ok(())
}

/// Change the user this exam is parsed for
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR", guild_only)]
pub async fn user(
    ctx: Context<'_, Data, Error>,
    #[description = "User to change to"] user: User,
) -> Result<(), Error> {
    ctx.defer_ephemeral().await?;

    let message = {
        let mut interactions = ctx.data().parse_interactions.lock().unwrap();
        let interaction = interactions.get_mut(&ctx.author().id);

        if let Some(interaction) = interaction {
            interaction.user = user;
            format!("{}", interaction)
        } else {
            "You don't have an ongoing 'parse' interaction. Use the context menu to start one first! (right click > Apps > Parse message and add exams)".to_owned()
        }
    };

    ctx.say(message).await?;

    Ok(())
}
