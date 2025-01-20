use crate::routes::request_app::utils::{
    display_user_request_server, search_by_users_request_server, send_user_profile_server,
    show_result_by_direction, MAIN_MENU_BUTTONS, PROFILE_MENU_BUTTONS, REQUEST_MENU_BUTTONS,
};
use core::ai::utils::{process_users_request, process_users_self_description};
use core::grammers::grammers_functionality::create_chat;
use core::state::request_app::app_state::RequestAppState;
use core::utils::common::{determine_user_request, update_user_state};

use core::models::request_app::request_app::{ServerResponse, UserAction};

use axum::{extract::State, Json};
use std::sync::Arc;
use teloxide::prelude::ChatId;
use tracing::info;

pub(crate) async fn handle_user_action(
    State(app_state): State<Arc<RequestAppState>>,
    Json(command): Json<UserAction>,
) -> Json<ServerResponse> {
    let user_id = ChatId(command.user_id);
    let action = command.action;
    let username = command.username.to_string();

    info!(
        "Fn handle_user_action | Got action from user {} ({}): {}",
        username, command.user_id, action
    );

    let current_state = {
        let user_states = app_state.user_states.lock().await;
        user_states.get(&user_id).cloned()
    };

    if action == "Mini-app initialized" && username == "Unknown User" {
        info!("Mommy's anon ({}) tried hard to start the App", user_id);

        return Json(ServerResponse {
            message: "Извините, но для использования приложения необходимо установить username в Telegram.\nПожалуйста, установите username в настройках что бы получить доступ к приложению".to_string(),
            buttons: vec![],
            action_buttons: vec![],
            can_input: false,
        });
    }

    if action == "Mini-app initialized" {
        info!("Mini-app initialized by user: {} ({})!", username, user_id);
        update_user_state(app_state, user_id, |state| {
            state.start_window = true;
            state.main_menu = false;
            state.profile_menu = false;
            state.checking_profile = false;
            state.editing_profile = false;
            state.creating_profile = false;
            state.creating_profile_process = false;
            state.request_menu = false;
            state.checking_request = false;
            state.editing_request = false;
            state.editing_request_process = false;
            state.creating_request = false;
            state.creating_request_process = false;
            state.request_actuality_menu = false;
            state.request_actuality = false;
            state.request_search_result = false;
            state.request_search_result_exploring = false;
            state.current_result_index = None;
        })
        .await;

        return Json(ServerResponse {
            message: "Push the start button!".to_string(),
            buttons: vec!["Start".to_string()],
            action_buttons: vec![],
            can_input: false,
        });
    }

    if let Some(state) = current_state {
        if state.main_menu {
            return handle_main_menu(app_state.clone(), user_id, action).await;
        } else if state.profile_menu {
            return handle_state_profile_menu(app_state.clone(), user_id, action).await;
        } else if state.request_menu {
            return handle_state_request_menu(app_state.clone(), user_id, action).await;
        } else if state.creating_request {
            return handle_state_creating_request_menu(app_state.clone(), user_id, action).await;
        } else if state.creating_request_process {
            return handle_state_creating_request_process(
                app_state.clone(),
                user_id,
                action,
                username,
            )
            .await;
        } else if state.request_search_result {
            return handle_state_request_search_result(
                app_state.clone(),
                user_id,
                action,
                username,
            )
            .await;
        } else if state.editing_request {
            return handle_state_editing_request(app_state.clone(), user_id, action).await;
        } else if state.editing_request_process {
            return handle_state_editing_request_process(
                app_state.clone(),
                user_id,
                action,
                username,
            )
            .await;
        } else if state.start_window {
            return handle_start_window(app_state.clone(), user_id, action).await;
        } else if state.creating_profile {
            return handle_state_creating_profile_menu(app_state.clone(), user_id, action).await;
        } else if state.creating_profile_process {
            return handle_state_creating_profile_process(app_state.clone(), user_id, action).await;
        }
    }
    
    update_user_state(app_state, user_id, |state| {
        state.main_menu = true;
    })
    .await;
    Json(ServerResponse {
        message: "Unknown state or no action matched. Please contact dev team: @spacewhaleblues"
            .to_string(),
        buttons: vec![],
        action_buttons: vec![],
        can_input: false,
    })
}

pub(crate) async fn handle_start_window(
    app_state: Arc<RequestAppState>,
    user_id: ChatId,
    action: String,
) -> Json<ServerResponse> {
    match action.as_str() {
        "Start" => {
            update_user_state(app_state, user_id, |state| {
                state.start_window = false;
                state.main_menu = true;
            })
            .await;

            send_main_menu_response().await
        }
        _ => {
            let wrong_choice_message = "Извините, я вас не понял.\nПожалуйста, выберите один из доступных вариантов кнопками.".to_string();

            Json(ServerResponse {
                message: wrong_choice_message,
                buttons: vec!["Start".to_string()],
                action_buttons: vec![],
                can_input: false,
            })
        }
    }
}

pub(crate) async fn send_main_menu_response() -> Json<ServerResponse> {
    Json(ServerResponse {
        message: "Вы находитесь в главном меню.\nВыберите интересующий вас раздел:".to_string(),
        buttons: MAIN_MENU_BUTTONS.clone(),
        action_buttons: vec!["Выход".to_string()],
        can_input: false,
    })
}

pub(crate) async fn send_profile_menu_response() -> Json<ServerResponse> {
    Json(ServerResponse {
        message: "Вы находитесь в меню управления профилем.\nВыберите опцию:".to_string(),
        buttons: PROFILE_MENU_BUTTONS.clone(),
        action_buttons: vec!["Назад".to_string(), "Главное меню".to_string()],
        can_input: false,
    })
}

pub(crate) async fn send_request_menu_response() -> Json<ServerResponse> {
    Json(ServerResponse {
        message: "Вы находитесь в меню управления запросом.\nВыберите опцию:".to_string(),
        buttons: REQUEST_MENU_BUTTONS.clone(),
        action_buttons: vec!["Назад".to_string(), "Главное меню".to_string()],
        can_input: false,
    })
}

async fn handle_main_menu(
    app_state: Arc<RequestAppState>,
    user_id: ChatId,
    action: String,
) -> Json<ServerResponse> {
    match action.as_str() {
        "Управление профилем" => {
            update_user_state(app_state, user_id, |state| {
                state.main_menu = false;
                state.profile_menu = true;
            })
                .await;

            send_profile_menu_response().await
        }
        "Управление запросом" => {
            update_user_state(app_state, user_id, |state| {
                state.main_menu = false;
                state.request_menu = true;
            })
                .await;

            send_request_menu_response().await
        }
        "Выполнить поиск по запросу" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.main_menu = false;
                state.request_search_result = true;
            })
                .await;

            let search_result_response = search_by_users_request_server(user_id, app_state.clone()).await.unwrap();
            Json(ServerResponse {
                message: search_result_response.message,
                buttons: search_result_response.buttons,
                action_buttons: search_result_response.action_buttons,
                can_input: false,
            })
        }
        "Выход" => {
            update_user_state(app_state, user_id, |state| {
                state.main_menu = false;
                state.start_window = true;
            })
                .await;

            Json(ServerResponse {
                message: "Bye!\nYou are always welcome here".to_string(),
                buttons: vec!["Start".to_string()],
                action_buttons: vec![],
                can_input: false,
            })
        }
        _ => {
            Json(ServerResponse {
                message: "Извините, я вас не понял.\nПожалуйста, выберите один из доступных вариантов кнопками.".to_string(),
                buttons: MAIN_MENU_BUTTONS.clone(),
                action_buttons: vec!["Выход".to_string()],
                can_input: false,
            })
        }
    }
}

async fn handle_state_profile_menu(
    app_state: Arc<RequestAppState>,
    user_id: ChatId,
    action: String,
) -> Json<ServerResponse> {
    match action.as_str() {
        "Посмотреть профиль" => {
            let profile_check_response = send_user_profile_server(user_id, app_state.clone()).await.unwrap();
            Json(ServerResponse {
                message: profile_check_response.message,
                buttons: profile_check_response.buttons,
                action_buttons: profile_check_response.action_buttons,
                can_input: false,
            })
        }
        // TODO: Реализовать логику редактирования профиля
        "Изменить профиль" => {
            // update_user_state(app_state.clone(), user_id, |state| {
            //     state.profile_menu = false;
            //     state.editing_profile = true;
            // })
            //     .await;

            Json(ServerResponse {
                message: "Эта функция пока недоступна.\nВыберите другую опцию:".to_string(),
                buttons: vec![
                    "Посмотреть профиль".to_string(),
                    "Изменить профиль".to_string(),
                    "Создать профиль".to_string()
                ],
                action_buttons: vec![
                    "Назад".to_string(),
                    "Главное меню".to_string()
                ],
                can_input: false,
            })
        }
        "Создать профиль" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.profile_menu = false;
                state.creating_profile = true;
            })
                .await;

            Json(ServerResponse{
                message: "Следующим шагом вам нужно будет рассказать вкратце о себе, готовы?".to_string(),
                buttons: vec!["Поехали!".to_string()],
                action_buttons: vec![
                    "Назад".to_string(),
                    "Главное меню".to_string()
                ],
                can_input: false,
            })
        }
        "Назад" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.profile_menu = false;
                state.main_menu = true;
            })
                .await;

            send_main_menu_response().await
        }
        "Главное меню" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.profile_menu = false;
                state.main_menu = true;
            })
                .await;

            send_main_menu_response().await
        }
        _ => {
            Json(ServerResponse {
                message: "Извините, я вас не понял.\nПожалуйста, выберите один из доступных вариантов кнопками.".to_string(),
                buttons: PROFILE_MENU_BUTTONS.clone(),
                action_buttons: vec![
                    "Назад".to_string(),
                    "Главное меню".to_string()
                ],
                can_input: false,
            })
        }
    }
}

pub(crate) async fn handle_state_request_menu(
    app_state: Arc<RequestAppState>,
    user_id: ChatId,
    action: String,
) -> Json<ServerResponse> {
    match action.as_str() {
        "Посмотреть запрос" => {
            let display_result_response = display_user_request_server(user_id, app_state.clone()).await.unwrap();
            Json(ServerResponse {
                message: display_result_response.message,
                buttons: display_result_response.buttons,
                action_buttons: display_result_response.action_buttons,
                can_input: false,
            })
        }
        "Изменить запрос" => {
            let user_request = match determine_user_request(user_id, app_state.clone()).await.unwrap() {
                Some(request) => request,
                None => {
                    return Json(ServerResponse {
                        message: "У вас нет сохраненного запроса. Пожалуйста, создайте его командой 'Создать запрос'.".to_string(),
                        buttons: REQUEST_MENU_BUTTONS.clone(),
                        action_buttons: vec![
                            "Назад".to_string(),
                            "Главное меню".to_string()
                        ],
                        can_input: false,
                    });
                }
            };

            update_user_state(app_state.clone(), user_id, |state| {
                state.request_menu = false;
                state.editing_request = true;
            })
                .await;

            let message_for_response = format!("{}\n\nСледующим шагом вам нужно будет рассказать о том что или кого вы ищете, чтобы изменить запрос, готовы?", user_request);

            Json(ServerResponse {
                message: message_for_response,
                buttons: vec![
                    "Готов!".to_string()
                ],
                action_buttons: vec![
                    "Назад".to_string(),
                    "Главное меню".to_string()
                ],
                can_input: false,
            })
        }
        "Создать запрос" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.request_menu = false;
                state.creating_request = true;
            })
                .await;

            Json(ServerResponse {
                message: "Следующим шагом вам нужно будет рассказать о том что или кого вы ищете, готовы?".to_string(),
                buttons: vec!["Готов!".to_string()],
                action_buttons: vec![
                    "Назад".to_string(),
                    "Главное меню".to_string()
                ],
                can_input: false,
            })
        }
        // TODO: Реализовать логику управления актуальностью запроса
        "Меню актуальности запроса" => {
            Json(ServerResponse {
                message: "Эта функция пока недоступна, пожалуйста, воспользуйтесь другой опцией".to_string(),
                buttons: vec![
                    "Посмотреть запрос".to_string(),
                    "Изменить запрос".to_string(),
                    "Создать запрос".to_string(),
                    "Меню актуальности запроса".to_string()
                ],
                action_buttons: vec!["Назад".to_string(), "Главное меню".to_string()],
                can_input: false,
            })
        }
        "Назад" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.request_menu = false;
                state.main_menu = true;
            })
                .await;

            send_main_menu_response().await
        }
        "Главное меню" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.request_menu = false;
                state.main_menu = true;
            })
                .await;

            send_main_menu_response().await
        }
        _ => {
            Json(ServerResponse {
                message: "Извините, я вас не понял.\nПожалуйста, выберите один из доступных вариантов кнопками.".to_string(),
                buttons: REQUEST_MENU_BUTTONS.clone(),
                action_buttons: vec![
                    "Назад".to_string(),
                    "Главное меню".to_string()
                ],
                can_input: false,
            })
        }
    }
}

pub(crate) async fn handle_state_creating_profile_menu(
    app_state: Arc<RequestAppState>,
    user_id: ChatId,
    action: String,
) -> Json<ServerResponse> {
    match action.as_str() {
        "Поехали!" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.creating_profile = false;
                state.creating_profile_process = true;
            })
                .await;

            Json(ServerResponse {
                message: "Отлично! Слушаю...".to_string(),
                buttons: vec![],
                action_buttons: vec![
                    "Назад".to_string(),
                    "Главное меню".to_string()
                ],
                can_input: true,
            })
        }
        "Назад" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.creating_profile = false;
                state.profile_menu = true;
            })
                .await;

            send_profile_menu_response().await
        }
        "Главное меню" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.creating_profile = false;
                state.main_menu = true;
            })
                .await;

            send_main_menu_response().await
        }
        _ => {
            Json(ServerResponse {
                message: "Извините, я вас не понял.\nПожалуйста, выберите один из доступных вариантов кнопками.".to_string(),
                buttons: vec!["Поехали!".to_string()],
                action_buttons: vec![
                    "Назад".to_string(),
                    "Главное меню".to_string()
                ],
                can_input: false,
            })
        }
    }
}

pub(crate) async fn handle_state_creating_profile_process(
    app_state: Arc<RequestAppState>,
    user_id: ChatId,
    action: String,
) -> Json<ServerResponse> {
    match action.as_str() {
        "Назад" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.creating_request_process = false;
                state.creating_request = true;
            })
            .await;

            Json(ServerResponse {
                message: "Вам нужно рассказать вкратце о себе, готовы?".to_string(),
                buttons: vec!["Поехали!".to_string()],
                action_buttons: vec!["Назад".to_string(), "Главное меню".to_string()],
                can_input: false,
            })
        }
        "Главное меню" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.creating_request_process = false;
                state.main_menu = true;
            })
            .await;

            send_main_menu_response().await
        }
        _ => {
            process_users_self_description(user_id, action, app_state.clone())
                .await
                .unwrap();

            update_user_state(app_state.clone(), user_id, |state| {
                state.creating_request_process = false;
                state.profile_menu = true;
            })
            .await;

            Json(ServerResponse {
                message: "Отлично, сохранил!".to_string(),
                buttons: PROFILE_MENU_BUTTONS.clone(),
                action_buttons: vec!["Назад".to_string(), "Главное меню".to_string()],
                can_input: false,
            })
        }
    }
}

pub(crate) async fn handle_state_creating_request_menu(
    app_state: Arc<RequestAppState>,
    user_id: ChatId,
    action: String,
) -> Json<ServerResponse> {
    match action.as_str() {
        "Готов!" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.creating_request = false;
                state.creating_request_process = true;
            })
                .await;

            Json(ServerResponse {
                message: "Отлично! Слушаю...".to_string(),
                buttons: vec![],
                action_buttons: vec![
                    "Назад".to_string(),
                    "Главное меню".to_string()
                ],
                can_input: true,
            })
        }
        "Назад" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.creating_request = false;
                state.request_menu = true;
            })
                .await;

            send_request_menu_response().await
        }
        "Главное меню" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.creating_request = false;
                state.main_menu = true;
            })
                .await;

            send_main_menu_response().await
        }
        _ => {
            Json(ServerResponse {
                message: "Извините, я вас не понял.\nПожалуйста, выберите один из доступных вариантов кнопками.".to_string(),
                buttons: vec!["Готов!".to_string()],
                action_buttons: vec![
                    "Назад".to_string(),
                    "Главное меню".to_string()
                ],
                can_input: false,
            })
        }
    }
}

pub(crate) async fn handle_state_creating_request_process(
    app_state: Arc<RequestAppState>,
    user_id: ChatId,
    action: String,
    username: String,
) -> Json<ServerResponse> {
    match action.as_str() {
        "Назад" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.creating_request_process = false;
                state.creating_request = true;
            })
            .await;

            Json(ServerResponse {
                message: "Вам нужно рассказать о том, что или кого вы ищите, готовы?".to_string(),
                buttons: vec!["Готов!".to_string()],
                action_buttons: vec!["Назад".to_string(), "Главное меню".to_string()],
                can_input: false,
            })
        }
        "Главное меню" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.creating_request_process = false;
                state.main_menu = true;
            })
            .await;

            send_main_menu_response().await
        }
        _ => {
            if let Err(e) =
                process_users_request(username, user_id, action.to_string(), app_state.clone())
                    .await
            {
                tracing::error!("Failed to process user's request: {}", e);

                update_user_state(app_state.clone(), user_id, |state| {
                    state.creating_request_process = false;
                    state.main_menu = true;
                })
                .await;

                Json(ServerResponse {
                    message: "Произошла ошибка при сохранении запроса.\nПожалуйста, обратитесь к разработчикам приложения: @spacewhaleblues".to_string(),
                    buttons: MAIN_MENU_BUTTONS.clone(),
                    action_buttons: vec!["Выход".to_string()],
                    can_input: false,
                })
            } else {
                update_user_state(app_state.clone(), user_id, |state| {
                    state.creating_request_process = false;
                    state.request_menu = true;
                })
                .await;
                Json(ServerResponse {
                    message: "Отлично, сохранил!".to_string(),
                    buttons: REQUEST_MENU_BUTTONS.clone(),
                    action_buttons: vec!["Назад".to_string(), "Главное меню".to_string()],
                    can_input: false,
                })
            }
        }
    }
}

pub(crate) async fn handle_state_editing_request(
    app_state: Arc<RequestAppState>,
    user_id: ChatId,
    action: String,
) -> Json<ServerResponse> {
    match action.as_str() {
        "Готов!" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.editing_request = false;
                state.editing_request_process = true;
            })
                .await;

            Json(ServerResponse {
                message: "Отлично! Слушаю...".to_string(),
                buttons: vec![],
                action_buttons: vec![
                    "Назад".to_string(),
                    "Главное меню".to_string()
                ],
                can_input: true,
            })
        }
        "Назад" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.editing_request = false;
                state.request_menu = true;
            })
                .await;

            send_request_menu_response().await
        }
        "Главное меню" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.editing_request = false;
                state.main_menu = true;
            })
                .await;

            send_main_menu_response().await
        }
        _ => {
            Json(ServerResponse {
                message: "Извините, я вас не понял.\nПожалуйста, выберите один из доступных вариантов кнопками.".to_string(),
                buttons: vec!["Готов!".to_string()],
                action_buttons: vec![
                    "Назад".to_string(),
                    "Главное меню".to_string()
                ],
                can_input: false,
            })
        }
    }
}

pub(crate) async fn handle_state_editing_request_process(
    app_state: Arc<RequestAppState>,
    user_id: ChatId,
    action: String,
    username: String,
) -> Json<ServerResponse> {
    match action.as_str() {
        "Назад" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.editing_request_process = false;
                state.editing_request = true;
            })
            .await;

            Json(ServerResponse {
                message: "Вам нужно рассказать о том что или кого вы ищете, чтобы изменить запрос, готовы?".to_string(),
                buttons: vec!["Готов!".to_string()],
                action_buttons: vec![
                    "Назад".to_string(),
                    "Главное меню".to_string()
                ],
                can_input: true,
            })
        }
        "Главное меню" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.editing_request_process = false;
                state.main_menu = true;
            })
            .await;

            send_main_menu_response().await
        }
        _ => {
            if let Err(e) =
                process_users_request(username, user_id, action.to_string(), app_state.clone())
                    .await
            {
                tracing::error!("Failed to process user's request: {}", e);

                update_user_state(app_state.clone(), user_id, |state| {
                    state.creating_request_process = false;
                    state.main_menu = true;
                })
                .await;

                Json(ServerResponse {
                    message: "Произошла ошибка при обновлении запроса.\nПожалуйста, обратитесь к разработчикам приложения: @spacewhaleblues".to_string(),
                    buttons: MAIN_MENU_BUTTONS.clone(),
                    action_buttons: vec!["Выход".to_string()],
                    can_input: false,
                })
            } else {
                update_user_state(app_state.clone(), user_id, |state| {
                    state.creating_request_process = false;
                    state.main_menu = true;
                })
                .await;
                Json(ServerResponse {
                    message: "Отлично, отредактировал!".to_string(),
                    buttons: MAIN_MENU_BUTTONS.clone(),
                    action_buttons: vec!["Выход".to_string()],
                    can_input: false,
                })
            }
        }
    }
}

pub(crate) async fn handle_state_request_search_result(
    app_state: Arc<RequestAppState>,
    user_id: ChatId,
    action: String,
    username: String,
) -> Json<ServerResponse> {
    match action.as_str() {
        "Предыдущий результат" => {
            let direction = "previous".to_string();
            let next_result_option = show_result_by_direction(user_id, app_state.clone(), &direction).await.unwrap();
            Json(next_result_option)
        }
        "Contact!" => {
            let current_result_index = {
                let user_state = app_state.user_states.lock().await;
                user_state
                    .get(&user_id)
                    .and_then(|state| state.current_result_index)
            };

            info!("Current_result_index: {}", current_result_index.unwrap());

            if let Some(result_index) = current_result_index {
                let search_results_map = app_state.user_search_results.lock().await;
                if let Some(user_results) = search_results_map.get(&user_id) {
                    if let Some(result) = user_results.points.get(&result_index) {
                        let recipient_user = result
                            .payload
                            .get("username")
                            .and_then(|value| value.as_str()) // Getting rid of quotes
                            .cloned()
                            .unwrap_or_default();

                        let recipient_user_username = recipient_user.to_string();

                        info!("Trying to connect user: |{}| and the recipient_user: |{}|", username, recipient_user_username);

                        let _x = create_chat(&username, &recipient_user_username).await;

                        update_user_state(app_state.clone(), user_id, |state| {
                            state.request_search_result = false;
                            state.main_menu = true;
                        })
                            .await;

                        return Json(ServerResponse {
                            message: "Я создал чат между вами и автором найденного результата.\nFeel free to contact p2p.".to_string(),
                            buttons: MAIN_MENU_BUTTONS.clone(),
                            action_buttons: vec!["Выход".to_string()],
                            can_input: false,
                        });
                    }
                }
            }

            update_user_state(app_state.clone(), user_id, |state| {
                state.request_search_result = false;
                state.main_menu = true;
            })
                .await;

            Json(ServerResponse {
                message: "Автор найденного запроса не имеет username, с ним невозможно связаться, увы.".to_string(),
                buttons: MAIN_MENU_BUTTONS.clone(),
                action_buttons: vec!["Выход".to_string()],
                can_input: false,
            })
        }
        "Следующий результат" => {
            let direction = "next".to_string();
            let next_result_option = show_result_by_direction(user_id, app_state.clone(), &direction).await.unwrap();
            Json(next_result_option)
        }
        "Главное меню" => {
            update_user_state(app_state.clone(), user_id, |state| {
                state.request_search_result = false;
                state.main_menu = true;
            })
                .await;

            send_main_menu_response().await
        }
        _ => {
            Json(ServerResponse {
                message: "Извините, я вас не понял.\nПожалуйста, выберите один из доступных вариантов кнопками".to_string(),
                buttons: vec![
                    "Предыдущий результат".to_string(),
                    "Contact!".to_string(),
                    "Следующий результат".to_string()
                ],
                action_buttons: vec![
                    "Назад".to_string(),
                    "Главное меню".to_string()
                ],
                can_input: false,
            })
        }
    }
}
