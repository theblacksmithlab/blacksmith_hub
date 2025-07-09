use teloxide::prelude::*;
use teloxide::types::CallbackQuery;
use anyhow::Result;
use std::sync::Arc;
use teloxide_core::types::{InlineKeyboardButton, InlineKeyboardMarkup};
use tracing::{info, error};
use core::state::tg_bot::app_state::BotAppState;
use core::utils::tg_bot::groot_bot::subscription_payment::{show_payment_confirmation, SubscriptionState, show_chat_selection_message, show_plan_selection, SUBSCRIPTION_PLANS, SubscriptionPlan};
use crate::groot_bot::subscription::check_chat_payment;

pub async fn groot_bot_callback_query_handler(
    bot: Bot,
    callback_query: CallbackQuery,
    app_state: Arc<BotAppState>,
) -> Result<()> {
    let user_id = callback_query.from.id.0;
    let callback_data = callback_query.data.as_ref().unwrap_or(&String::new()).clone();

    info!("Callback from user {}: {}", user_id, callback_data);
    
    match callback_data.as_str() {
        "pay_continue" => handle_pay_continue(bot, callback_query, app_state).await?,
        "pay_cancel" => handle_pay_cancel(bot, callback_query, app_state).await?,
        "back_to_chat_selection" => handle_back_to_chat_selection(bot, callback_query, app_state).await?,
        "back_to_plans" => handle_back_to_plans(bot, callback_query, app_state).await?,
        "payment_confirm" => handle_payment_confirm(bot, callback_query, app_state).await?,
        
        data if data.starts_with("plan_") => {
            let plan_id = &data[5..]; // Убираем "plan_"
            handle_plan_selection(bot, callback_query, app_state, plan_id).await?
        },
        
        _ => {
            error!("Unknown callback data: {}", callback_data);
            bot.answer_callback_query(callback_query.id)
                .text("❌ Неизвестная команда")
                .await?;
        }
    }

    Ok(())
}

async fn handle_pay_continue(
    bot: Bot,
    callback_query: CallbackQuery,
    app_state: Arc<BotAppState>,
) -> Result<()> {
    let user_id = callback_query.from.id.0;
    let chat_id = callback_query.message.as_ref().unwrap().chat().id;
    
    if let Some(payment_states_mutex) = &app_state.payment_states {
        let mut payment_states = payment_states_mutex.lock().await;
        if let Some(payment_process) = payment_states.get_mut(&user_id) {
            payment_process.state = SubscriptionState::AwaitingChatSelection;
        }
    }
    
    bot.answer_callback_query(callback_query.id)
        .text("✅ Переходим к выбору чата")
        .await?;
    
    if let Some(message) = callback_query.message {
        bot.delete_message(chat_id, message.id()).await?;
    }

    show_chat_selection_message(bot, chat_id).await
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

    bot.send_message(chat_id, "❌ Процесс оплаты отменен. Для повторной попытки используйте /subscription")
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

    let chat_username = {
        if let Some(payment_states_mutex) = &app_state.payment_states {
            let mut payment_states = payment_states_mutex.lock().await;
            if let Some(payment_process) = payment_states.get_mut(&user_id) {
                payment_process.state = SubscriptionState::AwaitingPaymentConfirmation;
                payment_process.selected_plan = Some(plan_id.to_string());
                payment_process.payment_amount = Some(plan.price_rub);

                payment_process.target_chat_username.clone().unwrap_or_default()
            } else {
                return handle_expired_session(bot, callback_query).await;
            }
        } else {
            return Ok(());
        }
    };
    
    bot.answer_callback_query(callback_query.id)
        .text(&format!("✅ Выбран план: {}", plan.name))
        .await?;
    
    if let Some(message) = callback_query.message {
        bot.delete_message(chat_id, message.id()).await?;
    }
    
    show_payment_confirmation(bot, chat_id, &chat_username, plan).await
}

async fn handle_back_to_plans(
    bot: Bot,
    callback_query: CallbackQuery,
    app_state: Arc<BotAppState>,
) -> Result<()> {
    let user_id = callback_query.from.id.0;
    let chat_id = callback_query.message.as_ref().unwrap().chat().id;
    
    let chat_username = {
        if let Some(payment_states_mutex) = &app_state.payment_states {
            let mut payment_states = payment_states_mutex.lock().await;
            if let Some(payment_process) = payment_states.get_mut(&user_id) {
                payment_process.state = SubscriptionState::AwaitingPlanSelection;
                payment_process.selected_plan = None;
                payment_process.payment_amount = None;
                payment_process.target_chat_username.clone().unwrap_or_default()
            } else {
                return handle_expired_session(bot, callback_query).await;
            }
        } else {
            return Ok(());
        }
    };
    
    bot.answer_callback_query(callback_query.id)
        .text("⬅️ Возврат к выбору планов")
        .await?;
    
    if let Some(message) = callback_query.message {
        bot.delete_message(chat_id, message.id()).await?;
    }
    
    show_plan_selection(bot, chat_id, &chat_username).await
}


async fn handle_expired_session(bot: Bot, callback_query: CallbackQuery) -> Result<()> {
    bot.answer_callback_query(callback_query.id)
        .text("⏰ Сессия истекла. Начните заново с /subscription")
        .await?;

    let chat_id = callback_query.message.as_ref().unwrap().chat().id;

    if let Some(message) = callback_query.message {
        bot.delete_message(chat_id, message.id()).await?;
    }

    bot.send_message(chat_id, "⏰ Сессия оплаты истекла. Для новой попытки используйте /subscription")
        .await?;

    Ok(())
}

pub async fn handle_forwarded_message(
    bot: Bot,
    msg: Message,
    app_state: Arc<BotAppState>,
) -> Result<()> {
    let user_id = msg.from.as_ref().unwrap().id.0;

    if msg.forward_origin().is_none() {
        bot.send_message(msg.chat.id,
                         "❌ Нужно **переслать** сообщение из чата, а не написать новое.\n\n\
            Найдите любое сообщение в чате и нажмите \"Переслать\"")
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await?;
        return Ok(());
    }

    let (target_chat_id, target_chat_username) = match &msg.forward_origin() {
        Some(teloxide::types::MessageOrigin::Channel { chat, .. }) => {
            (chat.id.0, chat.username().map(|u| u.to_string()))
        },
        Some(teloxide::types::MessageOrigin::Chat { sender_chat, .. }) => {
            (sender_chat.id.0, sender_chat.username().map(|u| u.to_string()))
        },
        _ => {
            bot.send_message(msg.chat.id,
                             "❌ Можно пересылать сообщения только из публичных чатов или каналов")
                .await?;
            return Ok(());
        }
    };

    let chat_username = match target_chat_username {
        Some(username) => username,
        None => {
            bot.send_message(msg.chat.id,
                             "❌ Чат должен иметь публичный @username для работы бота.\n\n\
                Попросите администратора чата установить username в настройках.")
                .parse_mode(teloxide::types::ParseMode::MarkdownV2)
                .await?;
            return Ok(());
        }
    };

    if let Some(db_pool) = &app_state.db_pool {
        if check_chat_payment(db_pool, target_chat_id).await.unwrap_or(false) {
            bot.send_message(msg.chat.id,
                             &format!("ℹ️ Чат @{} уже имеет активную подписку!", chat_username))
                .await?;
            return Ok(());
        }
    }

    if let Some(payment_states_mutex) = &app_state.payment_states {
        let mut payment_states = payment_states_mutex.lock().await;
        if let Some(payment_process) = payment_states.get_mut(&user_id) {
            payment_process.state = SubscriptionState::AwaitingPlanSelection;
            payment_process.target_chat_id = Some(target_chat_id);
            payment_process.target_chat_username = Some(chat_username.clone());
        } else {
            bot.send_message(msg.chat.id,
                             "⏰ Сессия истекла. Начните заново с /subscription")
                .await?;
            return Ok(());
        }
    }
    
    bot.send_message(msg.chat.id,
                     &format!("✅ Выбран чат: @{}", chat_username))
        .await?;

    show_plan_selection(bot, msg.chat.id, &chat_username).await
}

pub fn get_plan_by_id(plan_id: &str) -> Option<&'static SubscriptionPlan> {
    SUBSCRIPTION_PLANS.iter().find(|plan| plan.id == plan_id)
}

async fn handle_payment_confirm(
    bot: Bot,
    callback_query: CallbackQuery,
    app_state: Arc<BotAppState>,
) -> Result<()> {
    let user_id = callback_query.from.id.0;
    let chat_id = callback_query.message.as_ref().unwrap().chat().id;
    
    let (target_chat_id, target_chat_username, selected_plan, payment_amount) = {
        if let Some(payment_states_mutex) = &app_state.payment_states {
            let mut payment_states = payment_states_mutex.lock().await;
            if let Some(payment_process) = payment_states.get_mut(&user_id) {
                payment_process.state = SubscriptionState::ProcessingPayment;

                (
                    payment_process.target_chat_id.clone(),
                    payment_process.target_chat_username.clone(),
                    payment_process.selected_plan.clone(),
                    payment_process.payment_amount.clone(),
                )
            } else {
                return handle_expired_session(bot, callback_query).await;
            }
        } else {
            return Ok(());
        }
    };
    
    let (target_chat_id, target_chat_username, selected_plan, payment_amount) = match (
        target_chat_id, target_chat_username, selected_plan, payment_amount
    ) {
        (Some(chat_id), Some(username), Some(plan), Some(amount)) => (chat_id, username, plan, amount),
        _ => {
            bot.answer_callback_query(callback_query.id)
                .text("❌ Ошибка данных платежа")
                .await?;
            return Ok(());
        }
    };
    
    bot.answer_callback_query(callback_query.id)
        .text("🔄 Создаем платеж...")
        .await?;
    
    if let Some(message) = callback_query.message {
        bot.delete_message(chat_id, message.id()).await?;
    }

    // ТУТ БУДЕТ ИНТЕГРАЦИЯ С HELEKET
    // Пока что показываем заглушку
    show_payment_processing(bot, chat_id, target_chat_id, &target_chat_username, &selected_plan, payment_amount).await?;

    // TODO: После интеграции с Heleket:
    // 1. Создать платеж в Heleket
    // 2. Получить ссылку на оплату
    // 3. Отправить ссылку пользователю
    // 4. Дождаться webhook'а об оплате
    // 5. Создать подписку в БД
    // 6. Уведомить пользователя об успехе

    Ok(())
}

async fn show_payment_processing(
    bot: Bot,
    chat_id: ChatId,
    target_chat_id: i64,
    target_chat_username: &str,
    selected_plan: &str,
    payment_amount: u32,
) -> Result<()> {
    let plan = get_plan_by_id(selected_plan).unwrap();

    let message_text = format!(
        "🔄 **Создание платежа...**\n\n\
        🎯 **Чат:** @{}\n\
        🎯 **Чат ID:** @{}\n\
        📋 **План:** {}\n\
        💰 **Сумма:** {}₽\n\n\
        🚧 **DEMO MODE** - Интеграция с Heleket в разработке\n\n\
        ℹ️ После интеграции здесь будет:\n\
        • Ссылка на оплату криптовалютой\n\
        • Автоматическая активация подписки\n\
        • Уведомления в чат",
        target_chat_username,
        target_chat_id,
        plan.name,
        payment_amount
    );

    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback("🔄 Успешная оплату", "demo_payment_success")],
        vec![InlineKeyboardButton::callback("❌ Отмена", "pay_cancel")],
    ]);

    bot.send_message(chat_id, message_text)
        .parse_mode(teloxide::types::ParseMode::MarkdownV2)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

async fn handle_back_to_chat_selection(
    bot: Bot,
    callback_query: CallbackQuery,
    app_state: Arc<BotAppState>,
) -> Result<()> {
    let user_id = callback_query.from.id.0;
    let chat_id = callback_query.message.as_ref().unwrap().chat().id;
    
    if let Some(payment_states_mutex) = &app_state.payment_states {
        let mut payment_states = payment_states_mutex.lock().await;
        if let Some(payment_process) = payment_states.get_mut(&user_id) {
            payment_process.state = SubscriptionState::AwaitingChatSelection;
            payment_process.target_chat_id = None;
            payment_process.target_chat_username = None;
            payment_process.selected_plan = None;
            payment_process.payment_amount = None;
        } else {
            return handle_expired_session(bot, callback_query).await;
        }
    }
    
    bot.answer_callback_query(callback_query.id)
        .text("⬅️ Возврат к выбору чата")
        .await?;
    
    if let Some(message) = callback_query.message {
        bot.delete_message(chat_id, message.id()).await?;
    }
    
    show_chat_selection_message(bot, chat_id).await
}
