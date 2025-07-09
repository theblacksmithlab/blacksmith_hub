use crate::state::tg_bot::app_state::BotAppState;
use crate::utils::tg_bot::groot_bot::groot_bot_utils::auto_delete_message;
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::{Message, Requester};
use teloxide::types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup};
use teloxide::Bot;
use tracing::info;
use crate::utils::tg_bot::groot_bot::subscription::check_chat_payment;

#[derive(Debug, Clone, PartialEq)]
pub enum SubscriptionState {
    None,
    AwaitingChatSelection,
    AwaitingPlanSelection,
    AwaitingPaymentConfirmation,
    ProcessingPayment,
}

#[derive(Debug, Clone)]
pub struct PaymentProcess {
    pub state: SubscriptionState,
    pub target_chat_id: Option<i64>,
    pub target_chat_username: Option<String>,
    pub selected_plan: Option<String>,
    pub payment_amount: Option<u32>,
    pub payment_id: Option<String>,
}

pub struct SubscriptionPlan {
    pub id: &'static str,
    pub name: &'static str,
    pub duration_days: u32,
    pub price_usd: u32,
    pub description: &'static str,
}

pub mod callback_data {
    pub const PAY_CONTINUE: &str = "pay_continue";
    pub const PAY_CANCEL: &str = "pay_cancel";
    pub const PLAN_MONTHLY: &str = "plan_monthly";
    pub const PLAN_YEARLY: &str = "plan_yearly";
    pub const PAYMENT_CONFIRM: &str = "payment_confirm";
    pub const PAYMENT_CANCEL: &str = "payment_cancel";
    pub const BACK_TO_PLANS: &str = "back_to_plans";
}

pub const SUBSCRIPTION_PLANS: [SubscriptionPlan; 2] = [
    SubscriptionPlan {
        id: "monthly",
        name: "Месячная подписка",
        duration_days: 30,
        price_usd: 15,
        description: "30 дней полной защиты от спама",
    },
    SubscriptionPlan {
        id: "yearly",
        name: "Годовая подписка",
        duration_days: 365,
        price_usd: 150,
        description: "365 дней + скидка 17% (2 мес. - FREE!)",
    },
];

pub async fn handle_subscription_command(
    bot: Bot,
    msg: Message,
    app_state: Arc<BotAppState>,
) -> Result<()> {
    let user_id = msg.from.as_ref().unwrap().id.0;
    info!("user_id: {}", user_id);
    let user = msg.from.as_ref().unwrap();
    info!("user: {:?}", user);

    if msg.chat.is_private() {
        let bot_msg = "Команда /subscription доступна только в публичных чатах, чтобы я понял, какой чат вы хотите защитить.\n\
        После вызова команды /subscription в чате, мы продолжим общение тут";

        bot.send_message(msg.chat.id, bot_msg).await?;

        return Ok(());
    }

    let user_username = match &user.username {
        Some(username) => username.clone(),
        None => {
            let bot_msg = "❌ У вас нет @username. Установите его в настройках Telegram, чтобы я мог написать вам в ЛС";

            let bot_system_message = bot.send_message(msg.chat.id, bot_msg).await?;

            auto_delete_message(
                bot.clone(),
                bot_system_message.chat.id,
                bot_system_message.id,
                Duration::from_secs(120),
            ).await;

            return Ok(());
        }
    };

    let target_chat_id = msg.chat.id.0;
    let target_chat_username = match msg.chat.username() {
        Some(username) => username.to_string(),
        None => {
            bot.send_message(msg.chat.id, "❌ Чат должен иметь @username").await?;
            return Ok(());
        }
    };

    if let Some(db_pool) = &app_state.db_pool {
        if check_chat_payment(db_pool, target_chat_id).await.unwrap_or(false) {
            bot.send_message(msg.chat.id,
                             &format!("ℹ️ Чат @{} уже имеет активную подписку!", target_chat_username))
                .await?;
            return Ok(());
        }
    }

    if let Some(payment_states_mutex) = &app_state.payment_states {
        let mut payment_states = payment_states_mutex.lock().await;

        payment_states.insert(
            user_id,
            PaymentProcess {
                state: SubscriptionState::AwaitingPlanSelection,
                target_chat_id: Some(target_chat_id),
                target_chat_username: Some(target_chat_username.clone()),
                selected_plan: None,
                payment_amount: None,
                payment_id: None,
            },
        );
    }

    let group_msg = format!("✅ @{}, проверьте ЛС для оплаты подписки", user_username);
    let bot_system_message = bot.send_message(msg.chat.id, group_msg).await?;

    auto_delete_message(
        bot.clone(),
        bot_system_message.chat.id,
        bot_system_message.id,
        Duration::from_secs(30),
    ).await;

    show_plan_selection(bot, ChatId(user_id as i64), &target_chat_username).await
}

pub async fn show_plan_selection(bot: Bot, user_chat_id: ChatId, chat_username: &str) -> Result<()> {
    let message_text = format!(
        "📋 **Выберите тарифный план**\n\n\
        🏠 **Чат:** @{}\n\n\
        💰 **Доступные планы:**",
        chat_username
    );

    let mut keyboard_rows = vec![];

    for plan in &SUBSCRIPTION_PLANS {
        let button_text = format!("{} - {}₽", plan.name, plan.price_usd);
        keyboard_rows.push(vec![InlineKeyboardButton::callback(
            button_text,
            format!("plan_{}", plan.id),
        )]);
    }


    keyboard_rows.push(vec![
        InlineKeyboardButton::callback("❌ Отмена", "pay_cancel"),
    ]);

    let keyboard = InlineKeyboardMarkup::new(keyboard_rows);

    bot.send_message(user_chat_id, message_text)
        // .parse_mode(teloxide::types::ParseMode::MarkdownV2)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

pub async fn show_payment_confirmation(
    bot: Bot,
    chat_id: ChatId,
    chat_username: &str,
    plan: &SubscriptionPlan,
) -> Result<()> {
    let message_text = format!(
        "💳 **Подтверждение заказа**\n\n\
        🎯 **Чат:** @{}\n\
        📋 **План:** {}\n\
        💰 **Сумма:** {}₽\n\
        ⏱️ **Период:** {} дней\n\n\
        📝 **Описание:** {}\n\n\
        ✅ **Подтверждаете оплату?**",
        chat_username, plan.name, plan.price_usd, plan.duration_days, plan.description
    );

    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            "✅ Да, оплатить",
            "payment_confirm",
        )],
        vec![
            InlineKeyboardButton::callback("⬅️ Назад к планам", "back_to_plans"),
            InlineKeyboardButton::callback("❌ Отмена", "pay_cancel"),
        ],
    ]);

    bot.send_message(chat_id, message_text)
        // .parse_mode(teloxide::types::ParseMode::MarkdownV2)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

pub async fn show_payment_processing(
    bot: Bot,
    chat_id: ChatId,
    target_chat_id: i64,
    target_chat_username: &str,
    selected_plan: &str,
    payment_amount: u32,
) -> Result<()> {
    let plan = get_plan_by_id(selected_plan).unwrap();

    let message_text = format!(
        "🔄 **Создание платежа\\.\\.\\.**\n\n\
    🎯 **Чат:** @{}\n\
    🎯 **Чат ID:** `{}`\n\
    📋 **План:** {}\n\
    💰 **Сумма:** {}₽\n\n\
    🚧 **DEMO MODE** \\- Интеграция с Heleket в разработке\n\n\
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
        // .parse_mode(teloxide::types::ParseMode::MarkdownV2)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

pub fn get_plan_by_id(plan_id: &str) -> Option<&'static SubscriptionPlan> {
    SUBSCRIPTION_PLANS.iter().find(|plan| plan.id == plan_id)
}
