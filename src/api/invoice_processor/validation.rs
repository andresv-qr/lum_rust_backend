use crate::api::invoice_processor::models::{ProcessInvoiceRequest, ErrorType};

// ============================================================================
// VALIDATION FUNCTIONS
// ============================================================================

pub fn validate_process_request(request: &ProcessInvoiceRequest) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    
    // Validate URL
    if let Err(url_error) = validate_dgi_url(&request.url) {
        errors.push(url_error);
    }
    
    // Validate user_id
    if request.user_id.trim().is_empty() {
        errors.push("user_id no puede estar vacío".to_string());
    }
    
    // Validate email
    if let Err(email_error) = validate_email(&request.user_email) {
        errors.push(email_error);
    }
    
    // Validate origin
    if let Err(origin_error) = validate_origin(&request.origin) {
        errors.push(origin_error);
    }
    
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn validate_dgi_url(url: &str) -> Result<(), String> {
    // Check if URL contains DGI domain
    if !url.contains("dgi-fep.mef.gob.pa") {
        return Err("URL no corresponde a DGI Panamá".to_string());
    }
    
    // Basic URL format validation
    if !url.starts_with("https://") && !url.starts_with("http://") {
        return Err("URL debe comenzar con http:// o https://".to_string());
    }
    
    Ok(())
}

fn validate_email(email: &str) -> Result<(), String> {
    // Simple email validation without regex dependency
    if email.contains('@') && email.contains('.') && email.len() > 5 {
        Ok(())
    } else {
        Err("Email inválido".to_string())
    }
}

fn validate_origin(origin: &str) -> Result<(), String> {
    let allowed_origins = ["whatsapp", "aplicacion", "telegram"];
    
    if !allowed_origins.contains(&origin.to_lowercase().as_str()) {
        return Err(format!(
            "Origin debe ser uno de: {}. Recibido: {}",
            allowed_origins.join(", "),
            origin
        ));
    }
    
    Ok(())
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

pub fn determine_invoice_type(url: &str) -> String {
    if url.contains("FacturasPorQR") {
        "QR".to_string()
    } else {
        "CUFE".to_string()
    }
}

pub fn categorize_error(error_message: &str) -> ErrorType {
    let error_lower = error_message.to_lowercase();
    
    if error_lower.contains("url") && (error_lower.contains("invalid") || error_lower.contains("inválid")) {
        ErrorType::InvalidUrl
    } else if error_lower.contains("cufe") && error_lower.contains("not found") {
        ErrorType::CufeNotFound
    } else if error_lower.contains("html") || error_lower.contains("parse") {
        ErrorType::HtmlParseError
    } else if error_lower.contains("database") || error_lower.contains("db") {
        if error_lower.contains("connection") {
            ErrorType::DbConnectionError
        } else {
            ErrorType::DbTransactionError
        }
    } else if error_lower.contains("timeout") {
        ErrorType::Timeout
    } else if error_lower.contains("missing") || error_lower.contains("required") {
        ErrorType::MissingFields
    } else {
        ErrorType::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_validate_dgi_url() {
        // Valid URLs
        assert!(validate_dgi_url("https://dgi-fep.mef.gob.pa/FacturasPorQR").is_ok());
        assert!(validate_dgi_url("http://dgi-fep.mef.gob.pa/consulta").is_ok());
        
        // Invalid URLs
        assert!(validate_dgi_url("https://other-site.com").is_err());
        assert!(validate_dgi_url("dgi-fep.mef.gob.pa/test").is_err());
    }
    
    #[test]
    fn test_validate_email() {
        // Valid emails
        assert!(validate_email("user@example.com").is_ok());
        assert!(validate_email("test.email+tag@domain.co.uk").is_ok());
        
        // Invalid emails
        assert!(validate_email("invalid-email").is_err());
        assert!(validate_email("@domain.com").is_err());
        assert!(validate_email("user@").is_err());
    }
    
    #[test]
    fn test_validate_origin() {
        // Valid origins
        assert!(validate_origin("whatsapp").is_ok());
        assert!(validate_origin("aplicacion").is_ok());
        assert!(validate_origin("telegram").is_ok());
        assert!(validate_origin("WHATSAPP").is_ok()); // Case insensitive
        
        // Invalid origins
        assert!(validate_origin("facebook").is_err());
        assert!(validate_origin("").is_err());
    }
    
    #[test]
    fn test_determine_invoice_type() {
        assert_eq!(determine_invoice_type("https://dgi-fep.mef.gob.pa/FacturasPorQR"), "QR");
        assert_eq!(determine_invoice_type("https://dgi-fep.mef.gob.pa/consulta"), "CUFE");
    }
    
    #[test]
    fn test_categorize_error() {
        assert!(matches!(categorize_error("Invalid URL provided"), ErrorType::InvalidUrl));
        assert!(matches!(categorize_error("CUFE not found in HTML"), ErrorType::CufeNotFound));
        assert!(matches!(categorize_error("HTML parse error"), ErrorType::HtmlParseError));
        assert!(matches!(categorize_error("Database connection failed"), ErrorType::DbConnectionError));
        assert!(matches!(categorize_error("Request timeout"), ErrorType::Timeout));
        assert!(matches!(categorize_error("Unknown error occurred"), ErrorType::Unknown));
    }
}
