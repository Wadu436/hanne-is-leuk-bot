use std::{
    cmp::Ordering,
    sync::{Arc, Mutex},
    time::Duration,
};

use std::collections::BinaryHeap;

use crate::{
    database::{Database, DbExam, DbGuild},
    formatter::format_exam,
    Error,
};
use chrono::{DateTime, Days, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use poise::serenity_prelude::Mentionable;
use serenity::client::Context;
use tokio::time::{self, MissedTickBehavior};

// Schedules when an alarm is played
pub struct Scheduler {
    database: Database,
    exams: Mutex<BinaryHeap<ScheduledExam>>,
    bot_context: Context,
}

#[derive(Clone, Eq, PartialEq)]
pub struct ScheduledExam {
    scheduled_time: chrono::DateTime<Utc>,
    exam: DbExam,
    guild: DbGuild,
}

// The priority queue depends on `Ord`.
// Explicitly implement the trait so the queue becomes a min-heap
// instead of a max-heap.
impl Ord for ScheduledExam {
    fn cmp(&self, other: &Self) -> Ordering {
        // Notice that the we flip the ordering on costs.
        // In case of a tie we compare positions - this step is necessary
        // to make implementations of `PartialEq` and `Ord` consistent.
        other
            .scheduled_time
            .cmp(&self.scheduled_time)
            .then_with(|| self.exam.exam_id.cmp(&other.exam.exam_id))
    }
}

// `PartialOrd` needs to be implemented as well.
impl PartialOrd for ScheduledExam {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

async fn schedule_task(scheduler: Arc<Scheduler>) {
    // Load exams
    let _ = scheduler.load_exams_from_database().await;

    // Load
    let mut interval = time::interval(Duration::from_secs(60)); // Long delay, every minute

    interval.set_missed_tick_behavior(MissedTickBehavior::Skip);

    // Use initial tick
    interval.tick().await;

    loop {
        // Get current time
        {
            let now = Utc::now();
            let mut exams = scheduler.exams.lock().unwrap();

            while exams
                .peek()
                .and_then(|exam| Some(exam.scheduled_time <= now))
                .unwrap_or(false)
            {
                // Send the exam message
                let exam = exams.pop().unwrap();

                let scheduler_clone = scheduler.clone();
                tokio::spawn(async move { scheduler_clone.send_message(exam.exam.exam_id).await });
            }
        }

        interval.tick().await;
    }
}

fn calculate_schedule_time(
    date: NaiveDate,
    time: NaiveTime,
    timezone: chrono_tz::Tz,
) -> DateTime<Utc> {
    let naive_datetime = NaiveDateTime::new(date, time) - Days::new(1);
    let offset = match timezone.offset_from_local_datetime(&naive_datetime) {
        chrono::LocalResult::None => timezone.offset_from_utc_datetime(&naive_datetime),
        chrono::LocalResult::Single(offset) => offset,
        chrono::LocalResult::Ambiguous(offset, _) => offset,
    };

    let datetime_local: DateTime<Tz> = DateTime::from_local(naive_datetime, offset);
    datetime_local.with_timezone(&Utc)
}

impl Scheduler {
    pub fn new(database: Database, ctx: Context) -> Arc<Self> {
        let scheduler = Arc::new(Scheduler {
            database,
            exams: Mutex::new(BinaryHeap::new()),
            bot_context: ctx,
        });

        tokio::spawn(schedule_task(scheduler.clone()));

        scheduler
    }
    async fn send_message(&self, exam_id: i64) -> Result<(), Error> {
        if let Some(exam) = self.database.get_exam(exam_id).await? {
            if let Some(guild) = self.database.get_guild(exam.guild_id).await? {
                guild
                    .message_channel_id
                    .say(
                        &self.bot_context,
                        format!(
                            "{}\n{}",
                            exam.user_id.mention(),
                            format_exam(guild.format, exam)
                        ),
                    )
                    .await?;
            }
        };

        Ok(())
    }

    pub async fn load_exams_from_database(&self) -> Result<(), Error> {
        let exams_database = self.database.get_all_exams().await?;

        println!("Loading {} exams", exams_database.len());
        let mut exams_vec = Vec::new();
        for exam_database in exams_database {
            // Calculate scheduled time
            if let Some(guild_database) = self.database.get_guild(exam_database.guild_id).await? {
                let datetime_utc: DateTime<Utc> = calculate_schedule_time(
                    exam_database.day,
                    guild_database.message_time,
                    guild_database.message_timezone,
                );
                let exam = ScheduledExam {
                    scheduled_time: datetime_utc,
                    exam: exam_database,
                    guild: guild_database,
                };
                exams_vec.push(exam);
            }
        }

        {
            let mut exams = self.exams.lock().map_err(|_| "Error locking Mutex")?;
            exams.clear();
            for exam in exams_vec {
                exams.push(exam)
            }
        }

        Ok(())
    }

    pub async fn add_exam(&self, exam_id: i64) -> Result<(), Error> {
        if let Some(exam_database) = self.database.get_exam(exam_id).await? {
            if let Some(guild_database) = self.database.get_guild(exam_database.guild_id).await? {
                let datetime_utc: DateTime<Utc> = calculate_schedule_time(
                    exam_database.day,
                    guild_database.message_time,
                    guild_database.message_timezone,
                );
                let exam = ScheduledExam {
                    scheduled_time: datetime_utc,
                    exam: exam_database,
                    guild: guild_database,
                };
                {
                    let mut exams = self.exams.lock().map_err(|_| "Error locking Mutex")?;
                    exams.push(exam);
                }
            }
        }

        Ok(())
    }
}
