//! Database service for PostgreSQL operations

use crate::{config::DatabaseConfig, error::AppError, Result};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use std::time::Duration;
use tracing::info;

#[derive(Debug, Clone)]
pub struct DatabaseService {
    pool: PgPool,
}

impl DatabaseService {
    pub async fn new(config: &DatabaseConfig) -> Result<Self> {
        info!("Initializing database connection pool");

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .acquire_timeout(Duration::from_secs(config.acquire_timeout_seconds))
            .idle_timeout(Duration::from_secs(config.idle_timeout_seconds))
            .max_lifetime(Duration::from_secs(config.max_lifetime_seconds))
            .connect(&config.url)
            .await
            .map_err(|e| {
                AppError::configuration(format!("Failed to connect to database: {}", e))
            })?;

        // Test the connection
        sqlx::query("SELECT 1")
            .fetch_one(&pool)
            .await
            .map_err(|e| AppError::database_connection(format!("Database health check failed: {}", e)))?;

        info!("Database connection pool initialized successfully");

        Ok(Self { pool })
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Check if a user exists by their identifier and source
    pub async fn check_user_registration(
        &self,
        user_id: &str,
        source: &str,
    ) -> Result<Option<(String, String, Option<String>, i32)>> {
        let query = match source {
            "telegram" => {
                "SELECT ws_id, email, telegram_id, id FROM public.dim_users WHERE telegram_id = $1 LIMIT 1"
            }
            "whatsapp" => {
                "SELECT ws_id, email, telegram_id, id FROM public.dim_users WHERE ws_id = $1 LIMIT 1"
            }
            "email" => {
                "SELECT ws_id, email, telegram_id, id FROM public.dim_users WHERE email = $1 LIMIT 1"
            }
            _ => return Err(AppError::validation("Invalid source")),
        };

        let result = sqlx::query(query)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some(row) = result {
            Ok(Some((
                row.try_get::<Option<String>, _>("ws_id")?.unwrap_or_default(),
                row.try_get("email")?,
                row.try_get("telegram_id")?,
                row.try_get("id")?,
            )))
        } else {
            Ok(None)
        }
    }

    /// Register a new user
    pub async fn register_user(
        &self,
        user_id: &str,
        source: &str,
        email: &str,
        password: Option<&str>,
    ) -> Result<String> {
        let mut tx = self.pool.begin().await?;

        let id_column = match source {
            "telegram" => "telegram_id",
            "whatsapp" => "ws_id",
            "email" => "email",
            _ => return Err(AppError::validation("Invalid source")),
        };

        // Check if user already exists
        let check_query = format!("SELECT 1 FROM public.dim_users WHERE {} = $1", id_column);
        let existing = sqlx::query(&check_query)
            .bind(user_id)
            .fetch_optional(&mut *tx)
            .await?;

        if existing.is_some() {
            return Err(AppError::conflict("User already registered"));
        }

        // Simple insert query based on source
        let new_user_id: i32 = match id_column {
            "telegram_id" => {
                if let Some(pwd) = password {
                    let hashed = bcrypt::hash(pwd, bcrypt::DEFAULT_COST)
                        .map_err(|e| AppError::internal(format!("Password hashing failed: {}", e)))?;
                    sqlx::query(
                        "INSERT INTO public.dim_users (telegram_id, email_registration_date, email, password) 
                         VALUES ($1, $2, $3, $4) RETURNING id"
                    )
                    .bind(user_id)
                    .bind(chrono::Utc::now())
                    .bind(email)
                    .bind(hashed)
                    .fetch_one(&mut *tx)
                    .await?
                    .try_get("id")?
                } else {
                    sqlx::query(
                        "INSERT INTO public.dim_users (telegram_id, email_registration_date, email) 
                         VALUES ($1, $2, $3) RETURNING id"
                    )
                    .bind(user_id)
                    .bind(chrono::Utc::now())
                    .bind(email)
                    .fetch_one(&mut *tx)
                    .await?
                    .try_get("id")?
                }
            },
            "ws_id" => {
                if let Some(pwd) = password {
                    let hashed = bcrypt::hash(pwd, bcrypt::DEFAULT_COST)
                        .map_err(|e| AppError::internal(format!("Password hashing failed: {}", e)))?;
                    sqlx::query(
                        "INSERT INTO public.dim_users (ws_id, email_registration_date, email, password) 
                         VALUES ($1, $2, $3, $4) RETURNING id"
                    )
                    .bind(user_id)
                    .bind(chrono::Utc::now())
                    .bind(email)
                    .bind(hashed)
                    .fetch_one(&mut *tx)
                    .await?
                    .try_get("id")?
                } else {
                    sqlx::query(
                        "INSERT INTO public.dim_users (ws_id, email_registration_date, email) 
                         VALUES ($1, $2, $3) RETURNING id"
                    )
                    .bind(user_id)
                    .bind(chrono::Utc::now())
                    .bind(email)
                    .fetch_one(&mut *tx)
                    .await?
                    .try_get("id")?
                }
            },
            _ => {
                return Err(AppError::validation(format!("Invalid id_column: {}", id_column)));
            }
        };

        tx.commit().await?;

        Ok(format!("User registered successfully with ID: {}", new_user_id))
    }

    /// Update user email
    pub async fn update_user_email(
        &self,
        user_id: &str,
        source: &str,
        email: &str,
    ) -> Result<String> {
        let mut tx = self.pool.begin().await?;

        let id_column = match source {
            "telegram" => "telegram_id",
            "whatsapp" => "ws_id",
            _ => return Err(AppError::validation("Invalid source for email update")),
        };

        // Check if email already exists
        let existing_email = sqlx::query("SELECT id, telegram_id, ws_id FROM public.dim_users WHERE email = $1")
            .bind(email)
            .fetch_optional(&mut *tx)
            .await?;

        if let Some(row) = existing_email {
            // Email exists, update the record with new telegram_id/ws_id if not set
            let _db_user_id: i32 = row.try_get("id")?;
            let existing_telegram: Option<String> = row.try_get("telegram_id")?;
            let existing_ws: Option<String> = row.try_get("ws_id")?;

            let should_update = match source {
                "telegram" => existing_telegram.is_none(),
                "whatsapp" => existing_ws.is_none(),
                _ => false,
            };

            if should_update {
                let update_query = format!(
                    "UPDATE public.dim_users SET {} = $1, email_registration_date = $2 WHERE email = $3",
                    id_column
                );
                sqlx::query(&update_query)
                    .bind(user_id)
                    .bind(chrono::Utc::now())
                    .bind(email)
                    .execute(&mut *tx)
                    .await?;

                tx.commit().await?;
                Ok("User association updated successfully".to_string())
            } else {
                tx.rollback().await?;
                Ok("User already associated with this email".to_string())
            }
        } else {
            // Email doesn't exist, check if user exists and update their email
            let user_query = format!("SELECT id FROM public.dim_users WHERE {} = $1", id_column);
            let user_result = sqlx::query(&user_query)
                .bind(user_id)
                .fetch_optional(&mut *tx)
                .await?;

            if let Some(_row) = user_result {
                let update_query = format!(
                    "UPDATE public.dim_users SET email = $1, email_registration_date = $2 WHERE {} = $3",
                    id_column
                );
                sqlx::query(&update_query)
                    .bind(email)
                    .bind(chrono::Utc::now())
                    .bind(user_id)
                    .execute(&mut *tx)
                    .await?;

                tx.commit().await?;
                Ok("User email updated successfully".to_string())
            } else {
                // Neither email nor user exists, create new record
                let insert_query = format!(
                    "INSERT INTO public.dim_users ({}, email, email_registration_date) VALUES ($1, $2, $3) RETURNING id",
                    id_column
                );
                let new_user_id: i32 = sqlx::query(&insert_query)
                    .bind(user_id)
                    .bind(email)
                    .bind(chrono::Utc::now())
                    .fetch_one(&mut *tx)
                    .await?
                    .try_get("id")?;

                tx.commit().await?;
                Ok(format!("New user record created with ID: {}", new_user_id))
            }
        }
    }

    /// Check if CUFE exists
    pub async fn check_cufe_exists(&self, cufe: &str) -> Result<bool> {
        let result = sqlx::query("SELECT EXISTS (SELECT 1 FROM public.invoice_header WHERE cufe = $1)")
            .bind(cufe)
            .fetch_one(&self.pool)
            .await?;

        Ok(result.try_get::<bool, _>(0)?)
    }

    /// Insert records into a table
    pub async fn insert_records(
        &self,
        records: &[serde_json::Value],
        table: &str,
    ) -> Result<bool> {
        if records.is_empty() {
            return Ok(false);
        }

        let mut tx = self.pool.begin().await?;

        for record in records {
            let obj = record.as_object()
                .ok_or_else(|| AppError::validation("Record must be a JSON object"))?;

            let columns: Vec<String> = obj.keys().map(|k| format!("\"{}\"", k)).collect();
            let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("${}", i)).collect();

            let query = format!(
                "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT DO NOTHING",
                table,
                columns.join(", "),
                placeholders.join(", ")
            );

            let mut sql_query = sqlx::query(&query);
            for (_, value) in obj {
                sql_query = sql_query.bind(value);
            }

            sql_query.execute(&mut *tx).await?;
        }

        tx.commit().await?;
        Ok(true)
    }

    /// Execute a custom query - simplified version
    pub async fn execute_simple_query(&self, query: &str) -> Result<Vec<sqlx::postgres::PgRow>> {
        let results = sqlx::query(query).fetch_all(&self.pool).await?;
        Ok(results)
    }

    /// Get user summary
    pub async fn get_user_summary(&self, email: &str) -> Result<Option<sqlx::postgres::PgRow>> {
        let result = sqlx::query("SELECT * FROM public.vw_usr_general_metrics WHERE user_email = $1")
            .bind(email)
            .fetch_optional(&self.pool)
            .await?;

        Ok(result)
    }

    /// Health check
    pub async fn health_check(&self) -> Result<()> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(())
    }

    /// Close the connection pool
    pub async fn close(&self) {
        self.pool.close().await;
        info!("Database connection pool closed");
    }
}

// Helper trait for better error handling
impl AppError {
    pub fn database_connection(message: String) -> Self {
        Self::Configuration { message }
    }
}