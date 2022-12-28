use chrono::{NaiveTime, Utc};
use chrono_tz::Tz;
use poise::{serenity_prelude::Mentionable, Context};
use serenity::model::channel::Channel;

use crate::{
    database::{DbExam, DbGuild},
    default_channel,
    formatter::{format_exam, DEFAULT_FORMAT},
    Data, Error,
};

/// Change the bot's settings for this server
#[poise::command(
    slash_command,
    subcommands("channel", "time", "list", "message"),
    guild_only,
    required_permissions = "ADMINISTRATOR"
)]
pub async fn settings(_ctx: Context<'_, Data, Error>) -> Result<(), Error> {
    Ok(())
}

/// Change the channel in which this bot sends messages
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn channel(
    ctx: Context<'_, Data, Error>,
    #[description = "What channel to send messages in"]
    #[channel_types("Text")]
    channel: Channel,
) -> Result<(), Error> {
    // Get current settings
    let database = &ctx.data().database;

    let guild = ctx.guild().ok_or("Not running in a guild")?;
    let channel = channel.guild().ok_or("Didn't pass a valid channel")?;

    let current_user_id = ctx.serenity_context().cache.current_user_id();

    let permissions = channel.permissions_for_user(&ctx, &current_user_id)?;
    println!("Bot has these permissions in {}:\n{}", channel, permissions);
    if !permissions.send_messages() {
        ctx.say(format!(
            "This bot doesn't have permissions to send messages in {}",
            channel
        ))
        .await?;
        return Ok(());
    }

    let guild_settings = if let Some(mut guild_settings) = database.get_guild(guild.id).await? {
        // Modify
        guild_settings.message_channel_id = channel.id;
        guild_settings
    } else {
        // Insert (shouldn't happen but ok)
        DbGuild {
            guild_id: guild.id,
            message_channel_id: default_channel(ctx.serenity_context(), &guild).await?,
            message_time: NaiveTime::from_hms_opt(21, 0, 0).unwrap(),
            message_timezone: chrono_tz::UTC,
            format: DEFAULT_FORMAT.to_string(),
        }
    };

    database.set_guild(guild_settings).await?;

    ctx.say(format!("Updated bot message channel to {}!", channel))
        .await?;
    Ok(())
}

/// Change the time at which this bot sends messages
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn time(
    ctx: Context<'_, Data, Error>,
    #[description = "What channel to send messages in (24h notation, format: \"hour:minute\")"]
    time: String,
    #[description = "What timezone to use."] timezone: String,
) -> Result<(), Error> {
    let database = &ctx.data().database;

    let guild = ctx.guild().ok_or("Not running in a guild")?;

    let timezone: Option<Tz> = match timezone.parse() {
        Ok(timezone) => Some(timezone),
        Err(_) => None,
    };

    let time: Option<chrono::NaiveTime> = match chrono::NaiveTime::parse_from_str(&time, "%H:%M") {
        Ok(time) => Some(time),
        Err(_) => None,
    };

    match (time, timezone) {
        (Some(time), Some(timezone)) => {
            let guild_settings =
                if let Some(mut guild_settings) = database.get_guild(guild.id).await? {
                    // Modify
                    guild_settings.message_time = time;
                    guild_settings.message_timezone = timezone;
                    guild_settings
                } else {
                    // Insert (shouldn't happen but ok)
                    DbGuild {
                        guild_id: guild.id,
                        message_channel_id: default_channel(ctx.serenity_context(), &guild).await?,
                        message_time: NaiveTime::from_hms_opt(21, 0, 0).unwrap(),
                        message_timezone: chrono_tz::UTC,
                        format: DEFAULT_FORMAT.to_string(),
                    }
                };
            database.set_guild(guild_settings).await?;
            ctx.say(format!(
                "Updated bot message time to {} {}!",
                time, timezone
            ))
            .await?;
        }
        (Some(_), None) => {
            ctx.say("Invalid timezone").await?;
        }
        (None, Some(_)) => {
            ctx.say("Invalid time format").await?;
        }
        (None, None) => {
            ctx.say("Invalid time format and timezone").await?;
        }
    }

    Ok(())
}

/// Change the message this bot sends
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn message(
    ctx: Context<'_, Data, Error>,
    #[description = "The format string the bot uses."] format: String,
) -> Result<(), Error> {
    let database = &ctx.data().database;

    let guild = ctx.guild().ok_or("Not running in a guild")?;

    let guild_settings = if let Some(mut guild_settings) = database.get_guild(guild.id).await? {
        // Modify
        guild_settings.format = format.clone();
        guild_settings
    } else {
        // Insert (shouldn't happen but ok)
        DbGuild {
            guild_id: guild.id,
            message_channel_id: default_channel(ctx.serenity_context(), &guild).await?,
            message_time: NaiveTime::from_hms_opt(21, 0, 0).unwrap(),
            message_timezone: chrono_tz::UTC,
            format: format.clone(),
        }
    };

    let nameless_exam = DbExam {
        day: Utc::now().date_naive(),
        exam_id: 0,
        exam_name: "".to_string(),
        guild_id: guild.id,
        user_id: ctx.author().id,
    };
    let named_exam = DbExam {
        day: Utc::now().date_naive(),
        exam_id: 0,
        exam_name: "Algorithms and Datastructures".to_string(),
        guild_id: guild.id,
        user_id: ctx.author().id,
    };

    database.set_guild(guild_settings).await?;
    let message = format!("Updated bot format to \"{}\"!\n\nExample for nameless exam:\n{}\n\nExample for named exam:\n{}", format, format_exam(&format, nameless_exam), format_exam(&format, named_exam));
    ctx.say(message).await?;

    Ok(())
}

/// Change the time at which this bot sends messages
#[poise::command(slash_command, required_permissions = "ADMINISTRATOR")]
pub async fn list(ctx: Context<'_, Data, Error>) -> Result<(), Error> {
    let database = &ctx.data().database;
    let guild = ctx.guild().ok_or("Not running in a guild")?;

    let guild_settings = if let Some(guild_settings) = database.get_guild(guild.id).await? {
        guild_settings
    } else {
        ctx.say(format!("No settings saved for this guild."))
            .await?;
        return Ok(());
    };

    ctx.say(format!(
        "**Settings**:\nChannel: {}\nTime: {} {}\nFormat: {}",
        guild_settings.message_channel_id.mention(),
        guild_settings.message_time,
        guild_settings.message_timezone,
        guild_settings.format
    ))
    .await?;

    Ok(())
}
