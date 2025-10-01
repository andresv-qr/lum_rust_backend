use chrono::{DateTime, Utc};
use std::time::Duration;
use tokio::time::timeout;
use crate::api::invoice_processor::models::{FullInvoiceData};
use crate::api::invoice_processor::error_handling::InvoiceProcessingError;
use crate::api::invoice_processor::validation::categorize_error;
use crate::models::invoice::InvoicePayment;
use tracing::{info, error, debug};
use reqwest;

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
        user_id_str: &str, // Renamed to avoid shadowing
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
                user_id_str, 
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
                    error!("Retry {} failed: {:?}", retry_count + 1, e);
                    last_error = Some(e);
                    retry_count += 1;
                }
            }
        }
        
        // All retries exhausted
        let error_message = last_error
            .map(|e| format!("{:?}", e))
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
        user_id_str: &str, // Renamed
        user_email: &str,
        origin: &str,
        invoice_type: &str,
        reception_date: DateTime<Utc>,
        process_date: DateTime<Utc>,
    ) -> Result<(FullInvoiceData, u32), InvoiceProcessingError> {
        debug!("Starting scraping attempt for URL: {}", url);
        
        let scraping_future = self.perform_scraping(
            url, 
            user_id_str, 
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
    
    /// Core scraping logic - now enabled and aligned with database schema
    async fn perform_scraping(
        &self,
        url: &str,
        user_id_str: &str, // Renamed
        user_email: &str,
        origin: &str,
        invoice_type: &str,
        reception_date: DateTime<Utc>,
        process_date: DateTime<Utc>,
    ) -> Result<(FullInvoiceData, u32), InvoiceProcessingError> {
        use crate::processing::web_scraping::ocr_extractor::extract_main_info;
        
        // Parse user_id from string to i64, with error handling
        let _user_id: i64 = user_id_str.parse().unwrap_or_else(|e| {
            error!("🚨 Failed to parse user_id '{}' as i64: {}. Defaulting to 0.", user_id_str, e);
            0
        });

        let start_time = std::time::Instant::now();
        
        // Fetch HTML content and capture final URL after redirections
        let client = reqwest::Client::new();
        let response = client
            .get(url)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| InvoiceProcessingError::NetworkError(e.to_string()))?;
            
        let final_url = response.url().to_string();
        
        // Log redirection if it occurred
        if final_url != url {
            info!("🔄 Scraper service detected redirection: {} → {}", url, final_url);
        }
            
        let html_content = response
            .text()
            .await
            .map_err(|e| InvoiceProcessingError::NetworkError(e.to_string()))?;
            
        let extracted_data = extract_main_info(&html_content)
            .map_err(|e| InvoiceProcessingError::DataParsingError(e.to_string()))?;

        // Use the data parser to convert extracted HashMap to proper structs
        // Use final_url instead of original url for consistent data
        let (mut header, details, payments) = crate::processing::web_scraping::data_parser::parse_invoice_data(&extracted_data, &final_url)
            .map_err(|e| InvoiceProcessingError::DataParsingError(e.to_string()))?;
        
        let fields_count = 10; // Count of header fields extracted

        // Populate the remaining fields, now with the correct user_id type
        header.r#type = invoice_type.to_string();
        header.reception_date = reception_date;
        header.process_date = process_date;
        header.user_email = user_email.to_string();
        header.origin = origin.to_string();
        
        // Use the first payment or create a default one
        let payment = if !payments.is_empty() {
            payments[0].clone()
        } else {
            InvoicePayment {
                cufe: header.cufe.clone(),
                vuelto: Some("0.00".to_string()),
                total_pagado: Some(header.tot_amount.to_string()),
            }
        };

        let full_invoice_data = FullInvoiceData {
            header,
            details,
            payment,
        };
        
        let duration = start_time.elapsed();
        info!("Scraping and processing finished in {} ms.", duration.as_millis());

        Ok((full_invoice_data, fields_count))
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
