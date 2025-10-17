use reqwest::Client;
use chrono::{DateTime, Utc};
use std::time::Duration;
use tokio::time::timeout;
use crate::api::invoices::models::{FullInvoiceData, InvoiceData, InvoiceDetailItem, InvoicePayment, ErrorType};
use crate::api::invoices::error_handling::InvoiceProcessingError;
use crate::api::invoices::validation::categorize_error;
use tracing::{info, error, debug};

// Re-export the existing scraper functions from webscraping module
use crate::api::webscraping::{
    scrape_invoice, ScrapingResult
};

// ============================================================================
// SCRAPER SERVICE
// ============================================================================

pub struct ScraperService {
    client: Client,
    timeout_seconds: u64,
    max_retries: u32,
}

impl ScraperService {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            client,
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
    
    /// Core scraping logic
    async fn perform_scraping(
        &self,
        url: &str,
        user_id: &str,
        user_email: &str,
        origin: &str,
        invoice_type: &str,
        reception_date: DateTime<Utc>,
        process_date: DateTime<Utc>,
    ) -> Result<(FullInvoiceData, u32), InvoiceProcessingError> {
        debug!("Starting scraping for URL: {}", url);
        
        // 1. Use existing scraper function
        let scraping_result = self.extract_all_invoice_data("", url).await?;
        
        debug!("Successfully got scraping result");
        
        // 2. Convert to our API models and add user inputs
        let full_invoice_data = self.build_full_invoice_data(
            scraping_result,
            url,
            user_id,
            user_email,
            origin,
            invoice_type,
            reception_date,
            process_date,
        )?;
        
        // 3. Count extracted fields
        let fields_count = self.count_extracted_fields(&full_invoice_data);
        
        info!(
            "Successfully extracted {} fields for CUFE: {}",
            fields_count, full_invoice_data.header.cufe
        );
        
        Ok((full_invoice_data, fields_count))
    }
    
    /// Extract all invoice data using existing scraper functions
    async fn extract_all_invoice_data(
        &self,
        _html_content: &str,
        url: &str,
    ) -> Result<ScrapingResult, InvoiceProcessingError> {
        // Use the existing scraping function from webscraping module
        // TODO: Pass real user_id when available in this context
        match scrape_invoice(&self.client, url, 1).await {
            Ok(scraping_result) => {
                // Validate that essential fields are present
                if let Some(ref header) = scraping_result.header {
                    if header.cufe.is_empty() {
                        return Err(InvoiceProcessingError::ScrapingError {
                            message: "CUFE not found in HTML".to_string(),
                            error_type: ErrorType::CufeNotFound,
                            retry_attempts: 0,
                        });
                    }
                    debug!("Successfully extracted header data with CUFE: {}", header.cufe);
                } else {
                    return Err(InvoiceProcessingError::ScrapingError {
                        message: "No header data extracted from HTML".to_string(),
                        error_type: ErrorType::HtmlParseError,
                        retry_attempts: 0,
                    });
                }
                
                Ok(scraping_result)
            },
            Err(e) => {
                error!("Failed to extract invoice data: {}", e);
                Err(InvoiceProcessingError::ScrapingError {
                    message: format!("Extraction failed: {}", e),
                    error_type: categorize_error(&e),
                    retry_attempts: 0,
                })
            }
        }
    }
    
    /// Convert scraped data to API models
    fn build_full_invoice_data(
        &self,
        scraping_result: ScrapingResult,
        url: &str,
        user_id: &str,
        user_email: &str,
        origin: &str,
        invoice_type: &str,
        reception_date: DateTime<Utc>,
        process_date: DateTime<Utc>,
    ) -> Result<FullInvoiceData, InvoiceProcessingError> {
        // Extract header data from the scraping result
        let header_data = scraping_result.header.ok_or_else(|| {
            InvoiceProcessingError::ScrapingError {
                message: "No header data found in scraping result".to_string(),
                error_type: ErrorType::MissingFields,
                retry_attempts: 0,
            }
        })?;
        
        // Build header data (combining scraped + user inputs)
        let header = InvoiceData {
            // Scraped fields (10 fields as per documentation)
            no: header_data.no.unwrap_or_default(),
            date: header_data.date.unwrap_or_default(), // Use string directly
            cufe: header_data.cufe,
            issuer_name: header_data.issuer_name.unwrap_or_default(),
            issuer_ruc: header_data.issuer_ruc.unwrap_or_default(),
            issuer_dv: header_data.issuer_dv.unwrap_or_default(),
            issuer_address: header_data.issuer_address.unwrap_or_default(),
            issuer_phone: header_data.issuer_phone.unwrap_or_default(),
            tot_amount: header_data.tot_amount.map(|d| d.to_string()).unwrap_or_default(),
            tot_itbms: header_data.tot_itbms.map(|d| d.to_string()).unwrap_or_default(),
            
            // User input fields (7 fields as per documentation)
            url: url.to_string(),
            r#type: invoice_type.to_string(),
            process_date,
            reception_date,
            user_id: user_id.to_string(),
            origin: origin.to_string(),
            user_email: user_email.to_string(),
        };
        
        // Build details (convert from scraped details)
        let details = scraping_result.details.into_iter().map(|detail| {
            InvoiceDetailItem {
                cufe: header.cufe.clone(),
                quantity: detail.quantity.clone().unwrap_or_default(),
                code: detail.code.clone().unwrap_or_default(),
                description: detail.description.clone().unwrap_or_default(),
                unit_discount: detail.unit_discount.clone().unwrap_or_default(),
                unit_price: detail.unit_price.clone().unwrap_or_default(),
                itbms: detail.itbms.clone().unwrap_or_default(),
                information_of_interest: detail.information_of_interest.clone().unwrap_or_default()
            }
        }).collect();
        
        // Build payment data (convert from scraped payments)
        let payment = if let Some(first_payment) = scraping_result.payments.first() {
            InvoicePayment {
                cufe: header.cufe.clone(),
                vuelto: first_payment.vuelto.clone().unwrap_or_default(),
                total_pagado: first_payment.valor_pago.clone().unwrap_or_default(),
            }
        } else {
            // Default payment data if no payment info found
            InvoicePayment {
                cufe: header.cufe.clone(),
                vuelto: "0.00".to_string(),
                total_pagado: header.tot_amount.clone(),
            }
        };
        
        Ok(FullInvoiceData {
            header,
            details,
            payment,
        })
    }
    
    /// Count how many fields were successfully extracted
    fn count_extracted_fields(&self, invoice_data: &FullInvoiceData) -> u32 {
        let mut count = 0;
        
        // Header fields (only count non-empty scraped fields)
        if !invoice_data.header.no.is_empty() { count += 1; }
        if !invoice_data.header.date.is_empty() { count += 1; }
        if !invoice_data.header.cufe.is_empty() { count += 1; }
        if !invoice_data.header.issuer_name.is_empty() { count += 1; }
        if !invoice_data.header.issuer_ruc.is_empty() { count += 1; }
        if !invoice_data.header.issuer_dv.is_empty() { count += 1; }
        if !invoice_data.header.issuer_address.is_empty() { count += 1; }
        if !invoice_data.header.issuer_phone.is_empty() { count += 1; }
        if !invoice_data.header.tot_amount.is_empty() { count += 1; }
        if !invoice_data.header.tot_itbms.is_empty() { count += 1; }
        
        // Detail fields (count per item)
        for detail in &invoice_data.details {
            if !detail.quantity.is_empty() { count += 1; }
            if !detail.code.is_empty() { count += 1; }
            if !detail.description.is_empty() { count += 1; }
            if !detail.unit_discount.is_empty() { count += 1; }
            if !detail.unit_price.is_empty() { count += 1; }
            if !detail.itbms.is_empty() { count += 1; }
            if !detail.information_of_interest.is_empty() { count += 1; }
        }
        
        // Payment fields
        if !invoice_data.payment.vuelto.is_empty() { count += 1; }
        if !invoice_data.payment.total_pagado.is_empty() { count += 1; }
        
        count
    }
}

impl Default for ScraperService {
    fn default() -> Self {
        Self::new()
    }
}

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
    
    #[test]
    fn test_count_extracted_fields() {
        let service = ScraperService::new();
        
        // Create mock invoice data
        let invoice_data = FullInvoiceData {
            header: InvoiceData {
                no: "123".to_string(),
                date: "01/01/2025".to_string(),
                cufe: "FE123...".to_string(),
                issuer_name: "Test Company".to_string(),
                issuer_ruc: "123456".to_string(),
                issuer_dv: "1".to_string(),
                issuer_address: "".to_string(), // Empty field
                issuer_phone: "555-1234".to_string(),
                tot_amount: "100.00".to_string(),
                tot_itbms: "7.00".to_string(),
                url: "test".to_string(),
                r#type: "QR".to_string(),
                process_date: Utc::now(),
                reception_date: Utc::now(),
                user_id: "user1".to_string(),
                origin: "test".to_string(),
                user_email: "test@test.com".to_string(),
            },
            details: vec![
                InvoiceDetailItem {
                    cufe: "FE123...".to_string(),
                    quantity: "1".to_string(),
                    code: "ITEM1".to_string(),
                    description: "Test Item".to_string(),
                    unit_discount: "0.00".to_string(),
                    unit_price: "50.00".to_string(),
                    itbms: "3.50".to_string(),
                    information_of_interest: "".to_string(), // Empty field
                }
            ],
            payment: InvoicePayment {
                cufe: "FE123...".to_string(),
                vuelto: "0.00".to_string(),
                total_pagado: "100.00".to_string(),
            },
        };
        
        let count = service.count_extracted_fields(&invoice_data);
        // 9 header fields (1 empty) + 6 detail fields (1 empty) + 2 payment fields = 17
        assert_eq!(count, 17);
    }
}
