use std::str::FromStr;

use chrono::NaiveDate;
use log::info;
use poise::serenity_prelude::{ChannelId, GuildId, UserId};
use serenity::prelude::TypeMapKey;
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    Pool, Postgres,
};

use crate::Error;

// Should be safe to clone right now
// If new fields ever get added here in addition to pool, make sure they're fine to clone as well!
#[derive(Clone)]
pub struct Database {
    pool: Pool<Postgres>,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct DbGuild {
    pub guild_id: GuildId,
    pub message_channel_id: ChannelId,
    pub message_time: chrono::NaiveTime,
    pub message_timezone: chrono_tz::Tz,
    pub format: String,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct DbExam {
    pub exam_id: i64,
    pub user_id: UserId,
    pub guild_id: GuildId,
    pub day: NaiveDate,
    pub exam_name: String,
}

impl Database {
    pub async fn get_guild(&self, guild_id: GuildId) -> Result<Option<DbGuild>, Error> {
        let guild = sqlx::query!(
            "SELECT * FROM guilds WHERE guild_id = $1;",
            guild_id.0 as i64
        )
        .fetch_optional(&self.pool)
        .await?;

        if let Some(guild) = guild {
            Ok(Some(DbGuild {
                guild_id: GuildId::from(guild.guild_id as u64),
                message_channel_id: ChannelId::from(guild.message_channel_id as u64),
                message_time: guild.message_time,
                message_timezone: guild.message_timezone.parse::<chrono_tz::Tz>()?,
                format: guild.format,
            }))
        } else {
            Ok(None)
        }
    }

    pub async fn set_guild(&self, guild: DbGuild) -> Result<(), Error> {
        let message_time = guild.message_time;
        let message_timezone = guild.message_timezone.to_string();
        sqlx::query!(
            "INSERT INTO guilds(guild_id, message_channel_id, message_time, message_timezone, format) VALUES($1, $2, $3, $4, $5)
        ON CONFLICT(guild_id) DO UPDATE SET message_channel_id=excluded.message_channel_id, message_time=excluded.message_time, message_timezone=excluded.message_timezone, format=excluded.format;",
            guild.guild_id.0 as i64,
            guild.message_channel_id.0 as i64,
            message_time,
            message_timezone,
            guild.format
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn get_all_exams(&self) -> Result<Vec<DbExam>, Error> {
        let exams: Vec<_> = sqlx::query!("SELECT * FROM exams")
            .fetch_all(&self.pool)
            .await?;

        let exams = exams
            .into_iter()
            .map(|exam| DbExam {
                exam_id: exam.exam_id,
                user_id: UserId(exam.user_id as u64),
                guild_id: GuildId(exam.guild_id as u64),
                day: exam.day,
                exam_name: exam.exam_name,
            })
            .collect();

        Ok(exams)
    }

    pub async fn get_guild_exams(&self, guild_id: GuildId) -> Result<Vec<DbExam>, Error> {
        let exams: Vec<_> =
            sqlx::query!("SELECT * FROM exams WHERE guild_id = $1", guild_id.0 as i64)
                .fetch_all(&self.pool)
                .await?;

        let exams = exams
            .into_iter()
            .map(|exam| DbExam {
                exam_id: exam.exam_id,
                user_id: UserId(exam.user_id as u64),
                guild_id: GuildId(exam.guild_id as u64),
                day: exam.day,
                exam_name: exam.exam_name,
            })
            .collect();

        Ok(exams)
    }

    pub async fn get_exam(&self, exam_id: i64) -> Result<Option<DbExam>, Error> {
        let exam: Option<_> = sqlx::query!("SELECT * FROM exams WHERE exam_id = $1", exam_id)
            .fetch_optional(&self.pool)
            .await?;

        let exam = exam.map(|exam| DbExam {
            exam_id: exam.exam_id,
            user_id: UserId(exam.user_id as u64),
            guild_id: GuildId(exam.guild_id as u64),
            day: exam.day,
            exam_name: exam.exam_name,
        });
        Ok(exam)
    }

    pub async fn get_user_exams(
        &self,
        guild_id: GuildId,
        user_id: UserId,
    ) -> Result<Vec<DbExam>, Error> {
        let exams: Vec<_> = sqlx::query!(
            "SELECT * FROM exams WHERE guild_id = $1 AND user_id = $2",
            guild_id.0 as i64,
            user_id.0 as i64
        )
        .fetch_all(&self.pool)
        .await?;

        let exams = exams
            .into_iter()
            .map(|exam| DbExam {
                exam_id: exam.exam_id,
                user_id: UserId(exam.user_id as u64),
                guild_id: GuildId(exam.guild_id as u64),
                day: exam.day,
                exam_name: exam.exam_name,
            })
            .collect();

        Ok(exams)
    }

    // Inserts a DbExam, ignoring the exam_id
    pub async fn insert_exam(&self, exam: DbExam) -> Result<i64, Error> {
        let ret = sqlx::query!(
            "INSERT INTO exams(user_id, guild_id, day, exam_name) VALUES($1, $2, $3, $4) RETURNING exam_id;",
            exam.user_id.0 as i64,
            exam.guild_id.0 as i64,
            exam.day,
            exam.exam_name
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(ret.exam_id)
    }

    // Deletes a DbExam
    pub async fn delete_exam(&self, exam_id: i64) -> Result<(), Error> {
        sqlx::query!("DELETE FROM exams WHERE exam_id=$1;", exam_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

impl TypeMapKey for Database {
    type Value = Database;
}

pub async fn setup_database(url: &str) -> Result<Database, sqlx::Error> {
    let connect_options = PgConnectOptions::from_str(url)?;

    info!("Connecting to database...");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;
    info!("Connected to database");

    // Run migrations
    info!("Running any pending database migrations...");
    sqlx::migrate!().run(&pool).await?;
    info!("Done running migrations");

    Ok(Database { pool })
}
