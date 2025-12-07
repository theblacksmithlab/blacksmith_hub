use crate::local_db::telegram::groot_bot::subscription_management::has_active_subscription_for_other_chats;
use crate::state::tg_bot::GrootBotState;
use crate::utils::uniframe_studio::heleket_client::InvoiceResult;
use anyhow::Result;
use chrono::{DateTime, Utc};
use std::sync::Arc;
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
    pub original_price: Option<u32>,
    pub discount_percent: Option<u32>,
    pub final_price: Option<u32>,
    pub discount_reason: Option<String>,
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
        price_usd: 12,
        description: "30 дней ультимативной защиты от нечисти 🛡",
    },
    SubscriptionPlan {
        id: "yearly",
        name: "Годовая подписка",
        duration_days: 365,
        price_usd: 120,
        description: "365 дней 👉 2 месяца бесплатно 🎁)",
    },
];

pub async fn show_plan_selection(
    bot: Bot,
    user_chat_id: ChatId,
    chat_username: &str,
    target_chat_title: &str,
    user_id: i64,
    target_chat_id: i64,
    app_state: Arc<GrootBotState>,
) -> Result<()> {
    let has_discount_for_other_chats = if let Some(db_pool) = &app_state.core.db_pool {
        has_active_subscription_for_other_chats(db_pool, user_id, target_chat_id)
            .await
            .unwrap_or(false)
    } else {
        false
    };

    let discount_info = if has_discount_for_other_chats {
        "\n🎉 Ваша скидка: 30% (активная подписка на другой чат)\n"
    } else {
        "\n💡 Доступные скидки:\n• Годовая подписка: скидка 17%\n• При наличии подписки на другой чат: скидка 30%\n"
    };

    let message_text = format!(
        "Приветствую!\n\
        Я получил заявку на оплату подписки для чата: {} (@{})\n\
        {}\
        Внимательно проверьте username чата, изменить его после оплаты подписки будет невозможно!\n\n\
        Выберите тарифный план:",
        target_chat_title, chat_username, discount_info
    );

    let mut keyboard_rows = vec![];

    for plan in &SUBSCRIPTION_PLANS {
        let button_text = if has_discount_for_other_chats {
            let discounted_price = (plan.price_usd as f64 * 0.7).round() as u32;
            format!(
                "{} - {}$ → {}$ (-30%)",
                plan.name, plan.price_usd, discounted_price
            )
        } else if plan.id == "yearly" {
            let discounted_price = (plan.price_usd as f64 * 0.83).round() as u32;
            format!(
                "{} - {}$ → {}$ (-17%)",
                plan.name, plan.price_usd, discounted_price
            )
        } else {
            format!("{} - {}$", plan.name, plan.price_usd)
        };

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
    original_price: u32,
    discount_percent: u32,
    final_price: u32,
    discount_reason: &str,
) -> Result<()> {
    let price_text = if discount_percent > 0 {
        let discount_description = match discount_reason {
            "existing_subscription" => "скидка за активную подписку",
            "yearly_plan" => "скидка за годовую подписку",
            _ => "скидка",
        };

        format!(
            "💰 Сумма: ~~{} $~~ → {} $ ({} {}%)",
            original_price, final_price, discount_description, discount_percent
        )
    } else {
        format!("💰 Сумма: {} $", final_price)
    };

    let message_text = format!(
        "Подтверждение заказа\n\n\
        🎯 Чат: {} (@{})\n\
        📋 Тарифный план: {}\n\
        {}\n\
        ⏱️ Период: {} дней\n\n\
        📝 Описание: {}\n\n\
        ✅ Подтверждаете оплату?",
        target_chat_title,
        target_chat_username,
        plan.name,
        price_text,
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
