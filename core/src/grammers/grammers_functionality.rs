use anyhow::Result;
use grammers_client::Config as g_Config;
use grammers_client::{Client as g_Client, SignInError};

use crate::models::the_viper_room::the_viper_room::AuthStage::AuthError;
use crate::models::the_viper_room::the_viper_room::{AuthStage, TheViperRoomServerResponse};
use crate::state::the_viper_room::app_state::TheViperRoomAppState;
use crate::state::the_viper_room::app_state_operation::reset_user_state_with_message;
use crate::utils::common::update_the_viper_room_user_state;
use axum::Json;
use grammers_session::Session;
use grammers_tl_types as tl;
use grammers_tl_types::enums::InputUser;
use grammers_tl_types::functions::messages::CreateChat;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;
use std::{env, fs};
use tracing::info;

pub async fn create_chat(requester_username: &str, recipient_user: &str) -> Result<()> {
    let session_file = "common_res/request_app/grammers_system_session/7543812650.session";
    let session_path = Path::new(&session_file);

    let api_id: i32 = env::var("TELEGRAM_API_ID")
        .expect("API_ID not set")
        .parse()
        .expect("API_ID must be a number");
    let api_hash = env::var("TELEGRAM_API_HASH").expect("API_HASH not set");

    let client = g_Client::connect(g_Config {
        session: Session::load_file_or_create(session_path)?,
        api_id,
        api_hash,
        params: Default::default(),
    })
    .await?;

    if !client.is_authorized().await? {
        info!("Achtung! G_Client is not authorized!");
    } else {
        info!("Client is ok!");
    }

    let user_1 = client.resolve_username(requester_username).await?;
    let user_2 = client.resolve_username(recipient_user).await?;

    let user_1_id = user_1.clone().unwrap().pack().id;
    let user_1_access_hash = user_1.clone().unwrap().pack().access_hash;

    let user_2_id = user_2.clone().unwrap().pack().id;
    let user_2_access_hash = user_2.clone().unwrap().pack().access_hash;

    let users = vec![
        InputUser::User(tl::types::InputUser {
            user_id: user_1_id,
            access_hash: user_1_access_hash.expect("REASON"),
        }),
        InputUser::User(tl::types::InputUser {
            user_id: user_2_id,
            access_hash: user_2_access_hash.expect("REASON"),
        }),
    ];

    let title = format!(
        "reQuest App chat: {} | {}",
        requester_username, recipient_user
    );

    let create_chat = CreateChat {
        users,
        title: title.to_string(),
        ttl_period: None,
    };

    match client.invoke(&create_chat).await {
        Ok(_response) => {
            info!("Chat {} successfully created!", title);
        }
        Err(e) => {
            info!("Error creating chat: {:?}", e);
        }
    }

    Ok(())
}

pub async fn initialize_grammers_client(session_data: Vec<u8>) -> Result<g_Client> {
    let api_id: i32 = env::var("TELEGRAM_API_ID")
        .expect("API_ID not set")
        .parse()
        .expect("API_ID must be a number");
    let api_hash = env::var("TELEGRAM_API_HASH").expect("API_HASH not set");

    let client = g_Client::connect(g_Config {
        session: Session::load(&session_data)?,
        api_id,
        api_hash,
        params: Default::default(),
    })
    .await?;

    if !client.is_authorized().await? {
        info!("Achtung! Failed to initialize G_Client with provided session!");
    } else {
        info!("G_Client initialized successfully");
    }

    Ok(client)
}

pub async fn session_file_creation(
    action: Option<String>,
    the_viper_room_app_state: Arc<TheViperRoomAppState>,
    user_id: u64,
    api_id: i32,
    api_hash: String,
    nickname: String,
) -> Json<TheViperRoomServerResponse> {
    let session_file_path = env::current_dir()
        .expect("Failed to get current directory")
        .join("common_res")
        .join("the_viper_room_auth_tmp_data")
        .join(format!("{}.session", user_id));

    let is_awaiting_phone_number = {
        let user_states = the_viper_room_app_state.user_state.lock().await;
        user_states
            .get(&user_id)
            .map(|state| state.awaiting_phone_number)
            .unwrap_or(false)
    };

    let is_awaiting_passcode = {
        let user_states = the_viper_room_app_state.user_state.lock().await;
        user_states
            .get(&user_id)
            .map(|state| state.awaiting_passcode)
            .unwrap_or(false)
    };

    let is_awaiting_2fa = {
        let user_states = the_viper_room_app_state.user_state.lock().await;
        user_states
            .get(&user_id)
            .map(|state| state.awaiting_2fa)
            .unwrap_or(false)
    };

    if is_awaiting_phone_number {
        info!("is_awaiting_phone_number: {}", is_awaiting_phone_number);
        if let Some(parent_dir) = session_file_path.parent() {
            fs::create_dir_all(parent_dir).expect("Failed to create directory for sessions");
        }

        let session = match Session::load_file_or_create(session_file_path) {
            Ok(session) => session,
            Err(e) => {
                info!("Failed to create user's session: {}", e);
                let reset_message =
                    reset_user_state_with_message(the_viper_room_app_state.clone(), user_id).await;
                return Json(TheViperRoomServerResponse {
                    message: format!(
                        "Failed to create new Telegram-client user's session{}",
                        reset_message
                    ),
                    buttons: vec![],
                    action_buttons: vec![],
                    can_input: false,
                    session_data: None,
                    stage: Some(AuthError),
                    audio_data: None,
                });
            }
        };

        let client = match g_Client::connect(g_Config {
            session,
            api_id,
            api_hash,
            params: Default::default(),
        })
        .await
        {
            Ok(client) => client,
            Err(e) => {
                info!(
                    "Failed to initialize Telegram client with new session file: {}",
                    e
                );
                let reset_message =
                    reset_user_state_with_message(the_viper_room_app_state.clone(), user_id).await;
                return Json(TheViperRoomServerResponse {
                    message: format!("Failed to initialize Telegram client{}", reset_message),
                    buttons: vec![],
                    action_buttons: vec![],
                    can_input: false,
                    session_data: None,
                    stage: Some(AuthError),
                    audio_data: None,
                });
            }
        };

        if let Some(phone) = action {
            match client.request_login_code(&phone).await {
                Ok(token) => {
                    update_the_viper_room_user_state(the_viper_room_app_state, user_id, |state| {
                        state.phone_number = Some(phone.to_string());
                        state.awaiting_phone_number = false;
                        state.awaiting_passcode = true;
                        state.token = Some(Arc::new(token));
                        state.client = Some(client);
                    })
                    .await;

                    info!("State change: awaiting_phone_number = false, awaiting_passcode = true, awaiting_2fa = false");

                    return Json(TheViperRoomServerResponse {
                        message:
                            "Введи одноразовый код авторизации, который Telegram отправил тебе"
                                .to_string(),
                        buttons: vec![],
                        action_buttons: vec![],
                        can_input: true,
                        session_data: None,
                        stage: Some(AuthStage::PasscodeCodeRequest),
                        audio_data: None,
                    });
                }
                Err(e) => {
                    info!("Failed to request login code: {}", e);
                    let reset_message =
                        reset_user_state_with_message(the_viper_room_app_state.clone(), user_id)
                            .await;
                    return Json(TheViperRoomServerResponse {
                        message: format!("Failed to request login code{}", reset_message),
                        buttons: vec![],
                        action_buttons: vec![],
                        can_input: false,
                        session_data: None,
                        stage: Some(AuthError),
                        audio_data: None,
                    });
                }
            }
        }
    } else if is_awaiting_passcode {
        info!("is_awaiting_passcode: {}", is_awaiting_passcode);

        if let Some(passcode) = action {
            update_the_viper_room_user_state(the_viper_room_app_state.clone(), user_id, |state| {
                state.passcode = Some(passcode.clone());
                state.awaiting_passcode = false;
            })
            .await;

            let (client, token) = {
                let user_states = the_viper_room_app_state.user_state.lock().await;
                if let Some(state) = user_states.get(&user_id) {
                    if let (Some(client), Some(token)) =
                        (state.client.as_ref(), state.token.as_ref())
                    {
                        (client.clone(), token.clone())
                    } else {
                        let reset_message = reset_user_state_with_message(
                            the_viper_room_app_state.clone(),
                            user_id,
                        )
                        .await;
                        return Json(TheViperRoomServerResponse {
                            message: format!(
                                "Missing Telegram Client or login token in the App State{}",
                                reset_message
                            ),
                            buttons: vec![],
                            action_buttons: vec![],
                            can_input: false,
                            session_data: None,
                            stage: Some(AuthError),
                            audio_data: None,
                        });
                    }
                } else {
                    let reset_message =
                        reset_user_state_with_message(the_viper_room_app_state.clone(), user_id)
                            .await;
                    return Json(TheViperRoomServerResponse {
                        message: format!("User state not found in the App State{}", reset_message),
                        buttons: vec![],
                        action_buttons: vec![],
                        can_input: false,
                        session_data: None,
                        stage: Some(AuthError),
                        audio_data: None,
                    });
                }
            };

            match client.sign_in(&token, &passcode).await {
                Ok(_) => {
                    // // Local copy of user's session (TURNED OFF!!!)
                    // let session_file = env::current_dir()
                    //     .expect("Failed to get current directory")
                    //     .join("common_res")
                    //     .join("the_viper_room_grammers_sessions")
                    //     .join(format!("{}.session", user_id));
                    //
                    // client
                    //     .session()
                    //     .save_to_file(&session_file)
                    //     .expect("Failed to save session file");

                    let session_data = client.session().save();

                    match save_user_id(user_id).await {
                        Ok(_) => {
                            info!("Authorized user's id saved for the system purposes");
                        }
                        Err(e) => {
                            info!(
                                "Error saving authorized user's id for the system purposes: {}",
                                e
                            );
                        }
                    }

                    update_the_viper_room_user_state(
                        the_viper_room_app_state.clone(),
                        user_id,
                        |state| {
                            state.unauthorized = false;
                            state.authorized = true;
                        },
                    )
                    .await;

                    if let Err(e) = fs::remove_file(&session_file_path) {
                        info!("Failed to remove session file: {}", e);
                    } else {
                        info!("Successfully removed session file for user: {}", user_id);
                    }

                    return Json(TheViperRoomServerResponse {
                        message: format!(
                            "{}, ты авторизован, зацени доступные тебе опции",
                            nickname
                        ),
                        buttons: vec!["Get news!".to_string(), "Schedule newsblock".to_string()],
                        action_buttons: vec!["Sign out".to_string()],
                        can_input: false,
                        session_data: Some(session_data),
                        stage: Some(AuthStage::AuthSuccess),
                        audio_data: None,
                    });
                }
                Err(SignInError::PasswordRequired(password_token)) => {
                    let hint = password_token.hint().unwrap_or("No hint available");

                    update_the_viper_room_user_state(
                        the_viper_room_app_state.clone(),
                        user_id,
                        |state| {
                            state.awaiting_2fa = true;
                            state.password_token = Some(password_token.clone());
                        },
                    )
                    .await;

                    info!("State change: awaiting_phone_number = false, awaiting_passcode = false, awaiting_2fa = true");

                    return Json(TheViperRoomServerResponse {
                        message: format!("У тебя настроена 2FA-авторизация.\nПодсказка: {}.\n\nВведи пароль от своего Telegram-аккаунта:", hint),
                        buttons: vec![],
                        action_buttons: vec![],
                        can_input: true,
                        session_data: None,
                        stage: Some(AuthStage::TwoFAPassRequest),
                        audio_data: None,
                    });
                }
                Err(e) => {
                    let reset_message =
                        reset_user_state_with_message(the_viper_room_app_state.clone(), user_id)
                            .await;
                    return Json(TheViperRoomServerResponse {
                        message: format!("Failed to sign in: {}{}", e, reset_message),
                        buttons: vec![],
                        action_buttons: vec![],
                        can_input: false,
                        session_data: None,
                        stage: Some(AuthError),
                        audio_data: None,
                    });
                }
            }
        }
    } else if is_awaiting_2fa {
        info!("is_awaiting_2fa: {}", is_awaiting_2fa);

        if let Some(password) = action {
            let (client, password_token) = {
                let user_states = the_viper_room_app_state.user_state.lock().await;
                if let Some(state) = user_states.get(&user_id) {
                    if let (Some(client), Some(token)) =
                        (state.client.as_ref(), state.password_token.as_ref())
                    {
                        (client.clone(), token.clone())
                    } else {
                        let reset_message = reset_user_state_with_message(
                            the_viper_room_app_state.clone(),
                            user_id,
                        )
                        .await;
                        return Json(TheViperRoomServerResponse {
                            message: format!(
                                "Missing Telegram client or password token in the App State{}",
                                reset_message
                            ),
                            buttons: vec![],
                            action_buttons: vec![],
                            can_input: false,
                            session_data: None,
                            stage: Some(AuthError),
                            audio_data: None,
                        });
                    }
                } else {
                    let reset_message =
                        reset_user_state_with_message(the_viper_room_app_state.clone(), user_id)
                            .await;
                    return Json(TheViperRoomServerResponse {
                        message: format!("User state not found in the App State{}", reset_message),
                        buttons: vec![],
                        action_buttons: vec![],
                        can_input: false,
                        session_data: None,
                        stage: Some(AuthError),
                        audio_data: None,
                    });
                }
            };

            match client.check_password(password_token, password).await {
                Ok(_) => {
                    let session_data = client.session().save();

                    match save_user_id(user_id).await {
                        Ok(_) => {
                            info!("Authorized user's id saved for the system purposes");
                        }
                        Err(e) => {
                            info!(
                                "Error saving authorized user's id for the system purposes: {}",
                                e
                            );
                        }
                    }

                    update_the_viper_room_user_state(
                        the_viper_room_app_state.clone(),
                        user_id,
                        |state| {
                            state.awaiting_2fa = false;
                            state.unauthorized = false;
                            state.authorized = true;
                        },
                    )
                    .await;

                    // // Local copy of user's session (TURNED OFF!!!)
                    // let session_file = env::current_dir()
                    //     .expect("Failed to get current directory")
                    //     .join("common_res")
                    //     .join("the_viper_room_grammers_sessions")
                    //     .join(format!("{}.session", user_id));
                    //
                    // client
                    //     .session()
                    //     .save_to_file(&session_file)
                    //     .expect("Failed to save session file");

                    if !client
                        .is_authorized()
                        .await
                        .expect("Something went wrong during client authorization")
                    {
                        info!("Achtung! G_Client is not authorized!");
                    } else {
                        info!("Client is ok!");
                    }

                    if let Err(e) = fs::remove_file(&session_file_path) {
                        info!("Failed to remove session file: {}", e);
                    } else {
                        info!("Successfully removed session file for user: {}", user_id);
                    }

                    return Json(TheViperRoomServerResponse {
                        message: format!(
                            "{}, ты авторизован, зацени доступные тебе опции",
                            nickname
                        ),
                        buttons: vec!["Get news!".to_string(), "Schedule newsblock".to_string()],
                        action_buttons: vec!["Sign out".to_string()],
                        can_input: false,
                        session_data: Some(session_data),
                        stage: Some(AuthStage::AuthSuccess),
                        audio_data: None,
                    });
                }
                Err(e) => {
                    let reset_message =
                        reset_user_state_with_message(the_viper_room_app_state.clone(), user_id)
                            .await;
                    return Json(TheViperRoomServerResponse {
                        message: format!(
                            "Failed to authorize user with 2FA: {}{}",
                            e, reset_message
                        ),
                        buttons: vec![],
                        action_buttons: vec![],
                        can_input: false,
                        session_data: None,
                        stage: Some(AuthError),
                        audio_data: None,
                    });
                }
            }
        }
    }
    let reset_message =
        reset_user_state_with_message(the_viper_room_app_state.clone(), user_id).await;
    Json(TheViperRoomServerResponse {
        message: format!(
            "Global error. Invalid state or missing data{}",
            reset_message
        ),
        buttons: vec![],
        action_buttons: vec![],
        can_input: false,
        session_data: None,
        stage: Some(AuthError),
        audio_data: None,
    })
}

pub(crate) async fn save_user_id(user_id: u64) -> Result<()> {
    let users_count_dir = "common_res/the_viper_room/users_count";
    let users_count_file = format!("{}/users_count.txt", users_count_dir);

    fs::create_dir_all(users_count_dir).expect("Failed to create directory structure");

    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&users_count_file)
        .expect("Failed to open users count file")
        .write_all(format!("{}\n", user_id).as_bytes())
        .expect("Failed to write user_id to file");

    Ok(())
}
