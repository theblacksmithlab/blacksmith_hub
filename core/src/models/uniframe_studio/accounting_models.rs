use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

#[derive(sqlx::FromRow, Serialize, Deserialize)]
pub struct UserBalance {
    pub user_id: String,
    pub balance_usd: f64,
    pub active_dubbing_jobs: i32,
    pub active_lipsync_jobs: i32,
    pub max_concurrent_dubbing_jobs: i32,
    pub max_concurrent_lipsync_jobs: i32,
    pub updated_at: String,
}

impl UserBalance {
    pub async fn get_or_create(pool: &SqlitePool, user_id: &str) -> Result<Self, sqlx::Error> {
        match sqlx::query_as::<_, UserBalance>(
            "SELECT user_id, balance_usd, active_dubbing_jobs, active_lipsync_jobs, max_concurrent_dubbing_jobs, max_concurrent_lipsync_jobs, updated_at
         FROM user_balances WHERE user_id = ?",
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?
        {
            Some(balance) => Ok(balance),
            None => {
                sqlx::query(
                    "INSERT INTO user_balances (user_id, balance_usd, active_dubbing_jobs, active_lipsync_jobs, max_concurrent_dubbing_jobs, max_concurrent_lipsync_jobs)
                 VALUES (?, 100.0, 0, 0, 1, 1)"
                )
                    .bind(user_id)
                    .execute(pool)
                    .await?;

                sqlx::query_as::<_, UserBalance>(
                    "SELECT user_id, balance_usd, active_dubbing_jobs, active_lipsync_jobs, max_concurrent_dubbing_jobs, max_concurrent_lipsync_jobs, updated_at
                 FROM user_balances WHERE user_id = ?"
                )
                    .bind(user_id)
                    .fetch_one(pool)
                    .await
            }
        }
    }

    pub fn can_start_job(&self, job_type: ProcessingType) -> bool {
        match job_type {
            ProcessingType::Dubbing => self.active_dubbing_jobs == 0,
            ProcessingType::LipSync => self.active_lipsync_jobs == 0,
        }
    }

    pub fn has_sufficient_balance(&self, required_amount: f64) -> bool {
        self.balance_usd >= required_amount
    }

    pub async fn charge_and_reserve_job_slot(
        &mut self,
        pool: &SqlitePool,
        amount: f64,
        job_type: ProcessingType,
        description: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut tx = pool.begin().await?;

        // Charge from balance
        sqlx::query(
            "UPDATE user_balances 
             SET balance_usd = balance_usd - ?, updated_at = datetime('now') 
             WHERE user_id = ?",
        )
        .bind(amount)
        .bind(&self.user_id)
        .execute(&mut *tx)
        .await?;

        // Увеличиваем счетчик активных задач
        let field = match job_type {
            ProcessingType::Dubbing => "active_dubbing_jobs",
            ProcessingType::LipSync => "active_lipsync_jobs",
        };

        sqlx::query(&format!(
            "UPDATE user_balances SET {} = {} + 1 WHERE user_id = ?",
            field, field
        ))
        .bind(&self.user_id)
        .execute(&mut *tx)
        .await?;

        // Создаем транзакцию
        sqlx::query(
            "INSERT INTO transactions (id, user_id, type, amount_usd, status, description) 
             VALUES (?, ?, 'charge', ?, 'completed', ?)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&self.user_id)
        .bind(amount)
        .bind(description)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        // Обновляем локальное состояние
        self.balance_usd -= amount;
        match job_type {
            ProcessingType::Dubbing => self.active_dubbing_jobs += 1,
            ProcessingType::LipSync => self.active_lipsync_jobs += 1,
        }

        Ok(())
    }

    pub async fn complete_job(
        &mut self,
        pool: &SqlitePool,
        job_type: ProcessingType,
    ) -> Result<(), sqlx::Error> {
        let field = match job_type {
            ProcessingType::Dubbing => "active_dubbing_jobs",
            ProcessingType::LipSync => "active_lipsync_jobs",
        };

        sqlx::query(&format!(
            "UPDATE user_balances SET {} = CASE WHEN {} > 0 THEN {} - 1 ELSE 0 END WHERE user_id = ?",
            field, field, field
        ))
            .bind(&self.user_id)
            .execute(pool)
            .await?;

        // Обновляем локальное состояние
        match job_type {
            ProcessingType::Dubbing => {
                if self.active_dubbing_jobs > 0 {
                    self.active_dubbing_jobs -= 1;
                }
            }
            ProcessingType::LipSync => {
                if self.active_lipsync_jobs > 0 {
                    self.active_lipsync_jobs -= 1;
                }
            }
        }

        Ok(())
    }

    pub async fn add_funds(
        &mut self,
        pool: &SqlitePool,
        amount: f64,
        description: &str,
    ) -> Result<(), sqlx::Error> {
        let mut tx = pool.begin().await?;

        // Пополняем баланс
        sqlx::query(
            "UPDATE user_balances 
             SET balance_usd = balance_usd + ?, updated_at = datetime('now') 
             WHERE user_id = ?",
        )
        .bind(amount)
        .bind(&self.user_id)
        .execute(&mut *tx)
        .await?;

        // Создаем транзакцию
        sqlx::query(
            "INSERT INTO transactions (id, user_id, type, amount_usd, status, description) 
             VALUES (?, ?, 'deposit', ?, 'completed', ?)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(&self.user_id)
        .bind(amount)
        .bind(description)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        // Обновляем локальное состояние
        self.balance_usd += amount;

        Ok(())
    }
}

#[derive(sqlx::FromRow, Serialize, Deserialize)]
pub struct Transaction {
    pub id: String,
    pub user_id: String,
    pub transaction_type: String,
    pub amount_usd: f64,
    pub status: String,
    pub description: Option<String>,
    pub heleket_invoice_id: Option<String>,
    pub crypto_tx_hash: Option<String>,
    pub created_at: String,
    pub completed_at: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum ProcessingType {
    Dubbing,
    LipSync,
}

impl ProcessingType {
    pub fn calculate_cost(&self, duration_seconds: f64) -> f64 {
        let duration_minutes = (duration_seconds / 60.0).ceil();
        match self {
            ProcessingType::Dubbing => 2.0 + (duration_minutes * 2.0), // $2 per start + $2/min
            ProcessingType::LipSync => 3.0 + (duration_minutes * 2.0), // $3 per start + $2/min
        }
    }
}
