// Settings:
// When the message gets sent (the day before on a set hour, the morning of on a set hour, etc)
//      alternatively, x hours before the exam?
// What channel the message gets sent in

// sqlx w/ sqlite3 backend?
// has migrations support :)

// Tables:
// Servers + settings per server
//  Server_ID (From Discord)
//  Channel
//
// Exams table
//  User id (From Discord)
//  Day
//  Server

// Functionality
//  gelukswensen
//  overview van examens

use std::env;

use hanne_is_leuk_bot::{run_bot, Error};

use log::{error, warn};

use dotenvy::dotenv;

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenv().ok();

    env_logger::init();

    let token = env::var("DISCORD_TOKEN").expect("Missing Discord token");
    let database_url = env::var("DATABASE_URL").expect("Missing database url");

    if let Err(why) = run_bot(token, database_url).await {
        error!("An error occurred while running the bot: {:?}", why);
    } else {
        warn!("Exiting the program without errors");
    }

    Ok(())
}
