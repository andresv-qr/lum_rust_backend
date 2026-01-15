// ============================================================================
// INTERPRET RESULTS v4 - AI Interpretation of Query Results
// ============================================================================
// Fecha: 2026-01-04
// Descripción: Endpoint para interpretar resultados de queries ejecutados localmente
// Autor: LümAI Team

use axum::{
    extract::State,
    http::HeaderMap,
    Json,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tracing::{info, warn, error, debug};
use uuid::Uuid;

use crate::api::common::{ApiResponse, ApiError};
use crate::state::AppState;

// ============================================================================
// CONSTANTS
// ============================================================================
const MAX_ROWS: usize = 50;
const MAX_QUESTION_LENGTH: usize = 500;
const MAX_PAYLOAD_SIZE: usize = 10 * 1024; // 10KB
const DEFAULT_MODEL: &str = "deepseek/deepseek-v3.2";

// Reuse HTTP client from ask_ai_v4
use super::ask_ai_v4::HTTP_CLIENT;

// ============================================================================
// REQUEST/RESPONSE STRUCTS
// ============================================================================

/// Request body for interpret-results endpoint
#[derive(Debug, Deserialize)]
pub struct InterpretResultsRequest {
    /// Original question asked by the user
    pub question: String,
    /// Data returned from local SQL query execution (max 50 rows)
    pub data: Vec<Value>,
    /// Type of chart being displayed
    pub chart_type: String,
    /// Optional: column names for context
    #[serde(default)]
    pub columns: Option<Vec<String>>,
}

/// Response with AI interpretation
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InterpretResultsResponse {
    /// Main interpretation text (supports markdown)
    pub interpretation: String,
    /// Key insights extracted from the data
    pub insights: Vec<String>,
    /// Suggested actions or recommendations
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggested_actions: Option<Vec<String>>,
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
// SYSTEM PROMPT FOR INTERPRETATION
// ============================================================================

const INTERPRETATION_PROMPT: &str = r#"You are a friendly financial advisor assistant for LümAI, a personal finance app in Panama.

Your task is to analyze query results from a user's spending data and provide helpful, actionable insights.

## INPUT FORMAT
You will receive:
1. The user's original question (in Spanish)
2. The data results (JSON array)
3. The chart type being displayed

## OUTPUT FORMAT (strict JSON)
```json
{
  "interpretation": "Main analysis text in Spanish with **bold** for important numbers. 2-3 sentences max.",
  "insights": [
    "Key insight 1 (short, actionable)",
    "Key insight 2",
    "Key insight 3"
  ],
  "suggested_actions": [
    "Optional suggestion 1",
    "Optional suggestion 2"
  ]
}
```

## RULES
1. Write ALL text in Spanish (Panama)
2. Use **bold** markdown for important numbers and percentages
3. Be concise: interpretation should be 2-3 sentences max
4. Provide 2-4 insights maximum
5. suggested_actions is optional - only include if genuinely useful
6. Use currency format: $X,XXX.XX (USD)
7. Calculate percentages, trends, and comparisons when data allows
8. Be encouraging and helpful, not judgmental about spending
9. If data is empty, acknowledge it kindly and suggest checking the query
10. Return ONLY the JSON object, no markdown code blocks

## EXAMPLES

### Example 1: Monthly trend
Question: "¿Cuánto gasté cada mes?"
Data: [{"month":"2026-01","total":1200},{"month":"2025-12","total":1450}]
Chart: lineChart
```json
{
  "interpretation": "En enero 2026 gastaste **$1,200**, un **17% menos** que en diciembre ($1,450). ¡Buen trabajo controlando tus gastos!",
  "insights": [
    "Tendencia descendente: -$250 vs mes anterior",
    "Promedio mensual: $1,325"
  ],
  "suggested_actions": null
}
```

### Example 2: Top merchants
Question: "¿Dónde gasto más?"
Data: [{"merchant":"Super 99","total":450},{"merchant":"Rey","total":320}]
Chart: horizontalBarChart
```json
{
  "interpretation": "Tu comercio principal es **Super 99** con **$450** (58% del total). Rey ocupa el segundo lugar con $320.",
  "insights": [
    "Super 99 domina tu gasto en supermercados",
    "Concentración alta: 2 comercios = 100% del gasto mostrado"
  ],
  "suggested_actions": [
    "Compara precios entre Super 99 y Rey para los mismos productos"
  ]
}
```

### Example 3: Empty results
Question: "¿Cuánto gasté en restaurantes?"
Data: []
Chart: barChart
```json
{
  "interpretation": "No encontré registros de gastos en restaurantes en tus datos. Esto puede significar que no has registrado facturas de restaurantes o que la categoría está clasificada de otra forma.",
  "insights": [
    "Sin datos para el período consultado"
  ],
  "suggested_actions": [
    "Verifica si tienes facturas de restaurantes escaneadas",
    "Intenta buscar por nombre específico del restaurante"
  ]
}
```"#;

// ============================================================================
// HANDLER
// ============================================================================

/// Interpret Results endpoint - Analyzes query results with AI
/// 
/// POST /api/v4/interpret-results
/// 
/// Requires JWT authentication
pub async fn interpret_results(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<InterpretResultsRequest>,
) -> Result<Json<ApiResponse<InterpretResultsResponse>>, ApiError> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let start_time = std::time::Instant::now();
    
    debug!(request_id = %request_id, "📊 Interpret Results endpoint called");

    // ========================================================================
    // AUTHENTICATION
    // ========================================================================
    let current_user = crate::middleware::auth::extract_user_from_headers(&headers)
        .map_err(|(_status, json_error)| {
            warn!(request_id = %request_id, "Authentication failed");
            ApiError::unauthorized(&json_error.0.message)
        })?;
    
    info!(
        request_id = %request_id,
        user_id = current_user.user_id,
        "🔐 User authenticated for interpretation"
    );

    // ========================================================================
    // VALIDATION
    // ========================================================================
    let question = payload.question.trim();
    
    if question.is_empty() {
        return Err(ApiError::validation_error("La pregunta no puede estar vacía"));
    }
    
    if question.len() > MAX_QUESTION_LENGTH {
        return Err(ApiError::validation_error(
            &format!("La pregunta no puede exceder {} caracteres", MAX_QUESTION_LENGTH)
        ));
    }

    if payload.data.len() > MAX_ROWS {
        return Err(ApiError::validation_error(
            &format!("Los datos no pueden exceder {} filas", MAX_ROWS)
        ));
    }

    // Validate payload size
    let payload_json = serde_json::to_string(&payload.data).unwrap_or_default();
    if payload_json.len() > MAX_PAYLOAD_SIZE {
        return Err(ApiError::validation_error(
            &format!("El tamaño de los datos no puede exceder {}KB", MAX_PAYLOAD_SIZE / 1024)
        ));
    }

    debug!(
        request_id = %request_id,
        data_rows = payload.data.len(),
        chart_type = %payload.chart_type,
        "📝 Request validated"
    );

    // ========================================================================
    // BUILD USER MESSAGE
    // ========================================================================
    let user_message = format!(
        "Question: {}\nData: {}\nChart type: {}",
        question,
        serde_json::to_string(&payload.data).unwrap_or_else(|_| "[]".to_string()),
        payload.chart_type
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
                content: INTERPRETATION_PROMPT.to_string(),
            },
            OpenRouterMessage {
                role: "user".to_string(),
                content: user_message,
            },
        ],
        temperature: 0.3,  // Slightly higher for more natural interpretation
        max_tokens: 512,   // Interpretations are shorter than SQL generation
    };

    debug!(request_id = %request_id, model = DEFAULT_MODEL, "📤 Calling OpenRouter API for interpretation");

    let res = HTTP_CLIENT
        .post("https://openrouter.ai/api/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("HTTP-Referer", "https://lumapp.ai")
        .header("X-Title", "LumAI Data Interpreter")
        .json(&open_router_req)
        .send()
        .await
        .map_err(|e| {
            error!(request_id = %request_id, error = %e, "OpenRouter request failed");
            if e.is_timeout() {
                ApiError::new("AI_TIMEOUT", "El servicio de IA tardó demasiado en responder")
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
        return Err(ApiError::new("AI_EMPTY_RESPONSE", "La IA no generó una respuesta"));
    }

    // Clean up markdown code blocks if present
    let clean_content = content.trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let interpretation: InterpretResultsResponse = serde_json::from_str(clean_content)
        .map_err(|e| {
            error!(
                request_id = %request_id,
                error = %e,
                content = %clean_content,
                "Failed to parse AI interpretation response"
            );
            ApiError::new(
                "AI_RESPONSE_INVALID",
                "La respuesta de la IA no tiene el formato esperado"
            )
        })?;

    debug!(
        request_id = %request_id,
        insights_count = interpretation.insights.len(),
        "✅ Interpretation parsed successfully"
    );

    // ========================================================================
    // LOG USAGE TO DATABASE
    // ========================================================================
    let usage = open_router_res.usage.unwrap_or_default();
    let cost = Decimal::ZERO; // Free model

    let log_result = sqlx::query!(
        r#"INSERT INTO public.ai_askai_logs 
           (user_id, question, prompt_tokens, completion_tokens, total_tokens, cost, model) 
           VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
        current_user.user_id,
        format!("[INTERPRET] {}", question),
        usage.prompt_tokens,
        usage.completion_tokens,
        usage.total_tokens,
        cost,
        DEFAULT_MODEL
    )
    .execute(&state.db_pool)
    .await;

    if let Err(e) = log_result {
        warn!(request_id = %request_id, error = %e, "Failed to log interpretation usage (non-critical)");
    }

    let execution_time = start_time.elapsed().as_millis() as u64;
    
    info!(
        request_id = %request_id,
        user_id = current_user.user_id,
        execution_time_ms = execution_time,
        tokens = usage.total_tokens,
        "🎯 Interpretation completed successfully"
    );

    Ok(Json(ApiResponse::success(interpretation, request_id, Some(execution_time), false)))
}
