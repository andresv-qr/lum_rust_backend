use sqlx::{PgPool, Row};
use chrono::{DateTime, Utc};
use crate::api::invoices::models::{LogStatus, ErrorType};
use crate::api::invoices::repository::{
    create_initial_log, update_log_success, update_log_duplicate, update_log_error
};
use crate::api::invoices::error_handling::InvoiceProcessingError;
use tracing::{info, warn, error};

// ============================================================================
// LOGGING SERVICE
// ============================================================================

pub struct LoggingService {
    pool: PgPool,
}

impl LoggingService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    
    /// Creates an initial log entry when processing starts
    pub async fn start_processing(
        &self,
        url: &str,
        origin: &str,
        user_id: &str,
        user_email: &str,
    ) -> Result<i32, InvoiceProcessingError> {
        info!("Starting log for URL: {} from origin: {}", url, origin);
        create_initial_log(&self.pool, url, origin, user_id, user_email).await
    }
    
    /// Updates log when processing completes successfully
    pub async fn log_success(
        &self,
        log_id: i32,
        cufe: &str,
        start_time: DateTime<Utc>,
        fields_extracted: u32,
        retry_attempts: u32,
    ) -> Result<(), InvoiceProcessingError> {
        let execution_time_ms = (Utc::now() - start_time).num_milliseconds() as i32;
        
        update_log_success(
            &self.pool,
            log_id,
            cufe,
            execution_time_ms,
            fields_extracted as i32,
            retry_attempts as i32,
        ).await?;
        
        info!(
            "Logged successful processing for CUFE: {}, execution time: {}ms, fields: {}, retries: {}",
            cufe, execution_time_ms, fields_extracted, retry_attempts
        );
        
        Ok(())
    }
    
    /// Updates log when a duplicate invoice is detected
    pub async fn log_duplicate(
        &self,
        log_id: i32,
        cufe: &str,
        start_time: DateTime<Utc>,
    ) -> Result<(), InvoiceProcessingError> {
        let execution_time_ms = (Utc::now() - start_time).num_milliseconds() as i32;
        
        update_log_duplicate(&self.pool, log_id, cufe, execution_time_ms).await?;
        
        info!(
            "Logged duplicate detection for CUFE: {}, execution time: {}ms",
            cufe, execution_time_ms
        );
        
        Ok(())
    }
    
    /// Updates log when validation fails
    pub async fn log_validation_error(
        &self,
        log_id: i32,
        errors: &[String],
        start_time: DateTime<Utc>,
    ) -> Result<(), InvoiceProcessingError> {
        let execution_time_ms = (Utc::now() - start_time).num_milliseconds() as i32;
        let error_message = errors.join("; ");
        
        update_log_error(
            &self.pool,
            log_id,
            LogStatus::ValidationError,
            ErrorType::InvalidUrl, // Default - could be more specific
            &error_message,
            Some(execution_time_ms),
            0,
        ).await?;
        
        warn!(
            "Logged validation error for log_id: {}, errors: {}",
            log_id, error_message
        );
        
        Ok(())
    }
    
    /// Updates log when scraping fails
    pub async fn log_scraping_error(
        &self,
        log_id: i32,
        error_message: &str,
        error_type: ErrorType,
        start_time: DateTime<Utc>,
        retry_attempts: u32,
    ) -> Result<(), InvoiceProcessingError> {
        let execution_time_ms = (Utc::now() - start_time).num_milliseconds() as i32;
        
        update_log_error(
            &self.pool,
            log_id,
            LogStatus::ScrapingError,
            error_type.clone(),
            error_message,
            Some(execution_time_ms),
            retry_attempts as i32,
        ).await?;
        
        error!(
            "Logged scraping error for log_id: {}, type: {:?}, message: {}, retries: {}",
            log_id, error_type, error_message, retry_attempts
        );
        
        Ok(())
    }
    
    /// Updates log when database operations fail
    pub async fn log_database_error(
        &self,
        log_id: i32,
        error_message: &str,
        start_time: DateTime<Utc>,
    ) -> Result<(), InvoiceProcessingError> {
        let execution_time_ms = (Utc::now() - start_time).num_milliseconds() as i32;
        
        update_log_error(
            &self.pool,
            log_id,
            LogStatus::DatabaseError,
            ErrorType::DbTransactionError,
            error_message,
            Some(execution_time_ms),
            0,
        ).await?;
        
        error!(
            "Logged database error for log_id: {}, message: {}",
            log_id, error_message
        );
        
        Ok(())
    }
    
    /// Updates log when timeout occurs
    pub async fn log_timeout_error(
        &self,
        log_id: i32,
        start_time: DateTime<Utc>,
        retry_attempts: u32,
    ) -> Result<(), InvoiceProcessingError> {
        let execution_time_ms = (Utc::now() - start_time).num_milliseconds() as i32;
        let error_message = format!("Timeout after {} attempts", retry_attempts);
        
        update_log_error(
            &self.pool,
            log_id,
            LogStatus::TimeoutError,
            ErrorType::Timeout,
            &error_message,
            Some(execution_time_ms),
            retry_attempts as i32,
        ).await?;
        
        error!(
            "Logged timeout error for log_id: {}, attempts: {}, execution time: {}ms",
            log_id, retry_attempts, execution_time_ms
        );
        
        Ok(())
    }
    
    /// Updates log when network error occurs
    pub async fn log_network_error(
        &self,
        log_id: i32,
        error_message: &str,
        start_time: DateTime<Utc>,
        retry_attempts: u32,
    ) -> Result<(), InvoiceProcessingError> {
        let execution_time_ms = (Utc::now() - start_time).num_milliseconds() as i32;
        
        update_log_error(
            &self.pool,
            log_id,
            LogStatus::NetworkError,
            ErrorType::Unknown,
            error_message,
            Some(execution_time_ms),
            retry_attempts as i32,
        ).await?;
        
        error!(
            "Logged network error for log_id: {}, message: {}, retries: {}",
            log_id, error_message, retry_attempts
        );
        
        Ok(())
    }
}

// ============================================================================
// METRICS AND ANALYTICS HELPERS
// ============================================================================

impl LoggingService {
    /// Get statistics for a specific user
    pub async fn get_user_stats(
        &self,
        user_id: &str,
        days: i32,
    ) -> Result<UserProcessingStats, InvoiceProcessingError> {
        let query = r#"
            SELECT 
                COUNT(*) as total_requests,
                COUNT(CASE WHEN status = 'SUCCESS' THEN 1 END) as successful_requests,
                COUNT(CASE WHEN status = 'DUPLICATE' THEN 1 END) as duplicate_requests,
                COUNT(CASE WHEN status NOT IN ('SUCCESS', 'DUPLICATE') THEN 1 END) as failed_requests,
                AVG(execution_time_ms) as avg_execution_time_ms
            FROM logs.bot_rust_scrapy 
            WHERE user_id = $1 
            AND request_timestamp >= NOW() - INTERVAL '%d days'
        "#;
        
        let formatted_query = query.replace("%d", &days.to_string());
        
        let row = sqlx::query(&formatted_query)
            .bind(user_id)
            .fetch_one(&self.pool)
            .await?;
        
        Ok(UserProcessingStats {
            total_requests: row.try_get::<i64, _>("total_requests")? as u32,
            successful_requests: row.try_get::<i64, _>("successful_requests")? as u32,
            duplicate_requests: row.try_get::<i64, _>("duplicate_requests")? as u32,
            failed_requests: row.try_get::<i64, _>("failed_requests")? as u32,
            avg_execution_time_ms: row.try_get::<Option<f64>, _>("avg_execution_time_ms")?,
        })
    }
    
    /// Get system-wide statistics
    pub async fn get_system_stats(
        &self,
        hours: i32,
    ) -> Result<SystemProcessingStats, InvoiceProcessingError> {
        let query = r#"
            SELECT 
                COUNT(*) as total_requests,
                COUNT(CASE WHEN status = 'SUCCESS' THEN 1 END) as successful_requests,
                COUNT(CASE WHEN status = 'DUPLICATE' THEN 1 END) as duplicate_requests,
                COUNT(DISTINCT user_id) as unique_users,
                AVG(execution_time_ms) as avg_execution_time_ms,
                MAX(execution_time_ms) as max_execution_time_ms,
                COUNT(CASE WHEN retry_attempts > 0 THEN 1 END) as requests_with_retries
            FROM logs.bot_rust_scrapy 
            WHERE request_timestamp >= NOW() - INTERVAL '%d hours'
        "#;
        
        let formatted_query = query.replace("%d", &hours.to_string());
        
        let row = sqlx::query(&formatted_query)
            .fetch_one(&self.pool)
            .await?;
        
        Ok(SystemProcessingStats {
            total_requests: row.try_get::<i64, _>("total_requests")? as u32,
            successful_requests: row.try_get::<i64, _>("successful_requests")? as u32,
            duplicate_requests: row.try_get::<i64, _>("duplicate_requests")? as u32,
            unique_users: row.try_get::<i64, _>("unique_users")? as u32,
            avg_execution_time_ms: row.try_get::<Option<f64>, _>("avg_execution_time_ms")?,
            max_execution_time_ms: row.try_get::<Option<i32>, _>("max_execution_time_ms")?,
            requests_with_retries: row.try_get::<i64, _>("requests_with_retries")? as u32,
        })
    }
}

// ============================================================================
// STATISTICS MODELS
// ============================================================================

#[derive(Debug, Clone)]
pub struct UserProcessingStats {
    pub total_requests: u32,
    pub successful_requests: u32,
    pub duplicate_requests: u32,
    pub failed_requests: u32,
    pub avg_execution_time_ms: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct SystemProcessingStats {
    pub total_requests: u32,
    pub successful_requests: u32,
    pub duplicate_requests: u32,
    pub unique_users: u32,
    pub avg_execution_time_ms: Option<f64>,
    pub max_execution_time_ms: Option<i32>,
    pub requests_with_retries: u32,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    #[ignore] // Requires database setup
    async fn test_logging_service_full_flow() {
        // This test would verify the complete logging flow
        // from start_processing to final status update
    }
    
    #[tokio::test]
    #[ignore] // Requires database setup
    async fn test_user_stats_calculation() {
        // This test would verify user statistics calculation
    }
    
    #[tokio::test]
    #[ignore] // Requires database setup
    async fn test_system_stats_calculation() {
        // This test would verify system statistics calculation
    }
}
