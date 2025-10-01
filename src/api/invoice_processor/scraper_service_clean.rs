use chrono::{DateTime, Utc};
use std::time::Duration;
use tokio::time::timeout;
use crate::api::invoice_processor::models::{FullInvoiceData, ErrorType};
use crate::api::invoice_processor::error_handling::InvoiceProcessingError;
use crate::api::invoice_processor::validation::categorize_error;
use tracing::{info, error, debug};

// ============================================================================
// SCRAPER SERVICE
// ============================================================================

pub struct ScraperService {
    timeout_seconds: u64,
    max_retries: u32,
}

impl ScraperService {
    pub fn new() -> Self {
        Self {
            timeout_seconds: 30,
            max_retries: 2,
        }
    }
    
    pub fn with_timeout(mut self, seconds: u64) -> Self {
        self.timeout_seconds = seconds;
        self
    }
    
    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }
}

impl ScraperService {
    /// Main scraping function with retry logic and timeout
    pub async fn scrape_invoice_with_retries(
        &self,
        url: &str,
        user_id: &str,
        user_email: &str,
        origin: &str,
        invoice_type: &str,
        reception_date: DateTime<Utc>,
        process_date: DateTime<Utc>,
    ) -> Result<(FullInvoiceData, u32, u32), InvoiceProcessingError> {
        let mut retry_count = 0;
        let mut last_error = None;
        
        while retry_count <= self.max_retries {
            if retry_count > 0 {
                info!("Retry attempt {} for URL: {}", retry_count, url);
                // Exponential backoff: 1s, 2s, 4s
                let delay = Duration::from_secs(2_u64.pow(retry_count - 1));
                tokio::time::sleep(delay).await;
            }
            
            match self.scrape_invoice_attempt(
                url, 
                user_id, 
                user_email, 
                origin, 
                invoice_type,
                reception_date,
                process_date
            ).await {
                Ok((invoice_data, fields_count)) => {
                    info!(
                        "Successfully scraped invoice after {} retries, {} fields extracted",
                        retry_count, fields_count
                    );
                    return Ok((invoice_data, fields_count, retry_count));
                },
                Err(e) => {
                    error!("Scraping attempt {} failed: {}", retry_count + 1, e);
                    last_error = Some(e);
                    retry_count += 1;
                }
            }
        }
        
        // All retries exhausted
        let error_message = last_error
            .map(|e| e.to_string())
            .unwrap_or_else(|| "Unknown scraping error".to_string());
        
        let error_type = categorize_error(&error_message);
        
        Err(InvoiceProcessingError::ScrapingError {
            message: format!("Failed after {} attempts: {}", self.max_retries + 1, error_message),
            error_type,
            retry_attempts: retry_count,
        })
    }
    
    /// Single scraping attempt with timeout
    async fn scrape_invoice_attempt(
        &self,
        url: &str,
        user_id: &str,
        user_email: &str,
        origin: &str,
        invoice_type: &str,
        reception_date: DateTime<Utc>,
        process_date: DateTime<Utc>,
    ) -> Result<(FullInvoiceData, u32), InvoiceProcessingError> {
        debug!("Starting scraping attempt for URL: {}", url);
        
        let scraping_future = self.perform_scraping(
            url, 
            user_id, 
            user_email, 
            origin, 
            invoice_type,
            reception_date,
            process_date
        );
        
        match timeout(Duration::from_secs(self.timeout_seconds), scraping_future).await {
            Ok(result) => result,
            Err(_) => {
                error!("Scraping timeout after {} seconds for URL: {}", self.timeout_seconds, url);
                Err(InvoiceProcessingError::TimeoutError { attempts: 1 })
            }
        }
    }
    
    /// Core scraping logic - temporarily disabled
    async fn perform_scraping(
        &self,
        _url: &str,
        _user_id: &str,
        _user_email: &str,
        _origin: &str,
        _invoice_type: &str,
        _reception_date: DateTime<Utc>,
        _process_date: DateTime<Utc>,
    ) -> Result<(FullInvoiceData, u32), InvoiceProcessingError> {
        // TODO: Temporarily disabled to allow compilation for other tests.
        // This needs to be fixed to align with the new data structures.
        Err(InvoiceProcessingError::ScrapingError {
            message: "Scraping logic is temporarily disabled.".to_string(),
            error_type: ErrorType::Other,
            retry_attempts: 0,
        })
    }
}

impl Default for ScraperService {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_scraper_service_creation() {
        let service = ScraperService::new();
        assert_eq!(service.timeout_seconds, 30);
        assert_eq!(service.max_retries, 2);
    }
    
    #[tokio::test]
    async fn test_scraper_service_configuration() {
        let service = ScraperService::new()
            .with_timeout(60)
            .with_max_retries(5);
        
        assert_eq!(service.timeout_seconds, 60);
        assert_eq!(service.max_retries, 5);
    }
}
