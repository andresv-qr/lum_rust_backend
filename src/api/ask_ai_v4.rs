// ============================================================================
// ASK AI v4 - Natural Language to SQL Query Endpoint
// ============================================================================
// Fecha: 2026-01-04
// Descripción: Endpoint para consultas en lenguaje natural sobre datos de usuario
// Autor: LümAI Team

use axum::{
    extract::State,
    http::HeaderMap,
    Json,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::{Arc, LazyLock};
use std::time::Duration;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

use crate::api::common::{ApiResponse, ApiError};
use crate::state::AppState;

// ============================================================================
// CONSTANTS
// ============================================================================
const MAX_QUESTION_LENGTH: usize = 1000;
const MIN_QUESTION_LENGTH: usize = 3;
const OPENROUTER_TIMEOUT_SECS: u64 = 30;
const DEFAULT_MODEL: &str = "deepseek/deepseek-v3.2";

/// Reusable HTTP client for performance (connection pooling)
pub static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(OPENROUTER_TIMEOUT_SECS))
        .pool_max_idle_per_host(5)
        .build()
        .expect("Failed to create HTTP client")
});

// ============================================================================
// REQUEST/RESPONSE STRUCTS
// ============================================================================

/// Request body for ask-ai endpoint
#[derive(Debug, Deserialize)]
pub struct AskAiRequest {
    /// Natural language question about user's data
    pub question: String,
}

/// Response from AI with SQL query and chart configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AskAiResponse {
    /// Brief explanation of the analysis in Spanish
    pub explanation: String,
    /// SQLite query to execute on client's local database
    pub sql_query: String,
    /// Type of chart to render (barChart, lineChart, pieChart, etc.)
    pub chart_type: String,
    /// Configuration for the chart
    pub chart_config: ChartConfig,
}

/// Chart configuration for Flutter fl_chart library
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChartConfig {
    /// Label for X axis (categories, dates, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub x_axis_label: Option<String>,
    /// Label for Y axis (amounts, counts, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub y_axis_label: Option<String>,
    /// Field name from SQL result to use for X axis
    pub x_field: String,
    /// Field name from SQL result to use for Y axis
    pub y_field: String,
    /// Optional: field for grouping/stacking
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_field: Option<String>,
    /// Color scheme suggestion (blue, green, orange, purple)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_scheme: Option<String>,
    /// Additional properties for specific chart types
    #[serde(flatten)]
    pub extra: Option<Value>,
}

// ============================================================================
// OPENROUTER API STRUCTS (Internal)
// ============================================================================

#[derive(Serialize)]
struct OpenRouterRequest {
    model: String,
    messages: Vec<OpenRouterMessage>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Serialize)]
struct OpenRouterMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenRouterResponse {
    choices: Vec<OpenRouterChoice>,
    usage: Option<OpenRouterUsage>,
}

#[derive(Deserialize)]
struct OpenRouterChoice {
    message: OpenRouterMessageContent,
}

#[derive(Deserialize)]
struct OpenRouterMessageContent {
    content: String,
}

#[derive(Deserialize, Default)]
struct OpenRouterUsage {
    prompt_tokens: i32,
    completion_tokens: i32,
    total_tokens: i32,
}

// ============================================================================
// SYSTEM PROMPT
// ============================================================================

const SYSTEM_PROMPT: &str = r#"You are an expert data analyst for the LümAI personal finance platform in Panama.
Your task is to translate natural language questions (in Spanish) into SQLite queries and recommend visualizations.

The user has their invoice data stored locally on their mobile device. You generate the SQL query that their app will execute.

## DATABASE SCHEMA (SQLite)

### invoices
Primary table for purchase receipts.
| Column | Type | Description |
|--------|------|-------------|
| cufe | TEXT PK | Unique fiscal identifier (CUFE) |
| issuer_name | TEXT | Legal business name |
| issuer_ruc | TEXT | Tax ID (RUC) of merchant |
| store_id | TEXT | Branch/store identifier |
| date | TEXT | Purchase date (YYYY-MM-DD) |
| tot_amount | REAL | Total amount paid in USD |
| tot_itbms | REAL | ITBMS tax (7% Panama) |
| user_id | INTEGER | Owner user ID |

### invoice_details
Line items within each invoice.
| Column | Type | Description |
|--------|------|-------------|
| id | INTEGER PK | Auto-increment |
| cufe | TEXT FK | → invoices.cufe |
| description | TEXT | Product/service description |
| code | TEXT | Product SKU/code |
| quantity | REAL | Quantity purchased |
| unit_price | REAL | Price per unit (USD) |
| total_amount | REAL | Line total (qty × price) |
| discount | REAL | Discount applied |

### issuers
Merchant/store master data with categories.
| Column | Type | Description |
|--------|------|-------------|
| issuer_ruc | TEXT | Tax ID (composite PK) |
| store_id | TEXT | Branch ID (composite PK) |
| brand_name | TEXT | Commercial brand name (preferred for display) |
| l1 | TEXT | Category L1: Supermercado, Farmacia, Restaurante, Gasolinera, Tienda, Otro |
| l2 | TEXT | Category L2: subcategory |

### products
Product catalog with categories.
| Column | Type | Description |
|--------|------|-------------|
| code_cleaned | TEXT | Normalized product code |
| issuer_ruc | TEXT | Merchant tax ID |
| description | TEXT | Cleaned product name |
| l1 | TEXT | Category L1: Alimentos, Bebidas, Hogar, Salud, Cuidado Personal, Otro |
| l2 | TEXT | Category L2: subcategory |

## JOIN PATTERNS (always use these)
```sql
-- Invoices + Details
FROM invoice_details d
JOIN invoices i ON d.cufe = i.cufe

-- Invoices + Merchant info
FROM invoices i
LEFT JOIN issuers iss ON i.issuer_ruc = iss.issuer_ruc AND i.store_id = iss.store_id

-- Full product analysis
FROM invoice_details d
JOIN invoices i ON d.cufe = i.cufe
LEFT JOIN issuers iss ON i.issuer_ruc = iss.issuer_ruc AND i.store_id = iss.store_id
LEFT JOIN products p ON d.code = p.code_cleaned AND i.issuer_ruc = p.issuer_ruc
```

## CHART SELECTION GUIDE

| Question Type | chart_type | When to use |
|---------------|------------|-------------|
| Trend over time | lineChart | Monthly/weekly spending evolution |
| Compare categories | barChart | Top 5-10 merchants, categories |
| Long labels | horizontalBarChart | Merchant names, product names |
| Parts of whole | pieChart / donut | Distribution by category (≤6 segments) |
| Single metrics | kpiCards | Total spent, average, count |
| Detailed data | dataTable | Product lists, transaction history |
| Grouped comparison | stackedBarChart | Category breakdown by month |
| Cumulative | areaChart | Running total over time |

## OUTPUT FORMAT (strict JSON only)
```json
{
  "explanation": "Explicación breve en español de lo que muestra el análisis",
  "sql_query": "SELECT ... (valid SQLite)",
  "chart_type": "barChart|lineChart|pieChart|donut|horizontalBarChart|stackedBarChart|areaChart|kpiCards|dataTable",
  "chart_config": {
    "x_axis_label": "Etiqueta eje X (opcional)",
    "y_axis_label": "Etiqueta eje Y (opcional)",
    "x_field": "nombre_columna_sql",
    "y_field": "nombre_columna_sql",
    "group_field": null,
    "color_scheme": "blue"
  }
}
```

## FEW-SHOT EXAMPLES

### Example 1: Monthly spending trend
User: "¿Cuánto gasté cada mes?"
```json
{
  "explanation": "Gasto total por mes en los últimos 12 meses, mostrando la tendencia de consumo.",
  "sql_query": "SELECT strftime('%Y-%m', date) AS month, ROUND(SUM(tot_amount), 2) AS total FROM invoices WHERE date >= date('now', '-12 months') GROUP BY month ORDER BY month",
  "chart_type": "lineChart",
  "chart_config": {
    "x_axis_label": "Mes",
    "y_axis_label": "Gasto ($)",
    "x_field": "month",
    "y_field": "total",
    "group_field": null,
    "color_scheme": "blue"
  }
}
```

### Example 2: Top merchants
User: "¿Dónde gasto más dinero?"
```json
{
  "explanation": "Los 10 comercios donde más has gastado, ordenados de mayor a menor.",
  "sql_query": "SELECT COALESCE(iss.brand_name, i.issuer_name) AS merchant, ROUND(SUM(i.tot_amount), 2) AS total FROM invoices i LEFT JOIN issuers iss ON i.issuer_ruc = iss.issuer_ruc AND i.store_id = iss.store_id GROUP BY merchant ORDER BY total DESC LIMIT 10",
  "chart_type": "horizontalBarChart",
  "chart_config": {
    "x_axis_label": "Gasto Total ($)",
    "y_axis_label": "Comercio",
    "x_field": "total",
    "y_field": "merchant",
    "group_field": null,
    "color_scheme": "green"
  }
}
```

### Example 3: Category distribution
User: "Distribución de gastos por categoría"
```json
{
  "explanation": "Distribución porcentual de tus gastos según el tipo de comercio.",
  "sql_query": "SELECT COALESCE(iss.l1, 'Sin categoría') AS category, ROUND(SUM(i.tot_amount), 2) AS total FROM invoices i LEFT JOIN issuers iss ON i.issuer_ruc = iss.issuer_ruc AND i.store_id = iss.store_id GROUP BY category ORDER BY total DESC LIMIT 6",
  "chart_type": "donut",
  "chart_config": {
    "x_field": "category",
    "y_field": "total",
    "group_field": null,
    "color_scheme": "purple"
  }
}
```

### Example 4: KPI summary
User: "¿Cuánto he gastado en total?"
```json
{
  "explanation": "Resumen de tu gasto total acumulado.",
  "sql_query": "SELECT ROUND(SUM(tot_amount), 2) AS total_gastado, COUNT(*) AS num_facturas, ROUND(AVG(tot_amount), 2) AS promedio FROM invoices",
  "chart_type": "kpiCards",
  "chart_config": {
    "x_field": "metric",
    "y_field": "value",
    "group_field": null,
    "color_scheme": "blue"
  }
}
```

## STRICT RULES

### DO:
1. Use COALESCE(iss.brand_name, i.issuer_name) AS merchant for merchant names
2. Use strftime('%Y-%m', date) for monthly grouping
3. Use strftime('%Y', date) for yearly grouping  
4. Use table aliases: i=invoices, d=invoice_details, iss=issuers, p=products
5. Always include ORDER BY clause
6. Use ROUND(value, 2) for monetary amounts
7. Use LIMIT: 12 for time series, 10 for rankings, 6 for pie charts, 100 for tables
8. Write explanation in Spanish
9. Return ONLY the JSON object, nothing else

### DON'T:
1. Never use DATE_FORMAT() - use strftime()
2. Never use MySQL/PostgreSQL specific functions
3. Never return markdown code blocks (no ```)
4. Never add text before or after the JSON
5. Never use CURRENT_DATE - use date('now')
6. Never assume data exists - use LEFT JOIN for optional tables
7. Never return more than 12 segments for pie/donut charts

### EDGE CASES:
- If question is unclear: ask for clarification via explanation field, still provide best-guess query
- If no data might exist: the query should still be valid (will return empty results)
- If question is not about invoices/spending: politely redirect in explanation"#;

// ============================================================================
// HANDLER
// ============================================================================

/// Ask AI endpoint - Translates natural language to SQL + chart config
/// 
/// POST /api/v4/ask-ai
/// 
/// Requires JWT authentication
pub async fn ask_ai_data(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<AskAiRequest>,
) -> Result<Json<ApiResponse<AskAiResponse>>, ApiError> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let start_time = std::time::Instant::now();
    
    debug!(request_id = %request_id, "🤖 AI Ask endpoint called");

    // ========================================================================
    // AUTHENTICATION - Extract from header since middleware already validated
    // ========================================================================
    let current_user = crate::middleware::auth::extract_user_from_headers(&headers)
        .map_err(|(_status, json_error)| {
            warn!(request_id = %request_id, "Authentication failed");
            ApiError::unauthorized(&json_error.0.message)
        })?;
    
    info!(
        request_id = %request_id,
        user_id = current_user.user_id,
        "🔐 User authenticated for AI query"
    );

    // ========================================================================
    // VALIDATION
    // ========================================================================
    let question = payload.question.trim();
    
    if question.len() < MIN_QUESTION_LENGTH {
        return Err(ApiError::validation_error(
            &format!("La pregunta debe tener al menos {} caracteres", MIN_QUESTION_LENGTH)
        ));
    }
    
    if question.len() > MAX_QUESTION_LENGTH {
        return Err(ApiError::validation_error(
            &format!("La pregunta no puede exceder {} caracteres", MAX_QUESTION_LENGTH)
        ));
    }

    debug!(
        request_id = %request_id,
        question_length = question.len(),
        "📝 Question validated"
    );

    // ========================================================================
    // OPENROUTER API CALL
    // ========================================================================
    let api_key = std::env::var("OPENROUTER_API_KEY")
        .map_err(|_| {
            error!(request_id = %request_id, "OPENROUTER_API_KEY not configured");
            ApiError::internal_server_error("Servicio de IA no configurado")
        })?;
    
    if api_key.is_empty() {
        error!(request_id = %request_id, "OPENROUTER_API_KEY is empty");
        return Err(ApiError::internal_server_error("Servicio de IA no configurado"));
    }

    let open_router_req = OpenRouterRequest {
        model: DEFAULT_MODEL.to_string(),
        messages: vec![
            OpenRouterMessage {
                role: "system".to_string(),
                content: SYSTEM_PROMPT.to_string(),
            },
            OpenRouterMessage {
                role: "user".to_string(),
                content: question.to_string(),
            },
        ],
        temperature: 0.0,  // Zero temperature for deterministic SQL generation
        max_tokens: 1024,
    };

    debug!(request_id = %request_id, model = DEFAULT_MODEL, "📤 Calling OpenRouter API");

    let res = HTTP_CLIENT
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("HTTP-Referer", "https://lumapp.ai")
        .header("X-Title", "LumAI Data Assistant")
        .json(&open_router_req)
        .send()
        .await
        .map_err(|e| {
            error!(request_id = %request_id, error = %e, "OpenRouter request failed");
            if e.is_timeout() {
                ApiError::new("AI_TIMEOUT", "El servicio de IA tardó demasiado en responder. Intenta de nuevo.")
            } else if e.is_connect() {
                ApiError::new("AI_CONNECTION_ERROR", "No se pudo conectar al servicio de IA")
            } else {
                ApiError::new("AI_REQUEST_ERROR", "Error al comunicarse con el servicio de IA")
            }
        })?;

    let status = res.status();
    if !status.is_success() {
        let error_text = res.text().await.unwrap_or_default();
        error!(
            request_id = %request_id,
            status = %status,
            error = %error_text,
            "OpenRouter API error"
        );
        
        let error_msg = match status.as_u16() {
            401 => "API key inválida para el servicio de IA",
            429 => "Demasiadas solicitudes. Intenta en unos segundos.",
            500..=599 => "El servicio de IA está temporalmente no disponible",
            _ => "Error del servicio de IA",
        };
        
        return Err(ApiError::new("AI_SERVICE_ERROR", error_msg));
    }

    let open_router_res: OpenRouterResponse = res.json().await
        .map_err(|e| {
            error!(request_id = %request_id, error = %e, "Failed to parse OpenRouter response");
            ApiError::new("AI_PARSE_ERROR", "Error al procesar respuesta de IA")
        })?;

    // ========================================================================
    // PARSE AI RESPONSE
    // ========================================================================
    let content = open_router_res.choices.first()
        .map(|c| c.message.content.clone())
        .unwrap_or_default();

    if content.is_empty() {
        error!(request_id = %request_id, "Empty response from AI");
        return Err(ApiError::new("AI_EMPTY_RESPONSE", "La IA no generó una respuesta. Intenta reformular tu pregunta."));
    }

    // Clean up markdown code blocks if present
    let clean_content = content.trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let ai_response: AskAiResponse = serde_json::from_str(clean_content)
        .map_err(|e| {
            error!(
                request_id = %request_id,
                error = %e,
                content = %clean_content,
                "Failed to parse AI JSON response"
            );
            ApiError::new(
                "AI_RESPONSE_INVALID",
                "La respuesta de la IA no tiene el formato esperado. Intenta reformular tu pregunta."
            )
        })?;

    debug!(
        request_id = %request_id,
        chart_type = %ai_response.chart_type,
        "✅ AI response parsed successfully"
    );

    // ========================================================================
    // LOG USAGE TO DATABASE
    // ========================================================================
    let usage = open_router_res.usage.unwrap_or_default();
    let cost = Decimal::ZERO; // Free model, no cost

    let log_result = sqlx::query!(
        r#"INSERT INTO public.ai_askai_logs 
           (user_id, question, prompt_tokens, completion_tokens, total_tokens, cost, model) 
           VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
        current_user.user_id,
        question,
        usage.prompt_tokens,
        usage.completion_tokens,
        usage.total_tokens,
        cost,
        DEFAULT_MODEL
    )
    .execute(&state.db_pool)
    .await;

    if let Err(e) = log_result {
        warn!(request_id = %request_id, error = %e, "Failed to log AI usage (non-critical)");
        // Don't fail the request, just log the warning
    }

    let execution_time = start_time.elapsed().as_millis() as u64;
    
    info!(
        request_id = %request_id,
        user_id = current_user.user_id,
        execution_time_ms = execution_time,
        tokens = usage.total_tokens,
        "🎯 AI query completed successfully"
    );

    Ok(Json(ApiResponse::success(ai_response, request_id, Some(execution_time), false)))
}
