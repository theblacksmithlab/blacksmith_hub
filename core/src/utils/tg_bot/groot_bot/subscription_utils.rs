use crate::utils::uniframe_studio::heleket_client::InvoiceResult;
use anyhow::Result;
use chrono::{DateTime, Utc};
use teloxide::payloads::SendMessageSetters;
use teloxide::prelude::Requester;
use teloxide::types::{ChatId, InlineKeyboardButton, InlineKeyboardMarkup};
use teloxide::Bot;
use url::Url;

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
    pub target_chat_title: Option<String>,
    pub selected_plan: Option<String>,
    pub payment_amount: Option<u32>,
    pub payment_id: Option<String>,
    pub heleket_invoice_uuid: Option<String>,
    pub heleket_order_id: Option<String>,
}

pub struct SubscriptionPlan {
    pub id: &'static str,
    pub name: &'static str,
    pub duration_days: u32,
    pub price_usd: u32,
    pub description: &'static str,
}

pub fn get_plan_by_id(plan_id: &str) -> Option<&'static SubscriptionPlan> {
    SUBSCRIPTION_PLANS.iter().find(|plan| plan.id == plan_id)
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
        description: "30 дней ультимативной защиты от нечисти",
    },
    SubscriptionPlan {
        id: "yearly",
        name: "Годовая подписка",
        duration_days: 365,
        price_usd: 150,
        description: "365 дней + скидка 17% 👉 2 месяца - В ПОДАРОК!)",
    },
];

pub async fn show_plan_selection(
    bot: Bot,
    user_chat_id: ChatId,
    chat_username: &str,
    target_chat_title: &str,
) -> Result<()> {
    let message_text = format!(
        "Приветствую ещё раз!\n\n\
        Я получил заявку на оплату подписки для чата: {} (@{})\n\
        Внимательно проверьте username чата, изменить его после оплаты подписки будет невозможно!\n\n\
        Выберите тарифный план:",
        target_chat_title,
        chat_username
    );

    let mut keyboard_rows = vec![];

    for plan in &SUBSCRIPTION_PLANS {
        let button_text = format!("{} - {} $", plan.name, plan.price_usd);
        keyboard_rows.push(vec![InlineKeyboardButton::callback(
            button_text,
            format!("plan_{}", plan.id),
        )]);
    }

    keyboard_rows.push(vec![InlineKeyboardButton::callback(
        "❌ Отмена",
        "pay_cancel",
    )]);

    let keyboard = InlineKeyboardMarkup::new(keyboard_rows);

    bot.send_message(user_chat_id, message_text)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

pub async fn show_payment_confirmation(
    bot: Bot,
    chat_id: ChatId,
    target_chat_username: &str,
    target_chat_title: &str,
    plan: &SubscriptionPlan,
) -> Result<()> {
    let message_text = format!(
        "Подтверждение заказа\n\n\
        🎯 Чат: {} (@{})\n\
        📋 Тарифный план: {}\n\
        💰 Сумма: {} $\n\
        ⏱️ Период: {} дней\n\n\
        📝 Описание: {}\n\n\
        ✅ Подтверждаете оплату?",
        target_chat_title,
        target_chat_username,
        plan.name,
        plan.price_usd,
        plan.duration_days,
        plan.description
    );

    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::callback(
            "✅ Да, перейти к оплате",
            "payment_confirm",
        )],
        vec![
            InlineKeyboardButton::callback("⬅️ Назад к тарифам", "back_to_plans"),
            InlineKeyboardButton::callback("❌ Отмена", "pay_cancel"),
        ],
    ]);

    bot.send_message(chat_id, message_text)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}

pub async fn show_payment_link(
    bot: Bot,
    chat_id: ChatId,
    invoice: &InvoiceResult,
    chat_username: &str,
    target_chat_title: &str,
    amount: u32,
) -> Result<()> {
    let expired_at = DateTime::from_timestamp(invoice.expired_at, 0)
        .unwrap_or_else(|| Utc::now() + chrono::Duration::hours(1))
        .with_timezone(&chrono::FixedOffset::east_opt(3 * 3600).unwrap())
        .format("%d.%m.%Y %H:%M UTC+3");

    let message_text = format!(
        "Оплата подписки\n\n\
        🎯 Чат: {} (@{})\n\
        💰 Сумма: {} $\n\
        🆔 Идентификатор заказа: `{}`\n\n\
        ⏰ Время на оплату: до {}\n\n\
        👇 Нажмите кнопку для оплаты:",
        target_chat_title, chat_username, amount, invoice.order_id, expired_at
    );

    let payment_url =
        Url::parse(&invoice.url).map_err(|e| anyhow::anyhow!("Invalid payment URL: {}", e))?;

    let keyboard = InlineKeyboardMarkup::new(vec![
        vec![InlineKeyboardButton::url("💳 Оплатить", payment_url)],
        vec![InlineKeyboardButton::callback(
            "🔄 Проверить оплату",
            "check_payment",
        )],
        vec![InlineKeyboardButton::callback("❌ Отмена", "pay_cancel")],
    ]);

    bot.send_message(chat_id, message_text)
        .reply_markup(keyboard)
        .await?;

    Ok(())
}
