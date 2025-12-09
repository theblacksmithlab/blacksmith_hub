use core::local_db::the_viper_room::user_management;
use crate::the_viper_room_bot::the_viper_room_bot_utils::{
    generate_podcast, schedule_podcast, send_generated_podcast_via_telegram_agent, send_main_menu,
    stop_daily_podcasts,
};
use core::models::common::system_messages::AppsSystemMessages;
use core::models::common::system_messages::{CommonMessages, TheViperRoomBotMessages};
use core::models::tg_bot::the_viper_room_bot::the_viper_room_bot_commands::TheViperRoomBotCommands;
use core::models::the_viper_room::db_models::Recipient;
use core::models::the_viper_room::the_viper_room_bot::MainMenuMessageType;
use core::state::tg_bot::TheViperRoomBotState;
use core::utils::common::get_message;
use core::utils::tg_bot::tg_bot::{
    check_username_from_message, get_chat_title, get_username_from_message,
};
use std::env;
use std::sync::Arc;
use teloxide::prelude::{Message, Requester};
use teloxide::Bot;
use teloxide_core::payloads::SendPhotoSetters;
use teloxide_core::types::{InputFile, KeyboardButton, KeyboardMarkup, ParseMode, UserId};
use tracing::info;

pub(crate) async fn the_viper_room_command_handler(
    bot: Bot,
    msg: Message,
    cmd: TheViperRoomBotCommands,
    app_state: Arc<TheViperRoomBotState>,
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
    let user_id = user.id.0;
    let chat_title = get_chat_title(&msg);
    let photo_path = "common_res/the_viper_room/avatar.jpeg";

    let lord_admin_id: u64 = env::var("LORD_ADMIN_ID")
        .expect("LORD_ADMIN_ID environment variable must be set")
        .parse()
        .expect("LORD_ADMIN_ID must be a valid integer");

    let tg_agent_id: u64 = env::var("TG_AGENT_ID")
        .expect("TG_AGENT_ID must be set in environment")
        .parse()
        .expect("TG_AGENT_ID must be a valid i64 number");

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
            
            if let Some(db_pool) = &app_state.core.db_pool {
                let first_name = Some(user.first_name.as_str());
                let last_name = user.last_name.as_deref();

                user_management::create_or_update_user(
                    db_pool.as_ref(),
                    user_id,
                    Some(&username),
                    first_name,
                    last_name,
                    app_state.clone(),
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

        TheViperRoomBotCommands::Podcast if user_id == lord_admin_id => {
            bot.send_message(chat_id, "Starting podcast generation by /podcast cmd...")
                .await?;

            let podcast_path =
                generate_podcast(app_state.clone(), tg_agent_id, Recipient::Public).await?;

            bot.send_message(chat_id, "Podcast generated! Sending to channel...")
                .await?;

            send_generated_podcast_via_telegram_agent(
                podcast_path,
                &app_state.telegram_agent.client,
                "the_viper_room",
            )
            .await?;

            bot.send_message(chat_id, "Podcast sent successfully!")
                .await?;
        }

        // Testing podcast generation
        TheViperRoomBotCommands::Test if user_id == lord_admin_id => {
            bot.send_message(chat_id, "Starting test podcast generation by /test cmd...")
                .await?;

            let podcast_path =
                generate_podcast(app_state.clone(), tg_agent_id, Recipient::Public).await?;

            bot.send_message(chat_id, "Podcast generated! Sending to test chat...")
                .await?;

            send_generated_podcast_via_telegram_agent(
                podcast_path,
                &app_state.telegram_agent.client,
                "nervosettestchat",
            )
            .await?;

            bot.send_message(chat_id, "Test podcast sent successfully!")
                .await?;
        }

        TheViperRoomBotCommands::Schedule if user_id == lord_admin_id => {
            schedule_podcast(bot.clone(), chat_id, app_state.clone(), tg_agent_id).await?;
            bot.send_message(
                chat_id,
                "Daily podcast generation scheduled by /schedule cmd",
            )
            .await?;
        }

        TheViperRoomBotCommands::Stop if user_id == lord_admin_id => {
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
