use crate::routes::request_app::search_manager::activate_search_manager;
use anyhow::Result;
use core::local_db::local_db::get_user_profile_from_db;
use core::models::request_app::request_app::RequestAppServerResponse;
use core::state::request_app::app_state::RequestAppState;
use core::utils::common::{
    determine_user_request, extract_user_profile_from_app_state, format_user_profile,
    update_request_app_user_state,
};
use lazy_static::lazy_static;
use std::sync::Arc;
use teloxide::prelude::ChatId;
use tracing::info;

lazy_static! {
    pub static ref MAIN_MENU_BUTTONS: Vec<String> = vec![
        "Управление профилем".to_string(),
        "Управление запросом".to_string(),
        "Выполнить поиск по запросу".to_string(),
    ];
}

lazy_static! {
    pub static ref PROFILE_MENU_BUTTONS: Vec<String> = vec![
        "Посмотреть профиль".to_string(),
        "Изменить профиль".to_string(),
        "Создать профиль".to_string(),
    ];
}

lazy_static! {
    pub static ref REQUEST_MENU_BUTTONS: Vec<String> = vec![
        "Посмотреть запрос".to_string(),
        "Изменить запрос".to_string(),
        "Создать запрос".to_string(),
        "Меню актуальности запроса".to_string(),
    ];
}

pub(crate) async fn display_user_request_server(
    user_id: ChatId,
    app_state: Arc<RequestAppState>,
) -> Result<RequestAppServerResponse> {
    let user_request = match determine_user_request(user_id, app_state.clone()).await? {
        Some(request) => request,
        None => {
            return Ok(RequestAppServerResponse {
                message: "У вас нет сохраненного запроса. Пожалуйста, создайте его командой *'Создать запрос'*".to_string(),
                buttons: REQUEST_MENU_BUTTONS.clone(),
                action_buttons: vec![
                    "Назад".to_string(),
                    "Главное меню".to_string()
                ],
                can_input: false,
            });
        }
    };

    Ok(RequestAppServerResponse {
        message: format!("Ваш текущий запрос:\n>**{}**.", user_request),
        buttons: REQUEST_MENU_BUTTONS.clone(),
        action_buttons: vec!["Назад".to_string(), "Главное меню".to_string()],
        can_input: false,
    })
}

pub(crate) async fn search_by_users_request_server(
    user_id: ChatId,
    app_state: Arc<RequestAppState>,
) -> Result<RequestAppServerResponse, anyhow::Error> {
    let user_request = match determine_user_request(user_id, app_state.clone()).await? {
        Some(request) => request,
        None => {
            update_request_app_user_state(app_state.clone(), user_id, |state| {
                state.request_search_result = false;
                state.request_menu = true;
            })
            .await;
            return Ok(RequestAppServerResponse {
                message: "У вас нет ни одного сохраненного запроса. Пожалуйста, создайте его в меню управления запросом".to_string(),
                buttons: REQUEST_MENU_BUTTONS.clone(),
                action_buttons: vec![
                    "Назад".to_string(),
                    "Главное меню".to_string()
                ],
                can_input: false,
            });
        }
    };

    let search_manager_job_result =
        activate_search_manager(user_request, user_id, app_state.clone()).await?;

    info!(
        "Fn: search_by_users_request_server | Search manager job result: {}",
        search_manager_job_result
    );

    if search_manager_job_result == "No results found".to_string() {
        update_request_app_user_state(app_state.clone(), user_id, |state| {
            state.request_search_result = false;
            state.request_menu = true;
        })
        .await;

        return Ok(RequestAppServerResponse {
            message: "Извините, ничего подходящего в базе нет.\nПопробуйте изменить ваш запрос соответствующей командой".to_string(),
            buttons: REQUEST_MENU_BUTTONS.clone(),
            action_buttons: vec![
                "Назад".to_string(),
                "Главное меню".to_string()
            ],
            can_input: false,
        });
    } else if search_manager_job_result == "All's good" {
        let search_results_map = app_state.user_search_results.lock().await;

        if let Some(user_results) = search_results_map.get(&user_id) {
            if let Some(&first_result_index) = user_results.order.first() {
                info!(
                    "Fn: search_by_users_request_server | First result index: {}",
                    first_result_index
                );
                update_request_app_user_state(app_state.clone(), user_id, |state| {
                    state.current_result_index = Some(first_result_index);
                })
                .await;

                if let Some(first_result) = user_results.points.get(&first_result_index) {
                    let message_for_server_response = format!(
                        "Результаты поиска по вашему запросу:\n>**{}**.\n---\nВы можете посмотреть другие результаты или связаться с автором найденного результата, нажав кнопку *'Contact!'*",
                        first_result.payload.get("text").unwrap()
                    );

                    return Ok(RequestAppServerResponse {
                        message: message_for_server_response,
                        buttons: vec![
                            "Предыдущий результат".to_string(),
                            "Contact!".to_string(),
                            "Следующий результат".to_string(),
                        ],
                        action_buttons: vec!["Главное меню".to_string()],
                        can_input: false,
                    });
                }
            }
        }

        // Should never get here if the logic is correct;
        update_request_app_user_state(app_state.clone(), user_id, |state| {
            state.request_search_result = false;
            state.main_menu = true;
            state.current_result_index = None;
        })
        .await;

        return Ok(RequestAppServerResponse {
            message: "Ошибка: результат не найден. Попробуйте еще раз.\nОшибка: AppState doesn't contain search_results_map".to_string(),
            buttons: MAIN_MENU_BUTTONS.clone(),
            action_buttons: vec!["Выход".to_string()],
            can_input: false,
        });
    }
    // Should never get here if the logic is correct;
    Err(anyhow::anyhow!("Fn: search_by_users_request_server | Unexpected error. Search_manager returned unexpected data."))
}

pub(crate) async fn show_result_by_direction(
    user_id: ChatId,
    app_state: Arc<RequestAppState>,
    direction: &str,
) -> Result<RequestAppServerResponse, anyhow::Error> {
    let mut search_results_map = app_state.user_search_results.lock().await;

    if let Some(user_results) = search_results_map.get_mut(&user_id) {
        let mut user_state = app_state.user_states.lock().await;

        if let Some(state) = user_state.get_mut(&user_id) {
            if let Some(current_index) = state.current_result_index {
                if let Some(current_position) =
                    user_results.order.iter().position(|&x| x == current_index)
                {
                    let new_position = match direction {
                        "next" => {
                            if current_position + 1 < user_results.order.len() {
                                current_position + 1
                            } else {
                                return Ok(RequestAppServerResponse {
                                    message: "Это был последний результат в списке. Могу снова показать предыдущие варианты.".to_string(),
                                    buttons: vec!["Предыдущий результат".to_string(), "Contact!".to_string(), "Следующий результат".to_string()],
                                    action_buttons: vec!["Главное меню".to_string()],
                                    can_input: false,
                                });
                            }
                        }
                        "previous" => {
                            if current_position == 0 {
                                return Ok(RequestAppServerResponse {
                                    message: "Это первый результат в списке. Посмотрите следующие результаты".to_string(),
                                    buttons: vec!["Предыдущий результат".to_string(), "Contact!".to_string(), "Следующий результат".to_string()],
                                    action_buttons: vec!["Главное меню".to_string()],
                                    can_input: false,
                                });
                            }
                            current_position - 1
                        }
                        _ => {
                            // Should never get here if the logic is correct
                            return Err(anyhow::anyhow!("Invalid direction parameter."));
                        }
                    };

                    let new_result_index = user_results.order[new_position];
                    state.current_result_index = Some(new_result_index);

                    info!(
                        "NEW position: {} | Array index of NEW result: {}",
                        new_position, new_result_index
                    );

                    if let Some(new_result) = user_results.points.get(&new_result_index) {
                        let message = format!(
                            "{} результат:\n>**{}**.\n---\nНажмите *'Contact!'*, чтобы связаться с автором, или выберите *'Следующий результат'* для продолжения.",
                            if direction == "next" { "Следующий" } else { "Предыдущий" },
                            new_result.payload.get("text").unwrap()
                        );

                        return Ok(RequestAppServerResponse {
                            message,
                            buttons: vec![
                                "Предыдущий результат".to_string(),
                                "Contact!".to_string(),
                                "Следующий результат".to_string(),
                            ],
                            action_buttons: vec!["Главное меню".to_string()],
                            can_input: false,
                        });
                    }
                }
            }
        }
    }

    // Should never get here if the logic is correct
    Ok(RequestAppServerResponse {
        message: "Ошибка: Не удалось получить результат. Попробуйте еще раз.".to_string(),
        buttons: vec![
            "Предыдущий результат".to_string(),
            "Contact!".to_string(),
            "Следующий результат".to_string(),
        ],
        action_buttons: vec!["Главное меню".to_string()],
        can_input: false,
    })
}

pub(crate) async fn send_user_profile_server(
    chat_id: ChatId,
    app_state: Arc<RequestAppState>,
) -> Result<RequestAppServerResponse, anyhow::Error> {
    let user_profile = extract_user_profile_from_app_state(&app_state, chat_id).await;

    if let Some(profile) = user_profile {
        let profile_message = format_user_profile(&profile);
        let server_response_message = format!("Ваш профиль:\n```\n{}\n```", profile_message);
        info!("Server Response: {}", server_response_message);
        return Ok(RequestAppServerResponse {
            message: server_response_message,
            buttons: PROFILE_MENU_BUTTONS.clone(),
            action_buttons: vec!["Назад".to_string(), "Главное меню".to_string()],
            can_input: false,
        });
    }

    let pool = app_state.local_db_pool.lock().await;
    if let Some(pool) = pool.as_ref() {
        match get_user_profile_from_db(pool, chat_id).await {
            Ok(Some(profile)) => {
                let mut profiles = app_state.user_profile.lock().await;
                profiles.insert(chat_id, profile.clone());

                let profile_message = format_user_profile(&profile);
                let server_response_message =
                    format!("Ваш профиль:\n```\n{}\n```", profile_message);

                Ok(RequestAppServerResponse {
                    message: server_response_message,
                    buttons: PROFILE_MENU_BUTTONS.clone(),
                    action_buttons: vec!["Назад".to_string(), "Главное меню".to_string()],
                    can_input: false,
                })
            }
            Ok(None) => Ok(RequestAppServerResponse {
                message: "Я не смог найти ваш профиль. Создайте его командой *'Создать профиль'*"
                    .to_string(),
                buttons: PROFILE_MENU_BUTTONS.clone(),
                action_buttons: vec!["Назад".to_string(), "Главное меню".to_string()],
                can_input: false,
            }),
            Err(_) => Ok(RequestAppServerResponse {
                message: "Произошла ошибка при обращении к базе данных.".to_string(),
                buttons: PROFILE_MENU_BUTTONS.clone(),
                action_buttons: vec!["Назад".to_string(), "Главное меню".to_string()],
                can_input: false,
            }),
        }
    } else {
        Ok(RequestAppServerResponse {
            message: "База данных не инициализирована.".to_string(),
            buttons: PROFILE_MENU_BUTTONS.clone(),
            action_buttons: vec!["Назад".to_string(), "Главное меню".to_string()],
            can_input: false,
        })
    }
}
