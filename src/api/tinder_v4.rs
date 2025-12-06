use axum::{
    extract::{State, Request, Query},
    http::HeaderMap,
    routing::{get, post},
    Router,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, types::Json as SqlxJson};
use std::sync::Arc;
use uuid::Uuid;
use chrono::{DateTime, Utc, NaiveDate};
use tracing::{info, error, debug};

use crate::state::AppState;
use crate::api::common::{ApiResponse, ApiError};
use crate::middleware::auth::{get_current_user_from_request, extract_user_from_headers};

// --- Models ---

#[derive(Debug, Serialize, FromRow)]
pub struct TinderQuestion {
    pub id: Uuid,
    pub title: String,
    pub image_url: Option<String>,
    pub priority: i32,
    pub targeting_rules: SqlxJson<serde_json::Value>,
    pub specific_date: Option<NaiveDate>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct TinderOption {
    pub id: Uuid,
    pub question_id: Uuid,
    pub label: Option<String>,
    pub image_url: Option<String>,
    pub icon_url: Option<String>,
    pub display_order: i32,
}

#[derive(Debug, Serialize)]
pub struct QuestionWithOptions {
    #[serde(flatten)]
    pub question: TinderQuestion,
    pub options: Vec<TinderOption>,
}

#[derive(Debug, Deserialize)]
pub struct SubmitAnswerRequest {
    pub question_id: Uuid,
    pub option_id: Uuid,
}

/// Query parameters for GET /questions
#[derive(Debug, Deserialize)]
pub struct GetQuestionsParams {
    /// Number of questions to return (default: 20, max: 50)
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_limit() -> u32 { 20 }

// Helper struct for user context
struct UserContext {
    user_id: i64,
    age: Option<i32>,
    country: Option<String>,
    tags: Vec<String>,
}

// --- Logic ---

fn calculate_age(dob_str: &Option<String>) -> Option<i32> {
    let dob_str = dob_str.as_ref()?;
    // Try parsing YYYY-MM-DD
    let dob = NaiveDate::parse_from_str(dob_str, "%Y-%m-%d").ok()?;
    let now = Utc::now().date_naive();
    let age = now.years_since(dob).unwrap_or(0);
    Some(age as i32)
}

fn matches_targeting(rules: &serde_json::Value, context: &UserContext, specific_date: &Option<NaiveDate>) -> bool {
    // Check specific_date first (column-level, not in JSON)
    if let Some(target_date) = specific_date {
        let today = Utc::now().date_naive();
        if *target_date != today {
            return false;
        }
    }

    let rules_obj = match rules.as_object() {
        Some(obj) => obj,
        None => return true, // No rules = match all
    };

    // 1. User IDs Check (specific users)
    if let Some(user_ids) = rules_obj.get("user_ids").and_then(|v| v.as_array()) {
        let allowed: Vec<i64> = user_ids.iter().filter_map(|v| v.as_i64()).collect();
        if !allowed.is_empty() && !allowed.contains(&(context.user_id as i64)) {
            return false;
        }
    }

    // 2. Age Check
    if let Some(min_age) = rules_obj.get("min_age").and_then(|v| v.as_i64()) {
        if let Some(age) = context.age {
            if (age as i64) < min_age { return false; }
        } else {
            return false; // Age required but unknown
        }
    }
    if let Some(max_age) = rules_obj.get("max_age").and_then(|v| v.as_i64()) {
        if let Some(age) = context.age {
            if (age as i64) > max_age { return false; }
        }
    }

    // 3. Country Check
    if let Some(countries) = rules_obj.get("countries").and_then(|v| v.as_array()) {
        let user_country = context.country.as_deref().unwrap_or("");
        let allowed: Vec<&str> = countries.iter().filter_map(|v| v.as_str()).collect();
        if !allowed.is_empty() && !allowed.contains(&user_country) {
            return false;
        }
    }

    // 4. Tags Check (The "Engine" part)
    // required_tags: User MUST have ALL these tags
    if let Some(required) = rules_obj.get("required_tags").and_then(|v| v.as_array()) {
        for tag_val in required {
            if let Some(tag) = tag_val.as_str() {
                if !context.tags.contains(&tag.to_string()) {
                    return false;
                }
            }
        }
    }

    // any_tags: User MUST have AT LEAST ONE of these tags
    if let Some(any) = rules_obj.get("any_tags").and_then(|v| v.as_array()) {
        let mut found = false;
        for tag_val in any {
            if let Some(tag) = tag_val.as_str() {
                if context.tags.contains(&tag.to_string()) {
                    found = true;
                    break;
                }
            }
        }
        if !found && !any.is_empty() { return false; }
    }

    // excluded_tags: User MUST NOT have ANY of these tags
    if let Some(excluded) = rules_obj.get("excluded_tags").and_then(|v| v.as_array()) {
        for tag_val in excluded {
            if let Some(tag) = tag_val.as_str() {
                if context.tags.contains(&tag.to_string()) {
                    return false;
                }
            }
        }
    }

    // 5. Product-based targeting (via tags)
    // product_codes -> tags like "product_code:ABC123"
    if !check_prefixed_tags(rules_obj, "product_codes", "product_code:", &context.tags) { return false; }
    if !check_prefixed_tags(rules_obj, "product_l1", "product_l1:", &context.tags) { return false; }
    if !check_prefixed_tags(rules_obj, "product_l2", "product_l2:", &context.tags) { return false; }
    if !check_prefixed_tags(rules_obj, "product_l3", "product_l3:", &context.tags) { return false; }
    if !check_prefixed_tags(rules_obj, "product_l4", "product_l4:", &context.tags) { return false; }
    if !check_prefixed_tags(rules_obj, "product_brands", "product_brand:", &context.tags) { return false; }

    // 6. Issuer/Merchant-based targeting (via tags)
    if !check_prefixed_tags(rules_obj, "issuer_rucs", "issuer_ruc:", &context.tags) { return false; }
    if !check_prefixed_tags(rules_obj, "issuer_brand_names", "issuer_brand_name:", &context.tags) { return false; }
    if !check_prefixed_tags(rules_obj, "issuer_store_names", "issuer_store_name:", &context.tags) { return false; }
    if !check_prefixed_tags(rules_obj, "issuer_l1", "issuer_l1:", &context.tags) { return false; }
    if !check_prefixed_tags(rules_obj, "issuer_l2", "issuer_l2:", &context.tags) { return false; }
    if !check_prefixed_tags(rules_obj, "issuer_l3", "issuer_l3:", &context.tags) { return false; }
    if !check_prefixed_tags(rules_obj, "issuer_l4", "issuer_l4:", &context.tags) { return false; }

    true
}

/// Helper function: Check if user has at least one tag matching the pattern.
/// Example: rule_key="product_codes", prefix="product_code:", rule_values=["ABC", "XYZ"]
/// Checks if user has tag "product_code:ABC" OR "product_code:XYZ"
fn check_prefixed_tags(
    rules_obj: &serde_json::Map<String, serde_json::Value>,
    rule_key: &str,
    prefix: &str,
    user_tags: &[String],
) -> bool {
    if let Some(values) = rules_obj.get(rule_key).and_then(|v| v.as_array()) {
        if values.is_empty() { return true; }
        
        // User must have at least ONE of these (OR logic)
        for val in values {
            if let Some(v) = val.as_str() {
                let expected_tag = format!("{}{}", prefix, v.to_lowercase());
                if user_tags.iter().any(|t| t.to_lowercase() == expected_tag) {
                    return true;
                }
            }
        }
        return false; // Had rules but no match
    }
    true // No rule for this key = pass
}

// --- Handlers ---

/// Get pending questions for the current user
pub async fn get_pending_questions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(params): Query<GetQuestionsParams>,
    request: Request,
) -> Result<Json<ApiResponse<Vec<QuestionWithOptions>>>, ApiError> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
    
    let start_time = std::time::Instant::now();

    // Clamp limit to valid range (1-50)
    let requested_limit = params.limit.clamp(1, 50) as usize;
    // Over-fetch 3x to ensure enough questions after targeting filter
    let overfetch_limit = (requested_limit * 3).min(150) as i64;

    // 1. Get current user
    let current_user = get_current_user_from_request(&request)
        .map_err(|(_status, json_error)| {
            ApiError::new("UNAUTHORIZED", &json_error.0.message)
        })?;

    let user_id = current_user.user_id;

    // 2. Fetch User Context (Profile + Tags) IN PARALLEL
    // This reduces latency by ~30% compared to sequential fetching
    let profile_future = sqlx::query!(
        "SELECT date_of_birth, country_residence FROM dim_users WHERE id = $1",
        user_id as i64
    )
    .fetch_optional(&state.db_pool);

    let tags_future = sqlx::query_scalar!(
        "SELECT tag FROM lumimatch.user_tags WHERE user_id = $1",
        user_id as i32
    )
    .fetch_all(&state.db_pool);

    // Execute both queries concurrently
    let (user_profile_result, user_tags_result) = tokio::join!(profile_future, tags_future);

    let user_profile = user_profile_result.map_err(|e| {
        error!("Failed to fetch user profile: {}", e);
        ApiError::new("INTERNAL_SERVER_ERROR", "Could not fetch user profile")
    })?;

    let user_tags: Vec<String> = user_tags_result.unwrap_or_default();

    let context = if let Some(profile) = user_profile {
        UserContext {
            user_id,
            age: calculate_age(&profile.date_of_birth),
            country: profile.country_residence,
            tags: user_tags,
        }
    } else {
        // Should not happen for authenticated user
        UserContext { user_id, age: None, country: None, tags: vec![] }
    };

    // 3. Fetch active questions not answered by user
    // We use a LEFT JOIN on lumimatch.user_answers to exclude answered questions
    // Over-fetch to ensure enough after targeting filter
    let questions = sqlx::query_as::<_, TinderQuestion>(
        r#"
        SELECT q.id, q.title, q.image_url, q.priority, q.targeting_rules, q.specific_date, q.created_at
        FROM lumimatch.questions q
        LEFT JOIN lumimatch.user_answers a ON q.id = a.question_id AND a.user_id = $1
        WHERE q.is_active = true
          AND (q.valid_from IS NULL OR q.valid_from <= NOW())
          AND (q.valid_to IS NULL OR q.valid_to >= NOW())
          AND a.id IS NULL
        ORDER BY q.priority DESC, q.created_at DESC
        LIMIT $2
        "#
    )
    .bind(user_id)
    .bind(overfetch_limit)
    .fetch_all(&state.db_pool)
    .await
    .map_err(|e| {
        error!("Failed to fetch lumimatch questions: {}", e);
        ApiError::new("INTERNAL_SERVER_ERROR", "Could not fetch questions")
    })?;

    // 4. Filter questions based on targeting rules (including specific_date)
    let filtered_questions: Vec<TinderQuestion> = questions
        .into_iter()
        .filter(|q| matches_targeting(&q.targeting_rules, &context, &q.specific_date))
        .take(requested_limit) // Apply requested limit after filtering
        .collect();

    // 5. Fetch options for these filtered questions
    let question_ids: Vec<Uuid> = filtered_questions.iter().map(|q| q.id).collect();
    
    let options = if question_ids.is_empty() {
        vec![]
    } else {
        sqlx::query_as::<_, TinderOption>(
            r#"
            SELECT id, question_id, label, image_url, icon_url, display_order
            FROM lumimatch.options
            WHERE question_id = ANY($1)
            ORDER BY display_order ASC
            "#
        )
        .bind(&question_ids)
        .fetch_all(&state.db_pool)
        .await
        .map_err(|e| {
            error!("Failed to fetch lumimatch options: {}", e);
            ApiError::new("INTERNAL_SERVER_ERROR", "Could not fetch options")
        })?
    };

    // 6. Assemble response with options grouped by question

    let mut result = Vec::new();
    for q in filtered_questions {
        let q_options: Vec<TinderOption> = options
            .iter()
            .filter(|o| o.question_id == q.id)
            .map(|o| TinderOption {
                id: o.id,
                question_id: o.question_id,
                label: o.label.clone(),
                image_url: o.image_url.clone(),
                icon_url: o.icon_url.clone(),
                display_order: o.display_order,
            })
            .collect();

        result.push(QuestionWithOptions {
            question: q,
            options: q_options,
        });
    }

    let execution_time_ms = start_time.elapsed().as_millis() as u64;
    Ok(Json(ApiResponse::success(result, request_id, Some(execution_time_ms), false)))
}

/// Submit an answer
pub async fn submit_answer(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<SubmitAnswerRequest>,
) -> Result<Json<ApiResponse<String>>, ApiError> {
    let request_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();
    
    let start_time = std::time::Instant::now();

    let current_user = extract_user_from_headers(&headers)
        .map_err(|(_status, json_error)| {
            ApiError::new("UNAUTHORIZED", &json_error.0.message)
        })?;

    let user_id = current_user.user_id;

    // Insert answer
    // We use ON CONFLICT DO NOTHING to handle double submissions gracefully
    let result = sqlx::query(
        r#"
        INSERT INTO lumimatch.user_answers (user_id, question_id, option_id)
        VALUES ($1, $2, $3)
        ON CONFLICT (user_id, question_id) DO NOTHING
        "#
    )
    .bind(user_id)
    .bind(payload.question_id)
    .bind(payload.option_id)
    .execute(&state.db_pool)
    .await
    .map_err(|e| {
        error!("Failed to submit lumimatch answer: {}", e);
        ApiError::new("INTERNAL_SERVER_ERROR", "Could not save answer")
    })?;

    if result.rows_affected() == 0 {
        // Already answered, but we return success to be idempotent
        debug!("User {} already answered question {}", user_id, payload.question_id);
    } else {
        info!("User {} answered question {}", user_id, payload.question_id);
    }

    let execution_time_ms = start_time.elapsed().as_millis() as u64;
    Ok(Json(ApiResponse::success("Answer received".to_string(), request_id, Some(execution_time_ms), false)))
}

// --- Router ---

pub fn create_tinder_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v4/lumimatch/questions", get(get_pending_questions))
        .route("/api/v4/lumimatch/answers", post(submit_answer))
}
