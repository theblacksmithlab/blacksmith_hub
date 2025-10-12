use anyhow::Result;
use core::local_db::tg_bot::groot_bot::subscription_management::{
    create_subscription, has_active_subscription_for_other_chats,
};
use core::models::common::system_messages::{AppsSystemMessages, GrootBotMessages};
use core::state::tg_bot::app_state::BotAppState;
use core::utils::common::get_message;
use core::utils::tg_bot::groot_bot::subscription_utils::get_plan_by_id;
use core::utils::tg_bot::groot_bot::subscription_utils::{
    show_payment_confirmation, show_payment_link, show_plan_selection, SubscriptionState,
};
use std::sync::Arc;
use teloxide::prelude::*;
use teloxide::types::CallbackQuery;
use tracing::{error, info};

pub async fn groot_bot_callback_query_handler(
    bot: Bot,
    callback_query: CallbackQuery,
    app_state: Arc<BotAppState>,
) -> Result<()> {
    let user_id = callback_query.from.id.0;
    let callback_data = callback_query
        .data
        .as_ref()
        .unwrap_or(&String::new())
        .clone();

    info!("Callback from user {}: {}", user_id, callback_data);

    match callback_data.as_str() {
        "pay_cancel" => handle_pay_cancel(bot, callback_query, app_state).await?,
        "back_to_plans" => handle_back_to_plans(bot, callback_query, app_state).await?,
        "payment_confirm" => handle_payment_confirm(bot, callback_query, app_state).await?,
        "check_payment" => handle_check_payment(bot, callback_query, app_state).await?,

        data if data.starts_with("plan_") => {
            let plan_id = &data[5..];
            handle_plan_selection(bot, callback_query, app_state, plan_id).await?
        }

        _ => {
            error!("Unknown callback data: {}", callback_data);
            bot.answer_callback_query(callback_query.id)
                .text("❌ Неизвестная команда")
                .await?;
        }
    }

    Ok(())
}

async fn handle_pay_cancel(
    bot: Bot,
    callback_query: CallbackQuery,
    app_state: Arc<BotAppState>,
) -> Result<()> {
    let user_id = callback_query.from.id.0;
    let chat_id = callback_query.message.as_ref().unwrap().chat().id;

    if let Some(payment_states_mutex) = &app_state.payment_states {
        let mut payment_states = payment_states_mutex.lock().await;
        payment_states.remove(&user_id);
    }

    bot.answer_callback_query(callback_query.id)
        .text("❌ Процесс оплаты отменен")
        .await?;

    if let Some(message) = callback_query.message {
        bot.delete_message(chat_id, message.id()).await?;
    }

    bot.send_message(
        chat_id,
        "❌ Процесс оплаты отменен.\n\
        Для повторной попытки используйте /subscription (в публичном чате)",
    )
    .await?;

    Ok(())
}

async fn handle_plan_selection(
    bot: Bot,
    callback_query: CallbackQuery,
    app_state: Arc<BotAppState>,
    plan_id: &str,
) -> Result<()> {
    let user_id = callback_query.from.id.0;
    let chat_id = callback_query.message.as_ref().unwrap().chat().id;

    let plan = match get_plan_by_id(plan_id) {
        Some(plan) => plan,
        None => {
            bot.answer_callback_query(callback_query.id)
                .text("❌ Неизвестный план")
                .await?;
            return Ok(());
        }
    };

    let (target_chat_title, target_chat_username, target_chat_id) = {
        if let Some(payment_states_mutex) = &app_state.payment_states {
            let payment_states = payment_states_mutex.lock().await;
            if let Some(payment_process) = payment_states.get(&user_id) {
                (
                    payment_process
                        .target_chat_title
                        .clone()
                        .unwrap_or_default(),
                    payment_process
                        .target_chat_username
                        .clone()
                        .unwrap_or_default(),
                    payment_process.target_chat_id.unwrap_or(0),
                )
            } else {
                return handle_expired_session(bot, callback_query).await;
            }
        } else {
            return Ok(());
        }
    };

    let original_price = plan.price_usd;
    let (discount_percent, discount_reason, final_price) = if let Some(db_pool) = &app_state.db_pool
    {
        match has_active_subscription_for_other_chats(db_pool, user_id as i64, target_chat_id).await
        {
            Ok(true) => {
                info!(
                    "User {} gets 30% discount for having active subscription on other chats (current chat: {})",
                    user_id, target_chat_id
                );
                (
                    30,
                    "existing_subscription".to_string(),
                    (original_price as f64 * 0.7).round() as u32,
                )
            }
            Ok(false) => {
                if plan_id == "yearly" {
                    info!(
                        "User {} gets 17% yearly discount for chat {}",
                        user_id, target_chat_id
                    );
                    (
                        17,
                        "yearly_plan".to_string(),
                        (original_price as f64 * 0.83).round() as u32,
                    )
                } else {
                    info!(
                        "User {} gets no discount for chat {}",
                        user_id, target_chat_id
                    );
                    (0, "none".to_string(), original_price)
                }
            }
            Err(e) => {
                error!(
                    "Error checking other subscriptions for user {}: {}",
                    user_id, e
                );
                if plan_id == "yearly" {
                    (
                        17,
                        "yearly_plan".to_string(),
                        (original_price as f64 * 0.83).round() as u32,
                    )
                } else {
                    (0, "none".to_string(), original_price)
                }
            }
        }
    } else {
        if plan_id == "yearly" {
            (
                17,
                "yearly_plan".to_string(),
                (original_price as f64 * 0.83).round() as u32,
            )
        } else {
            (0, "none".to_string(), original_price)
        }
    };

    {
        if let Some(payment_states_mutex) = &app_state.payment_states {
            let mut payment_states = payment_states_mutex.lock().await;
            if let Some(payment_process) = payment_states.get_mut(&user_id) {
                payment_process.state = SubscriptionState::AwaitingPaymentConfirmation;
                payment_process.selected_plan = Some(plan_id.to_string());
                payment_process.payment_amount = Some(final_price);
                payment_process.original_price = Some(original_price);
                payment_process.discount_percent = Some(discount_percent);
                payment_process.final_price = Some(final_price);
                payment_process.discount_reason = Some(discount_reason.clone());
            } else {
                return handle_expired_session(bot, callback_query).await;
            }
        }
    }

    let discount_info = if discount_percent > 0 {
        format!(" (скидка {}% 🎁)", discount_percent)
    } else {
        String::new()
    };

    bot.answer_callback_query(callback_query.id)
        .text(&format!(
            "✅ Выбран тарифный план: {}{}",
            plan.name, discount_info
        ))
        .await?;

    if let Some(message) = callback_query.message {
        bot.delete_message(chat_id, message.id()).await?;
    }

    show_payment_confirmation(
        bot,
        chat_id,
        &target_chat_username,
        &target_chat_title,
        plan,
        original_price,
        discount_percent,
        final_price,
        &discount_reason,
    )
    .await
}

async fn handle_back_to_plans(
    bot: Bot,
    callback_query: CallbackQuery,
    app_state: Arc<BotAppState>,
) -> Result<()> {
    let user_id = callback_query.from.id.0;
    let chat_id = callback_query.message.as_ref().unwrap().chat().id;

    let (chat_username, target_chat_title, target_chat_id) = {
        if let Some(payment_states_mutex) = &app_state.payment_states {
            let mut payment_states = payment_states_mutex.lock().await;
            if let Some(payment_process) = payment_states.get_mut(&user_id) {
                payment_process.state = SubscriptionState::AwaitingPlanSelection;
                payment_process.selected_plan = None;
                payment_process.payment_amount = None;
                payment_process.original_price = None;
                payment_process.discount_percent = None;
                payment_process.final_price = None;
                payment_process.discount_reason = None;

                (
                    payment_process
                        .target_chat_username
                        .clone()
                        .unwrap_or_default(),
                    payment_process
                        .target_chat_title
                        .clone()
                        .unwrap_or_default(),
                    payment_process.target_chat_id.unwrap_or(0),
                )
            } else {
                return handle_expired_session(bot, callback_query).await;
            }
        } else {
            return Ok(());
        }
    };

    bot.answer_callback_query(callback_query.id)
        .text("⬅️ Возврат к выбору тарифного плана")
        .await?;

    if let Some(message) = callback_query.message {
        bot.delete_message(chat_id, message.id()).await?;
    }

    show_plan_selection(
        bot,
        chat_id,
        &chat_username,
        &target_chat_title,
        user_id as i64,
        target_chat_id,
        app_state,
    )
    .await
}

async fn handle_expired_session(bot: Bot, callback_query: CallbackQuery) -> Result<()> {
    bot.answer_callback_query(callback_query.id)
        .text(
            "⏰ Сессия истекла. Для повторной попытки используйте /subscription (в публичном чате)",
        )
        .await?;

    let chat_id = callback_query.message.as_ref().unwrap().chat().id;

    if let Some(message) = callback_query.message {
        bot.delete_message(chat_id, message.id()).await?;
    }

    bot.send_message(
        chat_id,
        "⏰ Сессия оплаты истекла. Для повторной попытки используйте /subscription (в публичном чате)",
    )
    .await?;

    Ok(())
}

async fn handle_payment_confirm(
    bot: Bot,
    callback_query: CallbackQuery,
    app_state: Arc<BotAppState>,
) -> Result<()> {
    let user_id = callback_query.from.id.0;
    let chat_id = callback_query.message.as_ref().unwrap().chat().id;

    let (target_chat_username, final_price, target_chat_title) = {
        if let Some(payment_states_mutex) = &app_state.payment_states {
            let mut payment_states = payment_states_mutex.lock().await;
            if let Some(payment_process) = payment_states.get_mut(&user_id) {
                payment_process.state = SubscriptionState::ProcessingPayment;

                (
                    payment_process.target_chat_username.clone(),
                    payment_process.final_price.clone(),
                    payment_process.target_chat_title.clone(),
                )
            } else {
                return handle_expired_session(bot, callback_query).await;
            }
        } else {
            return Ok(());
        }
    };

    let target_chat_username =
        target_chat_username.ok_or_else(|| anyhow::anyhow!("Missing target_chat_username"))?;

    let payment_amount = final_price.ok_or_else(|| anyhow::anyhow!("Missing final_price"))?;

    let target_chat_title =
        target_chat_title.ok_or_else(|| anyhow::anyhow!("Missing target_chat_username"))?;

    let heleket_client = app_state.heleket_client.as_ref().unwrap();
    let invoice = match heleket_client
        .create_invoice(payment_amount as f64, &user_id.to_string())
        .await
    {
        Ok(invoice) => invoice,
        Err(e) => {
            error!("Failed to create Heleket invoice: {}", e);
            bot.answer_callback_query(callback_query.id)
                .text("❌ Ошибка создания платежа")
                .await?;
            return Ok(());
        }
    };

    if let Some(payment_states_mutex) = &app_state.payment_states {
        let mut payment_states = payment_states_mutex.lock().await;
        if let Some(payment_process) = payment_states.get_mut(&user_id) {
            payment_process.heleket_order_id = Some(invoice.order_id.clone());
            payment_process.heleket_invoice_uuid = Some(invoice.uuid.clone());
        }
    }

    bot.answer_callback_query(callback_query.id)
        .text("🔄 Создаю инвойс...")
        .await?;

    if let Some(message) = callback_query.message {
        bot.delete_message(chat_id, message.id()).await?;
    }

    show_payment_link(
        bot,
        chat_id,
        &invoice,
        &target_chat_username,
        &target_chat_title,
        payment_amount,
    )
    .await?;

    Ok(())
}

async fn handle_check_payment(
    bot: Bot,
    callback_query: CallbackQuery,
    app_state: Arc<BotAppState>,
) -> Result<()> {
    let user_id = callback_query.from.id.0;
    let chat_id = callback_query.message.as_ref().unwrap().chat().id;

    let (invoice_uuid, payment_process) = {
        if let Some(payment_states_mutex) = &app_state.payment_states {
            let payment_states = payment_states_mutex.lock().await;
            if let Some(payment_process) = payment_states.get(&user_id) {
                (
                    payment_process.heleket_invoice_uuid.clone(),
                    payment_process.clone(),
                )
            } else {
                return handle_expired_session(bot, callback_query).await;
            }
        } else {
            return Ok(());
        }
    };

    let invoice_uuid = match invoice_uuid {
        Some(uuid) => uuid,
        None => {
            bot.answer_callback_query(callback_query.id)
                .text("❌ Ошибка: нет данных о платеже")
                .await?;
            return Ok(());
        }
    };

    let heleket_client = app_state.heleket_client.as_ref().unwrap();
    let status = match heleket_client.check_invoice_status(&invoice_uuid).await {
        Ok(status) => status,
        Err(e) => {
            error!("Failed to check payment status: {}", e);
            bot.answer_callback_query(callback_query.id)
                .text("❌ Ошибка проверки платежа")
                .await?;
            return Ok(());
        }
    };

    if status.payment_status == "paid" {
        if let Some(db_pool) = &app_state.db_pool {
            let plan_type = match payment_process.selected_plan.as_ref().unwrap().as_str() {
                "monthly" => "monthly",
                "yearly" => "yearly",
                _ => {
                    error!(
                        "Unknown plan type: {}",
                        payment_process.selected_plan.as_ref().unwrap()
                    );
                    "monthly"
                }
            };

            let user_username = callback_query.from.username.as_deref();

            // Tracing only block
            if let (
                Some(original_price),
                Some(final_price),
                Some(discount_percent),
                Some(discount_reason),
            ) = (
                payment_process.original_price,
                payment_process.final_price,
                payment_process.discount_percent,
                payment_process.discount_reason.as_ref(),
            ) {
                if discount_percent > 0 {
                    info!(
                        "User {} applied {}% discount ({}) for chat {}: {} $ -> {} $ (saved {} $)",
                        user_id,
                        discount_percent,
                        discount_reason,
                        payment_process.target_chat_id.unwrap(),
                        original_price,
                        final_price,
                        original_price - final_price
                    );
                } else {
                    info!(
                        "User {} paid full price for chat {}: {} $",
                        user_id,
                        payment_process.target_chat_id.unwrap(),
                        final_price
                    );
                }
            }

            create_subscription(
                db_pool,
                payment_process.target_chat_id.unwrap(),
                &payment_process.target_chat_username.clone().unwrap(),
                user_id as i64,
                user_username,
                plan_type,
            )
            .await?;
        }

        if let Some(payment_states_mutex) = &app_state.payment_states {
            let mut payment_states = payment_states_mutex.lock().await;
            payment_states.remove(&user_id);
        }

        if let Some(message) = callback_query.message {
            bot.delete_message(chat_id, message.id()).await?;
        }

        let plan = get_plan_by_id(&payment_process.selected_plan.unwrap()).unwrap();
        let success_msg = format!(
            "✅ Оплата прошла успешно!\n\n\
            🎯 Чат: {} (@{})\n\
            📋 Тарифный план: {}\n\
            💰 Сумма {} $\n\
            ⏱️ Период: {} дней",
            payment_process.target_chat_title.unwrap(),
            payment_process.target_chat_username.clone().unwrap(),
            plan.name,
            payment_process.payment_amount.unwrap(),
            plan.duration_days
        );

        bot.send_message(chat_id, success_msg).await?;

        bot.answer_callback_query(callback_query.id)
            .text("✅ Подписка активирована!")
            .await?;

        let important_instructions = get_message(AppsSystemMessages::GrootBot(
            GrootBotMessages::ImportantInstructions,
        ))
        .await?;

        bot.send_message(chat_id, important_instructions).await?;

        if let Some(chat_stats_mutex) = &app_state.chat_message_stats {
            let mut chat_stats = chat_stats_mutex.lock().await;
            if let Err(err) = chat_stats
                .fetch_chat_history_for_new_chat(
                    &app_state.app_name,
                    ChatId(payment_process.target_chat_id.unwrap()),
                    &payment_process.target_chat_username.clone().unwrap(),
                )
                .await
            {
                error!(
                    "Error fetching chat history for a new chat: {} with id: {}: {}",
                    payment_process.target_chat_username.as_ref().unwrap(),
                    payment_process.target_chat_id.unwrap(),
                    err
                );
            }
        }
    } else {
        bot.answer_callback_query(callback_query.id)
            .text("💰 Оплата еще не поступила. Попробуйте проверить статус чуть позже.")
            .await?;
    }

    Ok(())
}
