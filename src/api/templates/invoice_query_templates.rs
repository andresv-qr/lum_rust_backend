use serde::{Deserialize, Serialize, Deserializer};
use chrono::{DateTime, Utc, NaiveDateTime};
use chrono_tz::America::Panama;
use base64::Engine as _;

// ============================================================================
// REQUEST MODELS
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct InvoiceDetailsRequest {
    /// Fecha desde (timezone Panamá) - formato: "YYYY-MM-DD HH:MM:SS" o ISO 8601
    #[serde(deserialize_with = "deserialize_panama_datetime")]
    pub from_date: DateTime<Utc>,
    /// Fecha hasta (opcional)
    #[serde(default, deserialize_with = "deserialize_panama_datetime_option")]
    pub to_date: Option<DateTime<Utc>>,
    pub invoice_type: Option<String>,
    pub invoice_types: Option<String>,
    /// Monto mínimo
    pub min_amount: Option<f64>,
    /// Monto máximo
    pub max_amount: Option<f64>,
    /// Límite de resultados (default: 100, max: 1000)
    pub limit: Option<i32>,
    /// Offset para paginación (default: 0) - DEPRECATED: usar cursor
    pub offset: Option<i32>,
    /// Número de página (alternativo a offset) - DEPRECATED: usar cursor
    pub page: Option<i32>,
    /// Campo para ordenar (default: "date")
    pub order_by: Option<String>,
    /// Dirección de ordenamiento (default: "DESC")
    pub order_direction: Option<String>,
    /// Cursor para paginación keyset (base64 encoded: "YYYY-MM-DDTHH:MM:SSZ|id")
    pub cursor: Option<String>,
    /// Dirección de navegación: "forward" (default) o "backward"
    pub direction: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct InvoiceHeadersRequest {
    /// Fecha desde (timezone Panamá) - formato: "YYYY-MM-DD HH:MM:SS" o ISO 8601
    #[serde(default, deserialize_with = "deserialize_panama_datetime_option")]
    pub from_date: Option<DateTime<Utc>>,
    /// Fecha hasta (timezone Panamá) - formato: "YYYY-MM-DD HH:MM:SS" o ISO 8601
    #[serde(default, deserialize_with = "deserialize_panama_datetime_option")]
    pub to_date: Option<DateTime<Utc>>,
    pub issuer_name: Option<String>,
    pub min_amount: Option<f64>,
    pub max_amount: Option<f64>,
    pub limit: Option<i32>,
    pub offset: Option<i32>,
    
    // ✅ KEYSET PAGINATION SUPPORT
    /// Cursor for keyset pagination (base64 encoded position)
    pub cursor: Option<String>,
    /// Direction for cursor navigation ("next" or "prev")
    pub direction: Option<String>,
}

// ============================================================================
// RESPONSE MODELS
// ============================================================================

#[derive(Debug, Serialize)]
pub struct InvoiceDetailsResponse {
    pub data: Vec<InvoiceDetailItem>,
    pub pagination: PaginationMeta,
    pub performance: PerformanceMeta,
}

#[derive(Debug, Serialize)]
pub struct PaginationMeta {
    // OFFSET/PAGE PAGINATION (DEPRECATED)
    pub total: i64,
    pub limit: i32,
    pub offset: i32,
    pub page: i32,
    pub total_pages: i32,
    pub has_next: bool,
    pub has_previous: bool,
    pub next_offset: Option<i32>,
    pub previous_offset: Option<i32>,
    
    // CURSOR/KEYSET PAGINATION (PREFERRED)
    pub cursor_pagination: Option<CursorPaginationMeta>,
}

#[derive(Debug, Serialize)]
pub struct CursorPaginationMeta {
    pub next_cursor: Option<String>,
    pub previous_cursor: Option<String>,
    pub has_next_page: bool,
    pub has_previous_page: bool,
    pub page_size: i32,
    pub direction: String, // "next" or "prev"
}

#[derive(Debug, Serialize)]
pub struct PerformanceMeta {
    pub query_time_ms: u64,
    pub cached: bool,
}

#[derive(Debug, Serialize)]
pub struct InvoiceHeadersResponse {
    pub success: bool,
    pub data: Vec<InvoiceHeaderItem>,
    pub total_count: i64,
    pub page_info: PageInfo,
    pub summary: InvoiceSummary,
}

#[derive(Debug, Serialize)]
pub struct InvoiceDetailItem {
    pub id: i64,
    pub cufe: Option<String>,
    pub quantity: Option<f64>,
    pub code: Option<String>,
    pub date: Option<NaiveDateTime>,
    pub total: Option<f64>,
    pub unit_price: Option<f64>,
    pub amount: Option<f64>,
    pub unit_discount: Option<String>,
    pub description: Option<String>,
    pub issuer_name: Option<String>,
    pub reception_date: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct InvoiceHeaderItem {
    pub id: i64,
    pub date: Option<NaiveDateTime>,
    pub tot_itbms: Option<f64>,
    pub issuer_name: Option<String>,
    pub no: Option<String>,
    pub tot_amount: Option<f64>,
    pub url: Option<String>,
    pub process_date: Option<DateTime<Utc>>,
    pub reception_date: Option<DateTime<Utc>>,
    pub r#type: Option<String>,
    pub cufe: Option<String>,
    // Legacy fields for compatibility
    pub time: Option<String>,
    pub auth_date: Option<String>,
    pub issuer_ruc: Option<String>,
    pub issuer_dv: Option<String>,
    pub issuer_address: Option<String>,
    pub issuer_phone: Option<String>,
    pub receptor_name: Option<String>,
    pub details_count: i64,
    pub payments_count: i64,
}

#[derive(Debug, Serialize)]
pub struct PageInfo {
    pub current_page: i32,
    pub page_size: i32,
    pub total_pages: i32,
    pub has_next: bool,
    pub has_previous: bool,
    
    // ✅ CURSOR PAGINATION SUPPORT
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor_pagination: Option<CursorPaginationMeta>,
}

#[derive(Debug, Serialize)]
pub struct FiltersApplied {
    pub from_date: DateTime<Utc>,
    pub invoice_types: Vec<String>,
    pub date_range_days: i64,
}

#[derive(Debug, Serialize)]
pub struct InvoiceSummary {
    pub total_invoices: i64,
    pub total_amount: f64,
    pub unique_issuers: i64,
    pub date_range: DateRange,
    pub amount_range: AmountRange,
}

#[derive(Debug, Serialize)]
pub struct DateRange {
    pub earliest: Option<DateTime<Utc>>,
    pub latest: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct AmountRange {
    pub minimum: f64,
    pub maximum: f64,
    pub average: f64,
}

// ============================================================================
// QUERY TEMPLATES
// ============================================================================

pub struct InvoiceQueryTemplates;

impl InvoiceQueryTemplates {
    pub const GET_INVOICE_DETAILS: &'static str = r#"
        SELECT 
            ROW_NUMBER() OVER (ORDER BY d.date DESC) as id,
            d.cufe,
            d.quantity,
            d.code,
            d.description,
            d.unit_price,
            d.amount,
            d.unit_discount,
            d.date,
            d.total,
            d.issuer_name,
            d.reception_date
        FROM public.invoice_with_details d
        WHERE d.user_id = $1
    "#;
    
    pub const GET_INVOICE_DETAILS_WITH_TYPE_FILTER: &'static str = r#"
        SELECT 
            ROW_NUMBER() OVER (ORDER BY d.date DESC) as id,
            d.cufe, d.quantity, d.code, d.description, d.unit_price, d.amount,
            d.unit_discount, d.date, d.total,
            d.issuer_name, d.reception_date
        FROM public.invoice_with_details d
        WHERE d.user_id = $1
        AND d.type = ANY($2)
    "#;
    
    pub const COUNT_INVOICE_DETAILS: &'static str = r#"
        SELECT COUNT(*) as count
        FROM public.invoice_with_details d
        WHERE d.user_id = $1
        AND d.reception_date >= $2
    "#;
    
    pub const COUNT_INVOICE_DETAILS_WITH_TYPE_FILTER: &'static str = r#"
        SELECT COUNT(*) as count
        FROM public.invoice_with_details d
        WHERE d.user_id = $1
        AND d.reception_date >= $2
        AND d.type = ANY($3)
    "#;
    
    pub const GET_INVOICE_HEADERS: &'static str = r#"
        SELECT 
            ROW_NUMBER() OVER (ORDER BY h.reception_date DESC) as id,
            h.no,
            h.date,
            h.tot_itbms,
            h.cufe,
            h.issuer_name,
            h.tot_amount,
            h.url,
            h.process_date,
            h.reception_date,
            h.type,
            '' as time,
            '' as auth_date,
            h.issuer_ruc,
            h.issuer_dv,
            h.issuer_address,
            h.issuer_phone,
            '' as receptor_name,
            COALESCE(detail_counts.details_count, 0) as details_count,
            COALESCE(payment_counts.payments_count, 0) as payments_count
        FROM public.invoice_header h
        LEFT JOIN (
            SELECT cufe, COUNT(*) as details_count
            FROM public.invoice_detail
            GROUP BY cufe
        ) detail_counts ON h.cufe = detail_counts.cufe
        LEFT JOIN (
            SELECT cufe, COUNT(*) as payments_count
            FROM public.invoice_payment
            GROUP BY cufe
        ) payment_counts ON h.cufe = payment_counts.cufe
        WHERE h.user_id = $1
    "#;
    
    pub const COUNT_INVOICE_HEADERS: &'static str = r#"
        SELECT COUNT(*) as count
        FROM public.invoice_header h
        WHERE h.user_id = $1
    "#;
    
    pub const GET_INVOICE_SUMMARY: &'static str = r#"
        SELECT 
            COUNT(*) as total_invoices,
            COALESCE(SUM(tot_amount), 0) as total_amount,
            COUNT(DISTINCT issuer_name) as unique_issuers,
            MIN(reception_date) as earliest_date,
            MAX(reception_date) as latest_date,
            MIN(tot_amount) as min_amount,
            MAX(tot_amount) as max_amount,
            AVG(tot_amount) as avg_amount
        FROM public.invoice_header
        WHERE user_id = $1
    "#;
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

impl InvoiceDetailsRequest {
    pub fn get_invoice_types(&self) -> Vec<String> {
        if let Some(ref types_str) = self.invoice_types {
            types_str
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        } else if let Some(ref single_type) = self.invoice_type {
            if single_type == "all" {
                vec![]
            } else {
                vec![single_type.clone()]
            }
        } else {
            vec![]
        }
    }
    
    /// Get validated limit (100 default, 1000 max)
    pub fn get_limit(&self) -> i32 {
        self.limit.unwrap_or(100).min(1000).max(1)
    }
    
    /// Get offset from either offset parameter or calculated from page
    pub fn get_offset(&self) -> i32 {
        if let Some(page) = self.page {
            let limit = self.get_limit();
            (page.max(1) - 1) * limit
        } else {
            self.offset.unwrap_or(0).max(0)
        }
    }
    
    /// Get validated order by field
    pub fn get_order_by(&self) -> &str {
        match self.order_by.as_deref() {
            Some("date") | Some("reception_date") | Some("amount") | Some("issuer_name") => {
                self.order_by.as_deref().unwrap()
            }
            _ => "date"
        }
    }
    
    /// Get validated order direction
    pub fn get_order_direction(&self) -> &str {
        match self.order_direction.as_deref() {
            Some("ASC") | Some("asc") => "ASC",
            _ => "DESC"
        }
    }
}

impl InvoiceHeadersRequest {
    pub fn get_limit(&self) -> i32 {
        self.limit.unwrap_or(100).min(1000).max(1)
    }
    
    pub fn get_offset(&self) -> i32 {
        self.offset.unwrap_or(0).max(0)
    }
    
    /// Get order by field (default: reception_date)
    pub fn get_order_by(&self) -> &str {
        "reception_date" // Default for headers
    }
    
    /// Get order direction (default: DESC) 
    pub fn get_order_direction(&self) -> &str {
        "DESC" // Always DESC for headers
    }
}

impl PageInfo {
    pub fn new(current_page: i32, page_size: i32, total_count: i64) -> Self {
        let total_pages = ((total_count as f64) / (page_size as f64)).ceil() as i32;
        
        Self {
            current_page,
            page_size,
            total_pages,
            has_next: current_page < total_pages,
            has_previous: current_page > 1,
            cursor_pagination: None, // For offset pagination
        }
    }
    
    /// Create PageInfo with cursor pagination
    pub fn new_with_cursor(cursor_pagination: CursorPaginationMeta) -> Self {
        Self {
            current_page: -1, // Not applicable for cursor pagination
            page_size: cursor_pagination.page_size,
            total_pages: -1, // Not available in cursor pagination
            has_next: cursor_pagination.has_next_page,
            has_previous: cursor_pagination.has_previous_page,
            cursor_pagination: Some(cursor_pagination),
        }
    }
}

impl FiltersApplied {
    pub fn new(from_date: DateTime<Utc>, invoice_types: Vec<String>) -> Self {
        let date_range_days = (Utc::now() - from_date).num_days();
        
        Self {
            from_date,
            invoice_types,
            date_range_days,
        }
    }
}

impl InvoiceSummary {
    pub fn new(
        total_invoices: i64,
        total_amount: f64,
        unique_issuers: i64,
        earliest: Option<DateTime<Utc>>,
        latest: Option<DateTime<Utc>>,
        min_amount: f64,
        max_amount: f64,
        avg_amount: f64,
    ) -> Self {
        Self {
            total_invoices,
            total_amount,
            unique_issuers,
            date_range: DateRange { earliest, latest },
            amount_range: AmountRange {
                minimum: min_amount,
                maximum: max_amount,
                average: avg_amount,
            },
        }
    }
}

// ============================================================================
// RESPONSE HELPERS
// ============================================================================

impl InvoiceDetailsResponse {
    /// Create new response with pagination metadata
    pub fn new(data: Vec<InvoiceDetailItem>, total: i64, limit: i32, offset: i32, query_time_ms: u64, cached: bool) -> Self {
        Self {
            data,
            pagination: PaginationMeta::new(total, limit, offset),
            performance: PerformanceMeta {
                query_time_ms,
                cached,
            },
        }
    }

    /// Create new response with cursor pagination
    pub fn new_with_cursor(data: Vec<InvoiceDetailItem>, cursor_pagination: CursorPaginationMeta, query_time_ms: u64, cached: bool) -> Self {
        Self {
            data,
            pagination: PaginationMeta {
                total: -1, // Not available in cursor pagination
                limit: cursor_pagination.page_size,
                offset: -1, // Not applicable in cursor pagination
                page: -1, // Not applicable in cursor pagination
                total_pages: -1, // Not available in cursor pagination
                has_next: cursor_pagination.has_next_page,
                has_previous: cursor_pagination.has_previous_page,
                next_offset: None, // Not applicable in cursor pagination
                previous_offset: None, // Not applicable in cursor pagination
                cursor_pagination: Some(cursor_pagination),
            },
            performance: PerformanceMeta {
                query_time_ms,
                cached,
            },
        }
    }
}

impl PaginationMeta {
    pub fn new(total: i64, limit: i32, offset: i32) -> Self {
        let total_pages = if total == 0 { 1 } else { ((total as f64) / (limit as f64)).ceil() as i32 };
        let current_page = (offset / limit) + 1;
        let has_next = offset + limit < total as i32;
        let has_previous = offset > 0;
        
        let next_offset = if has_next { Some(offset + limit) } else { None };
        let previous_offset = if has_previous { Some((offset - limit).max(0)) } else { None };
        
        Self {
            total,
            limit,
            offset,
            page: current_page,
            total_pages,
            has_next,
            has_previous,
            next_offset,
            previous_offset,
            cursor_pagination: None, // Not used for offset pagination
        }
    }
}

impl InvoiceHeadersResponse {
    pub fn success(
        data: Vec<InvoiceHeaderItem>,
        total_count: i64,
        page_info: PageInfo,
        summary: InvoiceSummary,
    ) -> Self {
        Self {
            success: true,
            data,
            total_count,
            page_info,
            summary,
        }
    }
    
    pub fn empty(summary: InvoiceSummary) -> Self {
        Self {
            success: true,
            data: vec![],
            total_count: 0,
            page_info: PageInfo::new(1, 50, 0),
            summary,
        }
    }
}

// ============================================================================
// CONSTANTS
// ============================================================================

pub const DEFAULT_PAGE_SIZE: i32 = 50;
pub const MAX_PAGE_SIZE: i32 = 200;
pub const DEFAULT_RATE_LIMIT: &str = "50/hour";

// ============================================================================
// CUSTOM DESERIALIZERS FOR PANAMA TIMEZONE
// ============================================================================

/// Deserializa una fecha desde timezone de Panamá y la convierte a UTC
/// Acepta formatos: "YYYY-MM-DD HH:MM:SS", ISO 8601, o timestamp Unix
fn deserialize_panama_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;
    use chrono::{NaiveDateTime, TimeZone};
    
    let s = String::deserialize(deserializer)?;
    
    // Intentar parsear como ISO 8601 primero
    if let Ok(dt) = DateTime::parse_from_rfc3339(&s) {
        return Ok(dt.with_timezone(&Utc));
    }
    
    // Intentar parsear como timestamp Unix
    if let Ok(timestamp) = s.parse::<i64>() {
        if let Some(dt) = DateTime::from_timestamp(timestamp, 0) {
            return Ok(dt);
        }
    }
    
    // Intentar parsear como fecha naive y asumir timezone de Panamá
    let formats = [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d",
    ];
    
    for format in &formats {
        if let Ok(naive_dt) = NaiveDateTime::parse_from_str(&s, format) {
            // Convertir de timezone de Panamá a UTC
            let panama_dt = Panama.from_local_datetime(&naive_dt)
                .single()
                .ok_or_else(|| Error::custom("Ambiguous datetime in Panama timezone"))?;
            return Ok(panama_dt.with_timezone(&Utc));
        }
    }
    
    Err(Error::custom(format!("Unable to parse datetime: {}", s)))
}

/// Deserializa una fecha opcional desde timezone de Panamá
fn deserialize_panama_datetime_option<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    match opt {
        Some(s) if !s.is_empty() => {
            // Usar el deserializador principal
            deserialize_panama_datetime(serde::de::value::StringDeserializer::new(s))
                .map(Some)
        },
        _ => Ok(None),
    }
}

// ============================================================================
// CURSOR PAGINATION UTILITIES
// ============================================================================

/// Estructura para manejar cursor keyset (múltiples campos)
#[derive(Debug, Clone)]
pub struct CursorPosition {
    pub reception_date: Option<DateTime<Utc>>,
    pub date: Option<DateTime<Utc>>,
    pub amount: Option<rust_decimal::Decimal>,
    pub id: Option<i64>,
}

impl CursorPosition {
    /// Crear nueva posición de cursor
    pub fn new(
        date: Option<DateTime<Utc>>,
        amount: Option<rust_decimal::Decimal>,
        id: Option<i64>,
        reception_date: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            reception_date,
            date,
            amount,
            id,
        }
    }

    /// Codifica la posición en un string base64
    pub fn encode(&self) -> String {
        let mut parts = Vec::new();
        
        if let Some(date) = self.date {
            parts.push(format!("date:{}", date.format("%Y-%m-%dT%H:%M:%SZ")));
        }
        
        if let Some(amount) = self.amount {
            parts.push(format!("amount:{}", amount));
        }
        
        if let Some(id) = self.id {
            parts.push(format!("id:{}", id));
        }
        
        if let Some(reception_date) = self.reception_date {
            parts.push(format!("reception_date:{}", reception_date.format("%Y-%m-%dT%H:%M:%SZ")));
        }
        
        let cursor_str = parts.join("|");
        base64::engine::general_purpose::STANDARD.encode(&cursor_str)
    }
    
    /// Decodifica un cursor base64 a posición
    pub fn decode(cursor: &str) -> Result<Self, String> {
        let decoded = base64::engine::general_purpose::STANDARD.decode(cursor)
            .map_err(|e| format!("Base64 decode error: {}", e))?;
            
        let cursor_str = String::from_utf8(decoded)
            .map_err(|e| format!("UTF8 decode error: {}", e))?;
            
        let mut reception_date = None;
        let mut date = None;
        let mut amount = None;
        let mut id = None;
        
        for part in cursor_str.split('|') {
            if let Some((key, value)) = part.split_once(':') {
                match key {
                    "date" => {
                        date = Some(DateTime::parse_from_rfc3339(value)
                            .map_err(|e| format!("Date parse error: {}", e))?
                            .with_timezone(&Utc));
                    },
                    "amount" => {
                        amount = Some(value.parse::<rust_decimal::Decimal>()
                            .map_err(|e| format!("Amount parse error: {}", e))?);
                    },
                    "id" => {
                        id = Some(value.parse::<i64>()
                            .map_err(|e| format!("ID parse error: {}", e))?);
                    },
                    "reception_date" => {
                        reception_date = Some(DateTime::parse_from_rfc3339(value)
                            .map_err(|e| format!("Reception date parse error: {}", e))?
                            .with_timezone(&Utc));
                    },
                    _ => {} // Ignore unknown keys
                }
            }
        }
        
        Ok(Self {
            reception_date,
            date,
            amount,
            id,
        })
    }
    
    /// Crea un cursor desde un item de invoice
    pub fn from_invoice_item(item: &InvoiceDetailItem) -> Self {
        CursorPosition {
            reception_date: item.reception_date,
            date: item.date.map(|d| d.and_utc()),
            amount: item.amount.map(|a| rust_decimal::Decimal::from_f64_retain(a).unwrap_or_default()),
            id: Some(item.id),
        }
    }
}
