use crate::the_viper_room_bot::the_viper_room_bot_utils::{
    generate_podcast, schedule_podcast, send_main_menu, stop_daily_podcasts, MainMenuMessageType,
};
use core::local_db::the_viper_room::user_management;
use core::models::common::system_messages::AppsSystemMessages;
use core::models::common::system_messages::{CommonMessages, TheViperRoomBotMessages};
use core::models::tg_bot::the_viper_room_bot::the_viper_room_bot_commands::TheViperRoomBotCommands;
use core::state::tg_bot::app_state::BotAppState;
use core::telegram_client::grammers_functionality::initialize_grammers_client;
use core::utils::common::get_message;
use core::utils::tg_bot::tg_bot::{
    check_username_from_message, get_chat_title, get_username_from_message,
};
use std::path::Path;
use std::sync::Arc;
use std::{env, fs};
use teloxide::prelude::{Message, Requester};
use teloxide::Bot;
use teloxide_core::payloads::SendPhotoSetters;
use teloxide_core::types::{InputFile, KeyboardButton, KeyboardMarkup, ParseMode, UserId};
use tracing::info;

pub(crate) async fn the_viper_room_command_handler(
    bot: Bot,
    msg: Message,
    cmd: TheViperRoomBotCommands,
    app_state: Arc<BotAppState>,
) -> anyhow::Result<()> {
    if check_username_from_message(&bot, &msg).await == false {
        return Ok(());
    }
    let username = get_username_from_message(&msg);
    let chat_id = msg.chat.id;
    let user = msg
        .from
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("No user in message"))?;
    let user_id = UserId(user.id.0);
    let chat_title = get_chat_title(&msg);
    let photo_path = "common_res/the_viper_room/avatar.jpeg";

    let nickname_for_public_podcast: String = env::var("PUBLIC_PODCAST_NICKNAME")
        .expect("PUBLIC_PODCAST_NICKNAME environment variable not set")
        .parse()
        .expect("PUBLIC_PODCAST_NICKNAME environment variable is not a valid string");

    let lord_admin_id: u64 = env::var("LORD_ADMIN_ID")
        .expect("LORD_ADMIN_ID environment variable must be set")
        .parse()
        .expect("LORD_ADMIN_ID must be a valid integer");

    let tg_agent_id =
        Arc::new(env::var("TG_AGENT_ID").expect("TG_AGENT_ID must be set in environment"));

    let session_path = format!(
        "common_res/the_viper_room/grammers_system_session/{}.session",
        tg_agent_id
    );

    if !Path::new(&session_path).exists() {
        return Err(anyhow::anyhow!(
            "Telegram agent session file not found: {}. Please ensure the session file exists",
            session_path
        ));
    }

    let session_data = fs::read(Path::new(&session_path))
        .map_err(|e| anyhow::anyhow!("Failed to read session file {}: {}", session_path, e))?;

    let g_client = initialize_grammers_client(session_data.clone()).await?;

    if matches!(
        cmd,
        TheViperRoomBotCommands::Start
            | TheViperRoomBotCommands::Stop
            | TheViperRoomBotCommands::Podcast
            | TheViperRoomBotCommands::Test
            | TheViperRoomBotCommands::Schedule
    ) {
        if !msg.chat.is_private() {
            info!(
                "User: {} [{}] tried to execute {:?} cmd in public chat: {} [{}]",
                username, user_id, cmd, chat_title, chat_id
            );
            let bot_msg = get_message(AppsSystemMessages::Common(
                CommonMessages::PrivateCmdUsedInPublicChat,
            ))
            .await?;
            bot.send_message(chat_id, bot_msg).await?;
            return Ok(());
        }
    }

    match cmd {
        TheViperRoomBotCommands::Start => {
            info!(
                "User: {} [{}] executed {:?} cmd in private chat",
                username, user_id, cmd
            );

            // Register or update user in database
            if let Some(db_pool) = &app_state.db_pool {
                let user_id_i64 = user_id.0 as i64;
                user_management::create_or_update_user(
                    db_pool.as_ref(),
                    user_id_i64,
                    Some(&username),
                )
                .await?;
                info!(
                    "User {} [{}] registered/updated in database",
                    username, user_id
                );
            }

            let welcome_text_template = get_message(AppsSystemMessages::TheViperRoomBot(
                TheViperRoomBotMessages::StartMessage,
            ))
            .await?;
            let welcome_text = welcome_text_template.replace("{}", &username.to_string());

            let keyboard = KeyboardMarkup::new(vec![
                vec![KeyboardButton::new("🏠 Главное меню")],
                vec![KeyboardButton::new("❓ Задать вопрос")],
            ])
            .resize_keyboard()
            .one_time_keyboard();

            bot.send_photo(chat_id, InputFile::file(photo_path))
                .caption(welcome_text)
                .parse_mode(ParseMode::Html)
                .reply_markup(keyboard)
                .await?;
        }

        TheViperRoomBotCommands::Menu => {
            info!(
                "User: {} [{}] executed {:?} cmd in private chat",
                username, user_id, cmd
            );

            send_main_menu(
                &bot,
                user_id,
                chat_id,
                &app_state,
                MainMenuMessageType::Full,
            )
            .await?;
        }

        TheViperRoomBotCommands::Podcast if user_id.0 == lord_admin_id => {
            bot.send_message(chat_id, "Starting podcast generation by /podcast cmd...")
                .await?;
            generate_podcast(
                g_client,
                bot.clone(),
                chat_id,
                app_state.clone(),
                &tg_agent_id,
                nickname_for_public_podcast,
                "the_viper_room",
            )
            .await?;
        }

        // Testing podcast generation
        TheViperRoomBotCommands::Test if user_id.0 == lord_admin_id => {
            bot.send_message(chat_id, "Starting test podcast generation by /test cmd...")
                .await?;
            generate_podcast(
                g_client,
                bot.clone(),
                chat_id,
                app_state.clone(),
                &tg_agent_id,
                nickname_for_public_podcast,
                "nervosettestchat",
            )
            .await?;
        }

        TheViperRoomBotCommands::Schedule if user_id.0 == lord_admin_id => {
            schedule_podcast(
                bot.clone(),
                chat_id,
                app_state.clone(),
                tg_agent_id,
                nickname_for_public_podcast,
                session_data,
            )
            .await?;
            bot.send_message(
                chat_id,
                "Daily podcast generation scheduled by /schedule cmd",
            )
            .await?;
        }

        TheViperRoomBotCommands::Stop if user_id.0 == lord_admin_id => {
            stop_daily_podcasts(app_state.clone()).await?;
            bot.send_message(chat_id, "Daily podcast generation stopped by /stop cmd")
                .await?;
        }

        _ => {
            let bot_msg = get_message(AppsSystemMessages::TheViperRoomBot(
                TheViperRoomBotMessages::WrongCmdOrNoRightsMessage,
            ))
            .await?;
            bot.send_message(chat_id, bot_msg).await?;
        }
    }

    Ok(())
}
