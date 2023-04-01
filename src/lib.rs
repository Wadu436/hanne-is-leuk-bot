use std::collections::HashMap;
use std::{sync::Arc, vec};

use chrono::NaiveTime;
use database::Database;
use formatter::DEFAULT_FORMAT;
use log::{debug, info};
use poise::serenity_prelude::{CommandDataOption, UserId};
use poise::{FrameworkContext, FrameworkOptions};

use scheduler::Scheduler;
use serenity::prelude::*;

use crate::database::DbGuild;

mod commands;
mod database;
mod formatter;
mod schedule_parser;
mod scheduler;

use serenity::model::guild::Guild;
use serenity::model::id::ChannelId;

use commands::ParseInteraction;

use std::sync::Mutex;

pub type Error = Box<dyn std::error::Error + Send + Sync>;

pub struct Data {
    database: Database,
    scheduler: Arc<Scheduler>,
    parse_interactions: Arc<Mutex<HashMap<UserId, ParseInteraction>>>,
}

fn format_options(options: &Vec<CommandDataOption>) -> String {
    options
        .iter()
        .map(|option| format_option(option))
        .collect::<Vec<String>>()
        .join(",")
}

fn format_option(option: &CommandDataOption) -> String {
    match option.kind {
        poise::serenity_prelude::CommandOptionType::SubCommand
        | poise::serenity_prelude::CommandOptionType::SubCommandGroup => {
            format!("{}: ({})", option.name, format_options(&option.options))
        }
        _ => {
            format!("{}: {:?}", option.name, option.value)
        }
    }
}

async fn event_handler(
    ctx: &Context,
    event: &poise::Event<'_>,
    framework: FrameworkContext<'_, Data, Error>,
) -> Result<(), Error> {
    match event {
        poise::Event::Ready { data_about_bot } => {
            info!("{} is connected!", data_about_bot.user.name);
            // Register commands
            poise::builtins::register_globally(ctx, &framework.options().commands).await?;

            Ok(())
        }
        poise::Event::GuildCreate { guild, .. } => {
            // Check if the guild is already in the database, if not, add it
            let database = &framework.user_data.database;

            if let Ok(None) = database.get_guild(guild.id).await {
                let _ = database
                    .set_guild(DbGuild {
                        guild_id: guild.id,
                        message_channel_id: default_channel(ctx, guild).await?,
                        message_time: NaiveTime::from_hms_opt(21, 0, 0).unwrap(),
                        message_timezone: chrono_tz::UTC,
                        format: DEFAULT_FORMAT.to_string(),
                    })
                    .await;
            }
            Ok(())
        }
        poise::Event::InteractionCreate { interaction } => {
            if let Some(command) = interaction.clone().application_command() {
                let options = format_options(&command.data.options);
                debug!(
                    "Received command {:?} with options ({})",
                    command.data.name, options
                );
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

async fn default_channel(ctx: &Context, guild: &Guild) -> Result<ChannelId, Error> {
    let channel_id = if let Some(channel_id) = guild.system_channel_id {
        channel_id
    } else {
        guild
            .channels(&ctx)
            .await?
            .into_iter()
            .find(|(_, channel)| {
                channel
                    .permissions_for_user(&ctx, ctx.cache.current_user_id())
                    .and_then(|perms| Ok(perms.send_messages()))
                    .unwrap_or(false)
            })
            .and_then(|(channel_id, _)| Some(channel_id))
            .ok_or("Couldn't find channel")?
    };

    Ok(channel_id)
}

pub async fn run_bot(token: String, database_url: String) -> Result<(), Error> {
    let database = database::setup_database(&database_url).await?;

    let framework = poise::Framework::builder()
        .options(FrameworkOptions {
            commands: vec![
                commands::settings(),
                commands::exam(),
                commands::exams(),
                commands::add_parse_menu(),
                commands::parse(),
            ],
            event_handler: |ctx, event, framework, _data| {
                Box::pin(async move { event_handler(ctx, event, framework).await })
            },
            ..Default::default()
        })
        .setup(move |ctx, _ready, _framework| {
            Box::pin(async move {
                Ok(Data {
                    database: database.clone(),
                    scheduler: Scheduler::new(database, ctx.clone()),
                    parse_interactions: Arc::new(Mutex::new(HashMap::new())),
                })
            })
        })
        .token(token)
        .intents(GatewayIntents::GUILDS)
        .build()
        .await?;

    framework.start().await?;

    Ok(())
}
