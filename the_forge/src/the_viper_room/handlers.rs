use crate::the_viper_room::local_utils::{generate_user_system_nickname, get_user_system_nickname};
use axum::extract::State;
use axum::Json;
use core::models::the_viper_room::the_viper_room_web_ui::AuthStage::MiniAppInitConfirmed;
use core::models::the_viper_room::the_viper_room_web_ui::{
    ActionStep, AuthStage, TheViperRoomServerResponse, TheViperRoomUserAction,
};
use core::state::the_viper_room::app_state::TheViperRoomAppState;
use core::state::the_viper_room::app_state_operation::{
    reset_user_state, reset_user_state_with_message,
};
use core::telegram_client::grammers_functionality::{
    initialize_grammers_client, session_file_creation,
};
use core::utils::common::{update_the_viper_room_user_data, update_the_viper_room_user_state};
use core::utils::the_viper_room::news_block_creation::news_block_creation;
use std::env;
use std::fs::{read, remove_file};
use std::sync::Arc;
use tracing::info;

pub(crate) async fn handle_the_viper_room_user_request(
    State(the_viper_room_app_state): State<Arc<TheViperRoomAppState>>,
    Json(command): Json<TheViperRoomUserAction>,
) -> Json<TheViperRoomServerResponse> {
    let user_id = command.user_id as u64;
    let user_id_as_str = user_id.to_string();
    let action = command.action;
    let action_step = command.action_step;
    let username = command.username.unwrap_or(String::from("anonymous_user"));
    let user_first_name = command.user_first_name.unwrap_or(String::from("Anonymous"));
    let user_last_name = command.user_last_name.unwrap_or(String::from("User"));
    let full_user_data = format!(
        "{} {} (username: @{}) [id: {}]",
        user_first_name, user_last_name, username, user_id
    );
    let session_data = command.session_data;
    let api_id: i32 = env::var("TELEGRAM_API_ID")
        .expect("API_ID not set")
        .parse()
        .expect("API_ID must be a number");
    let api_hash = env::var("TELEGRAM_API_HASH").expect("API_HASH not set");

    info!(
        "Received user action from @{} [id: {}], action: {:?} and/or action step: {:?}",
        username, user_id, action, action_step
    );

    if let Some(step) = action_step {
        match step {
            ActionStep::MiniAppInitialized => {
                info!("Mini-App initialized by: {}", full_user_data);
                reset_user_state(the_viper_room_app_state.clone(), user_id).await;

                match generate_user_system_nickname(
                    the_viper_room_app_state.clone(),
                    username,
                    user_first_name.clone(),
                    user_last_name.clone(),
                )
                .await
                {
                    Ok(nickname) => {
                        update_the_viper_room_user_data(
                            the_viper_room_app_state.clone(),
                            user_id,
                            |data| {
                                data.user_system_nickname = nickname.clone();
                            },
                        )
                        .await;
                        info!("User's system nickname generated: {}", nickname);
                    }
                    Err(e) => {
                        info!("Failed to generate user's system nickname: {}", e);
                    }
                }

                let nickname =
                    match get_user_system_nickname(the_viper_room_app_state.clone(), user_id).await
                    {
                        Some(nick) => nick,
                        None => format!("{} {}", user_first_name, user_last_name),
                    };

                info!(
                    "Initializing g_Client with provided session for: {}",
                    full_user_data
                );
                match initialize_grammers_client(session_data).await {
                    Ok(client) => {
                        update_the_viper_room_user_state(
                            the_viper_room_app_state.clone(),
                            user_id,
                            |state| {
                                state.authorized = true;
                                state.client = Some(client);
                            },
                        )
                        .await;

                        Json(TheViperRoomServerResponse {
                            message: format!(
                                "Приветствую, {}!\nТы авторизован, зацени доступные тебе опции",
                                nickname
                            ),
                            buttons: vec![
                                "Get news!".to_string(),
                                "Schedule newsblock".to_string(),
                            ],
                            action_buttons: vec!["Sign out".to_string()],
                            can_input: false,
                            session_data: None,
                            stage: Some(MiniAppInitConfirmed),
                            audio_data: None,
                        })
                    }
                    Err(e) => {
                        let reset_message = reset_user_state_with_message(
                            the_viper_room_app_state.clone(),
                            user_id,
                        )
                        .await;

                        Json(TheViperRoomServerResponse {
                            message: format!(
                                "Failed to initialize Telegram client with provided session: {}{}",
                                e, reset_message
                            ),
                            buttons: vec![],
                            action_buttons: vec![],
                            can_input: false,
                            session_data: None,
                            stage: None,
                            audio_data: None,
                        })
                    }
                }
            }
            ActionStep::LoginStart => {
                info!("Starting login process for: {}", full_user_data);
                update_the_viper_room_user_state(
                    the_viper_room_app_state.clone(),
                    user_id,
                    |state| {
                        state.awaiting_phone_number = true;
                    },
                )
                .await;

                Json(TheViperRoomServerResponse {
                    message: "Для авторизации введи номер телефона твоего Telegram-аккаунта (начиная с '+7 ...')".to_string(),
                    buttons: vec![],
                    action_buttons: vec![],
                    can_input: true,
                    session_data: None,
                    stage: Some(AuthStage::PhoneNumerRequest),
                    audio_data: None,
                })
            }
            ActionStep::SignOut => {
                info!("Got sign out action step from: {}", full_user_data);
                update_the_viper_room_user_state(
                    the_viper_room_app_state.clone(),
                    user_id,
                    |state| {
                        state.authorized = false;
                        state.unauthorized = true;
                    },
                )
                .await;

                info!("{} signed out", full_user_data);
                Json(TheViperRoomServerResponse {
                    message: "Good-bye for a while! Your session has been deleted".to_string(),
                    buttons: vec![],
                    action_buttons: vec![],
                    can_input: true,
                    session_data: None,
                    stage: Some(AuthStage::SignedOut),
                    audio_data: None,
                })
            }
        }
    } else if let Some(action) = action {
        let nickname =
            match get_user_system_nickname(the_viper_room_app_state.clone(), user_id).await {
                Some(nick) => nick,
                None => format!("{} {}", user_first_name, user_last_name),
            };

        let needs_session_file_creation = {
            let user_states = the_viper_room_app_state.user_state.lock().await;
            user_states
                .get(&user_id)
                .map(|state| {
                    state.awaiting_phone_number || state.awaiting_passcode || state.awaiting_2fa
                })
                .unwrap_or(false)
        };
        info!(
            "Needs_session_file_creation: {}",
            needs_session_file_creation
        );

        let action_from_authorized_user = {
            let user_states = the_viper_room_app_state.user_state.lock().await;
            user_states
                .get(&user_id)
                .map(|state| state.authorized)
                .unwrap_or(false)
        };
        info!(
            "action_from_authorized_user: {}",
            action_from_authorized_user
        );

        if needs_session_file_creation {
            return session_file_creation(
                Some(action.to_string()),
                the_viper_room_app_state.clone(),
                user_id,
                api_id,
                api_hash,
                nickname,
            )
            .await;
        }

        if action_from_authorized_user {
            if action == "Get news!" {
                let user_states = the_viper_room_app_state.user_state.lock().await;
                if let Some(state) = user_states.get(&user_id) {
                    if let Some(client) = state.client.as_ref() {
                        return match news_block_creation(
                            client,
                            &user_id_as_str,
                            the_viper_room_app_state.clone(),
                            nickname,
                            false,
                        )
                        .await
                        {
                            Ok(podcast_file) => match read(&podcast_file) {
                                Ok(audio_data) => {
                                    if let Err(e) = remove_file(&podcast_file) {
                                        info!("Failed to remove podcast file: {}", e);
                                    }

                                    Json(TheViperRoomServerResponse {
                                        message: "Держи подкаст с последними новостями!"
                                            .to_string(),
                                        buttons: vec![
                                            "Get news!".to_string(),
                                            "Schedule newsblock".to_string(),
                                        ],
                                        action_buttons: vec!["Sign out".to_string()],
                                        can_input: false,
                                        session_data: None,
                                        audio_data: Some(audio_data),
                                        stage: None,
                                    })
                                }
                                Err(e) => {
                                    info!("Failed to read podcast file: {}", e);
                                    Json(TheViperRoomServerResponse {
                                        message: "Произошла ошибка при чтении подкаста".to_string(),
                                        buttons: vec![
                                            "Get news!".to_string(),
                                            "Schedule newsblock".to_string(),
                                        ],
                                        action_buttons: vec!["Sign out".to_string()],
                                        can_input: false,
                                        session_data: None,
                                        audio_data: None,
                                        stage: None,
                                    })
                                }
                            },
                            Err(e) => {
                                info!("Failed to create news block: {}", e);
                                Json(TheViperRoomServerResponse {
                                    message: "Не удалось создать подкаст, попробуйте позже"
                                        .to_string(),
                                    buttons: vec![
                                        "Get news!".to_string(),
                                        "Schedule newsblock".to_string(),
                                    ],
                                    action_buttons: vec!["Sign out".to_string()],
                                    can_input: false,
                                    session_data: None,
                                    audio_data: None,
                                    stage: None,
                                })
                            }
                        };
                    }
                }
            }
        }

        Json(TheViperRoomServerResponse {
            message: "Этот функционал пока не реализован, зацени лучше пока доступные тебе опции"
                .to_string(),
            buttons: vec!["Get news!".to_string(), "Schedule newsblock".to_string()],
            action_buttons: vec!["Sign out".to_string()],
            can_input: false,
            session_data: None,
            stage: None,
            audio_data: None,
        })
    } else {
        let reset_message =
            reset_user_state_with_message(the_viper_room_app_state.clone(), user_id).await;

        Json(TheViperRoomServerResponse {
            message: format!(
                "Invalid request: no action or action_step provided{}",
                reset_message
            ),
            buttons: vec![],
            action_buttons: vec![],
            can_input: false,
            session_data: None,
            stage: None,
            audio_data: None,
        })
    }
}
