use crate::utils::{generate_podcast, schedule_podcast, stop_daily_podcasts};
use anyhow::Result;
use core::grammers::grammers_functionality::initialize_grammers_client;
use core::state::tg_bot::app_state::BotAppState;
use core::utils::common::get_message;
use std::path::Path;
use std::sync::Arc;
use std::{env, fs};
use teloxide::macros::BotCommands;
use teloxide::prelude::{ChatId, Message, Requester};
use teloxide::Bot;
use tracing::info;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase")]
pub enum BotCommands {
    Start,
    Podcast,
    Schedule,
    Stop,
}

pub(crate) async fn message_handler(bot: Bot, msg: Message) -> Result<()> {
    let user_id = msg.chat.id;
    let bot_msg = get_message("the_viper_room", "auto_reply").await?;
    bot.send_message(user_id, bot_msg).await?;

    Ok(())
}

pub(crate) async fn command_handler(
    bot: Bot,
    msg: Message,
    cmd: BotCommands,
    app_state: Arc<BotAppState>,
) -> Result<()> {
    let user_id = msg.chat.id;
    
    let lord_admin_id: i64 = env::var("LORD_ADMIN_ID")
        .expect("LORD_ADMIN_ID environment variable must be set")
        .parse()
        .expect("LORD_ADMIN_ID must be a valid integer");

    let app_tg_account_id = ChatId(
        env::var("APP_TG_ACCOUNT_ID")
            .expect("APP_TG_ACCOUNT_ID must be set in environment")
            .parse()
            .expect("APP_TG_ACCOUNT_ID must be a valid integer")
    );
   
    let session_path = format!("common_res/grammers_system_sessions/{}.session", app_tg_account_id.0);

    if !Path::new(&session_path).exists() {
        return Err(anyhow::anyhow!(
        "System session file not found: {}. Please ensure the session file exists", 
        session_path
    ));
    }
    
    let session_data = fs::read(Path::new(&session_path))
        .map_err(|e| anyhow::anyhow!("Failed to read session file {}: {}", session_path, e))?;

    let nickname = "Public".to_string();
    
    let g_client = initialize_grammers_client(session_data.clone()).await?;

    match cmd {
        BotCommands::Start => {
            info!("Healthy user starts the App... Ok");
            let bot_msg = get_message("the_viper_room", "start_message").await?;
            bot.send_message(user_id, bot_msg).await?;
        }

        BotCommands::Podcast if user_id.0 == lord_admin_id => {
            bot.send_message(user_id, "Starting podcast generation by /podcast cmd...")
                .await?;
            generate_podcast(
                g_client,
                bot.clone(),
                user_id,
                app_state.clone(),
                app_tg_account_id,
                nickname,
            )
            .await?;
        }

        BotCommands::Schedule if user_id.0 == lord_admin_id => {
            schedule_podcast(
                bot.clone(),
                user_id,
                app_state.clone(),
                app_tg_account_id,
                nickname,
                session_data,
            )
            .await?;
            bot.send_message(
                user_id,
                "Daily podcast generation scheduled by /schedule cmd",
            )
            .await?;
        }

        BotCommands::Stop if user_id.0 == lord_admin_id => {
            stop_daily_podcasts(app_state.clone()).await?;
            bot.send_message(user_id, "Daily podcast generation stopped by /stop cmd")
                .await?;
        }

        _ => {
            let bot_msg = get_message("the_viper_room", "wrong_cmd_or_no_rights_message").await?;
            bot.send_message(user_id, bot_msg).await?;
        }
    }

    Ok(())
}
