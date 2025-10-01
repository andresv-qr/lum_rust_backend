use axum::{
    extract::{State, Query, Extension},
    http::{StatusCode, HeaderMap, HeaderValue},
    response::Json,
    routing::get,
    Router,
};
use std::sync::Arc;
use tracing::{info, error};
use sqlx::Row;

use crate::state::AppState;
use crate::middleware::CurrentUser; // now using extractor
use crate::api::templates::invoice_query_templates::*;


// ============================================================================
// API HANDLERS
// ============================================================================

/// Get invoice details for the authenticated user (requires JWT authentication via middleware)
pub async fn get_invoice_details(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Query(params): Query<InvoiceDetailsRequest>,
) -> Result<(HeaderMap, Json<InvoiceDetailsResponse>), StatusCode> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start_time = std::time::Instant::now();
    info!(%request_id, "Request: Getting invoice details with enhanced pagination");

    let user_id = current_user.user_id;
    
    // Check if we should use keyset pagination
    if let Some(_cursor) = &params.cursor {
        return handle_keyset_pagination(&state, user_id, &params, request_id, start_time).await;
    }
    
    // Fall back to offset pagination for compatibility
    let limit = params.get_limit();
    let offset = params.get_offset();
    let order_by = params.get_order_by();
    let order_direction = params.get_order_direction();
    
    info!("Request {}: Processing for user {} with limit {} offset {} order_by {} direction {}", 
          request_id, user_id, limit, offset, order_by, order_direction);
    
    // Build dynamic ORDER BY clause
    let order_clause = match order_by {
        "reception_date" => format!("d.reception_date {}", order_direction),
        "amount" => format!("d.amount {}", order_direction),
        "issuer_name" => format!("d.issuer_name {}", order_direction),
        _ => format!("d.date {}", order_direction), // default
    };
    
    // Get invoice types filter
    let invoice_types = params.get_invoice_types();
    let has_type_filter = !invoice_types.is_empty();
    
    // Build date range filters
    let mut date_filters = Vec::new();
    let mut bind_index = 2; // Start after user_id
    
    date_filters.push(format!("d.reception_date >= ${}", bind_index));
    bind_index += 1;
    
    if let Some(_to_date) = params.to_date {
        date_filters.push(format!("d.reception_date <= ${}", bind_index));
        bind_index += 1;
    }
    
    // Build amount filters
    let mut amount_filters = Vec::new();
    if let Some(_min_amount) = params.min_amount {
        amount_filters.push(format!("d.amount >= ${}", bind_index));
        bind_index += 1;
    }
    if let Some(_max_amount) = params.max_amount {
        amount_filters.push(format!("d.amount <= ${}", bind_index));
        bind_index += 1;
    }
    
    info!("Request {}: Querying details for user {} with enhanced filters", request_id, user_id);
    
    // Build dynamic query with enhanced filtering
    let base_select = r#"
        SELECT 
            ROW_NUMBER() OVER (ORDER BY d.date DESC) as id,
            d.cufe, d.quantity, d.code, d.description, d.unit_price, d.amount,
            d.unit_discount, d.date, d.total, d.issuer_name, d.reception_date
        FROM public.invoice_with_details d
        WHERE d.user_id = $1
    "#;
    
    let base_count = r#"
        SELECT COUNT(*) as total
        FROM public.invoice_with_details d
        WHERE d.user_id = $1
    "#;
    
    // Build WHERE conditions
    let mut where_conditions = Vec::new();
    where_conditions.extend(date_filters);
    where_conditions.extend(amount_filters);
    
    if has_type_filter {
        where_conditions.push(format!("d.type = ANY(${})", bind_index));
        bind_index += 1;
    }
    
    // Construct final queries
    let where_clause = if where_conditions.is_empty() {
        String::new()
    } else {
        format!(" AND {}", where_conditions.join(" AND "))
    };
    
    let final_query = format!(
        "{}{} ORDER BY {} LIMIT ${} OFFSET ${}",
        base_select, where_clause, order_clause, bind_index, bind_index + 1
    );
    
    let count_query = format!("{}{}", base_count, where_clause);
    
    info!("Request {}: Executing optimized query: {}", request_id, final_query);
    
    // Prepare query parameters
    let mut query_builder = sqlx::query(&final_query).bind(user_id);
    let mut count_builder = sqlx::query(&count_query).bind(user_id);
    
    // Add date parameters
    query_builder = query_builder.bind(params.from_date);
    count_builder = count_builder.bind(params.from_date);
    
    if let Some(to_date) = params.to_date {
        query_builder = query_builder.bind(to_date);
        count_builder = count_builder.bind(to_date);
    }
    
    // Add amount parameters
    if let Some(min_amount) = params.min_amount {
        query_builder = query_builder.bind(min_amount);
        count_builder = count_builder.bind(min_amount);
    }
    if let Some(max_amount) = params.max_amount {
        query_builder = query_builder.bind(max_amount);
        count_builder = count_builder.bind(max_amount);
    }
    
    // Add type filter if present
    if has_type_filter {
        query_builder = query_builder.bind(&invoice_types);
        count_builder = count_builder.bind(&invoice_types);
    }
    
    // Add pagination parameters
    query_builder = query_builder.bind(limit).bind(offset);
    
    // Execute queries concurrently
    let (details_result, count_result) = tokio::try_join!(
        query_builder.fetch_all(&state.db_pool),
        count_builder.fetch_one(&state.db_pool)
    ).map_err(|e| {
        error!("Request {}: Database error in enhanced query: {}", request_id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    let total_count: i64 = count_result.get("total");
    
    // Convert rows to response items
    let detail_items: Vec<InvoiceDetailItem> = details_result
        .into_iter()
        .map(|row| InvoiceDetailItem {
            id: row.get("id"),
            cufe: row.get("cufe"),
            quantity: row.get("quantity"),
            code: row.get("code"),
            date: row.get("date"),
            total: row.get("total"),
            unit_price: row.get("unit_price"),
            amount: row.get("amount"),
            unit_discount: row.get("unit_discount"),
            description: row.get("description"),
            issuer_name: row.get("issuer_name"),
            reception_date: row.get("reception_date"),
        })
        .collect();
    
    let processing_time = start_time.elapsed().as_millis() as u64;
    info!("Request {}: Retrieved {} invoice details (total: {}) in {}ms with enhanced pagination", 
          request_id, detail_items.len(), total_count, processing_time);
    
    // Create optimized response
    let response = InvoiceDetailsResponse::new(
        detail_items,
        total_count,
        limit,
        offset,
        processing_time,
        false, // Not cached in this implementation
    );
    
    // Create response headers
    let mut headers = HeaderMap::new();
    headers.insert("X-Total-Count", HeaderValue::from_str(&total_count.to_string()).unwrap());
    headers.insert("X-Page-Count", HeaderValue::from_str(&response.pagination.total_pages.to_string()).unwrap());
    headers.insert("X-Current-Page", HeaderValue::from_str(&response.pagination.page.to_string()).unwrap());
    
    // Add Link header for navigation
    let mut link_values = Vec::new();
    if let Some(next_offset) = response.pagination.next_offset {
        link_values.push(format!("</api/v4/invoices/details?offset={}&limit={}>; rel=\"next\"", next_offset, limit));
    }
    if let Some(prev_offset) = response.pagination.previous_offset {
        link_values.push(format!("</api/v4/invoices/details?offset={}&limit={}>; rel=\"prev\"", prev_offset, limit));
    }
    link_values.push(format!("</api/v4/invoices/details?offset=0&limit={}>; rel=\"first\"", limit));
    if response.pagination.total_pages > 1 {
        let last_offset = (response.pagination.total_pages - 1) * limit;
        link_values.push(format!("</api/v4/invoices/details?offset={}&limit={}>; rel=\"last\"", last_offset, limit));
    }
    
    if !link_values.is_empty() {
        headers.insert("Link", HeaderValue::from_str(&link_values.join(", ")).unwrap());
    }
    
    Ok((headers, Json(response)))
}

/// Get invoice headers for the authenticated user (requires JWT authentication via middleware)
pub async fn get_invoice_headers(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
    Query(params): Query<InvoiceHeadersRequest>,
) -> Result<Json<InvoiceHeadersResponse>, StatusCode> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start_time = std::time::Instant::now();
    info!(%request_id, "Request: Getting invoice headers with enhanced filtering");

    let user_id = current_user.user_id;
    
    // Check if we should use keyset pagination
    if let Some(_cursor) = &params.cursor {
        return handle_keyset_pagination_headers(&state, user_id, &params, request_id, start_time).await;
    }
    
    // Fall back to offset pagination for compatibility
    let limit = params.get_limit();
    let offset = params.get_offset();
    
    info!("Request {}: Processing headers for user {} with limit {} offset {}", 
          request_id, user_id, limit, offset);
    
    // Build dynamic queries with proper parameter handling
    let mut where_conditions = Vec::new();
    let mut bind_index = 2; // Start after user_id
    
    // Add date filters
    if params.from_date.is_some() {
        where_conditions.push(format!("h.reception_date >= ${}", bind_index));
        bind_index += 1;
    }
    
    if params.to_date.is_some() {
        where_conditions.push(format!("h.reception_date <= ${}", bind_index));
        bind_index += 1;
    }
    
    // Add amount filters
    if params.min_amount.is_some() {
        where_conditions.push(format!("h.tot_amount >= ${}", bind_index));
        bind_index += 1;
    }
    
    if params.max_amount.is_some() {
        where_conditions.push(format!("h.tot_amount <= ${}", bind_index));
        bind_index += 1;
    }
    
    // Add issuer name filter
    if params.issuer_name.is_some() {
        where_conditions.push(format!("h.issuer_name ILIKE ${}", bind_index));
        bind_index += 1;
    }
    
    // Construct final queries
    let where_clause = if where_conditions.is_empty() {
        String::new()
    } else {
        format!(" AND {}", where_conditions.join(" AND "))
    };
    
    let final_query = format!(
        "{}{} ORDER BY h.reception_date DESC LIMIT ${} OFFSET ${}",
        InvoiceQueryTemplates::GET_INVOICE_HEADERS, 
        where_clause, 
        bind_index, 
        bind_index + 1
    );
    
    let count_query = format!(
        "{}{}",
        InvoiceQueryTemplates::COUNT_INVOICE_HEADERS,
        where_clause
    );
    
    info!("Request {}: Executing header query: {}", request_id, final_query);
    
    // Build query parameters with proper types
    let mut query_builder = sqlx::query(&final_query).bind(user_id);
    let mut count_builder = sqlx::query(&count_query).bind(user_id);
    
    // Add date parameters as DateTime<Utc>
    if let Some(from_date) = params.from_date {
        query_builder = query_builder.bind(from_date);
        count_builder = count_builder.bind(from_date);
    }
    
    if let Some(to_date) = params.to_date {
        query_builder = query_builder.bind(to_date);
        count_builder = count_builder.bind(to_date);
    }
    
    // Add amount parameters as f64
    if let Some(min_amount) = params.min_amount {
        query_builder = query_builder.bind(min_amount);
        count_builder = count_builder.bind(min_amount);
    }
    
    if let Some(max_amount) = params.max_amount {
        query_builder = query_builder.bind(max_amount);
        count_builder = count_builder.bind(max_amount);
    }
    
    // Add issuer name parameter as String with LIKE pattern
    let issuer_pattern = params.issuer_name.as_ref().map(|name| format!("%{}%", name));
    if let Some(ref pattern) = issuer_pattern {
        query_builder = query_builder.bind(pattern);
        count_builder = count_builder.bind(pattern);
    }
    
    // Add pagination parameters
    query_builder = query_builder.bind(limit).bind(offset);
    
    // Execute queries concurrently
    let (headers_result, count_result) = tokio::try_join!(
        query_builder.fetch_all(&state.db_pool),
        count_builder.fetch_one(&state.db_pool)
    ).map_err(|e| {
        error!("Request {}: Database error querying invoice headers: {}", request_id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // Extract total count
    let total_count: i64 = count_result.get("count");
    
    // Convert rows to response items
    let header_items: Vec<InvoiceHeaderItem> = headers_result
        .into_iter()
        .map(|row| InvoiceHeaderItem {
            id: row.get("id"),
            date: row.get("date"),
            tot_itbms: row.get("tot_itbms"),
            issuer_name: row.get("issuer_name"),
            no: row.get("no"),
            tot_amount: row.get("tot_amount"),
            url: row.get("url"),
            process_date: row.get("process_date"),
            reception_date: row.get("reception_date"),
            r#type: row.get("type"),
            cufe: row.get("cufe"),
            // Legacy fields for compatibility
            time: row.get("time"),
            auth_date: row.get("auth_date"),
            issuer_ruc: row.get("issuer_ruc"),
            issuer_dv: row.get("issuer_dv"),
            issuer_address: row.get("issuer_address"),
            issuer_phone: row.get("issuer_phone"),
            receptor_name: row.get("receptor_name"),
            details_count: row.get("details_count"),
            payments_count: row.get("payments_count"),
        })
        .collect();
    
    // Create a simple summary (can be enhanced later)
    let summary = InvoiceSummary {
        total_invoices: total_count,
        total_amount: header_items.iter().map(|h| h.tot_amount.unwrap_or(0.0)).sum(),
        unique_issuers: header_items.iter().filter_map(|h| h.issuer_name.as_ref()).collect::<std::collections::HashSet<_>>().len() as i64,
        date_range: crate::api::templates::invoice_query_templates::DateRange {
            earliest: header_items.iter().filter_map(|h| h.reception_date).min(),
            latest: header_items.iter().filter_map(|h| h.reception_date).max(),
        },
        amount_range: crate::api::templates::invoice_query_templates::AmountRange {
            minimum: header_items.iter().filter_map(|h| h.tot_amount).fold(f64::INFINITY, f64::min),
            maximum: header_items.iter().filter_map(|h| h.tot_amount).fold(f64::NEG_INFINITY, f64::max),
            average: if header_items.is_empty() { 0.0 } else { 
                header_items.iter().map(|h| h.tot_amount.unwrap_or(0.0)).sum::<f64>() / header_items.len() as f64 
            },
        },
    };
    
    // Create response
    let current_page = (offset / limit) + 1;
    let page_info = PageInfo::new(current_page, limit, total_count);
    
    let processing_time = start_time.elapsed().as_millis();
    info!("Request {}: Retrieved {} invoice headers (total: {}) in {}ms with enhanced filtering", 
          request_id, header_items.len(), total_count, processing_time);
    
    Ok(Json(InvoiceHeadersResponse::success(
        header_items,
        total_count,
        page_info,
        summary,
    )))
}

// ============================================================================
// KEYSET PAGINATION HELPERS
// ============================================================================

/// Handle keyset pagination for invoice headers
async fn handle_keyset_pagination_headers(
    state: &Arc<AppState>,
    user_id: i64,
    params: &InvoiceHeadersRequest,
    request_id: String,
    start_time: std::time::Instant,
) -> Result<Json<InvoiceHeadersResponse>, StatusCode> {
    let cursor = params.cursor.as_ref().unwrap();
    let limit = params.get_limit();
    let order_by = params.get_order_by();
    let order_direction = params.get_order_direction();
    let direction = params.direction.as_deref().unwrap_or("next");
    
    info!("Request {}: Using keyset pagination with cursor for headers user {} order_by {} direction {} page_direction {}", 
          request_id, user_id, order_by, order_direction, direction);
    
    // Decode cursor to get position values
    let cursor_position = match CursorPosition::decode(cursor) {
        Ok(pos) => pos,
        Err(e) => {
            error!("Request {}: Invalid cursor format: {}", request_id, e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };
    
    info!("Request {}: Decoded cursor - reception_date: {:?}, id: {:?}", 
          request_id, cursor_position.reception_date, cursor_position.id);
    
    // Build keyset query based on order_by field and direction
    let (keyset_condition, keyset_params, effective_order) = build_keyset_condition_headers(
        &order_by, 
        &order_direction, 
        direction, 
        &cursor_position
    );
    
    // Build date range filters
    let mut date_filters = Vec::new();
    let mut bind_index = 2; // Start after user_id
    
    if let Some(_from_date) = params.from_date {
        date_filters.push(format!("h.reception_date >= ${}", bind_index));
        bind_index += 1;
    }
    
    if let Some(_to_date) = params.to_date {
        date_filters.push(format!("h.reception_date <= ${}", bind_index));
        bind_index += 1;
    }
    
    // Build amount filters
    let mut amount_filters = Vec::new();
    if let Some(_min_amount) = params.min_amount {
        amount_filters.push(format!("h.tot_amount >= ${}", bind_index));
        bind_index += 1;
    }
    if let Some(_max_amount) = params.max_amount {
        amount_filters.push(format!("h.tot_amount <= ${}", bind_index));
        bind_index += 1;
    }
    
    // Build issuer name filter
    if let Some(_issuer_name) = &params.issuer_name {
        amount_filters.push(format!("h.issuer_name ILIKE ${}", bind_index));
        bind_index += 1;
    }
    
    // Build dynamic query with keyset condition
    let base_select = r#"
        SELECT h.id, h.date, h.reception_date, h.tot_itbms, h.tot_amount, h.issuer_name, h.no, h.type, h.cufe,
               h.time, h.auth_date, h.issuer_ruc, h.issuer_dv, h.issuer_address, h.issuer_phone,
               h.receptor_name, h.details_count, h.payments_count
        FROM public.invoice_headers h
        WHERE h.user_id = $1
    "#;
    
    // Build WHERE conditions
    let mut where_conditions = Vec::new();
    where_conditions.extend(date_filters);
    where_conditions.extend(amount_filters);
    
    // Add keyset condition with proper parameter indices
    let keyset_condition_with_indices = substitute_keyset_params(&keyset_condition, bind_index);
    where_conditions.push(format!("({})", keyset_condition_with_indices));
    bind_index += keyset_params.len() as i32;
    
    // Construct final query
    let where_clause = format!(" AND {}", where_conditions.join(" AND "));
    let final_query = format!(
        "{}{} ORDER BY {} LIMIT ${}",
        base_select, where_clause, effective_order, bind_index
    );
    
    info!("Request {}: Executing keyset query: {}", request_id, final_query);
    
    // Prepare query parameters
    let mut query_builder = sqlx::query(&final_query).bind(user_id);
    
    // Add date parameters
    if let Some(from_date) = params.from_date {
        query_builder = query_builder.bind(from_date);
    }
    
    if let Some(to_date) = params.to_date {
        query_builder = query_builder.bind(to_date);
    }
    
    // Add amount parameters
    if let Some(min_amount) = params.min_amount {
        query_builder = query_builder.bind(min_amount);
    }
    if let Some(max_amount) = params.max_amount {
        query_builder = query_builder.bind(max_amount);
    }
    
    // Add issuer name parameter
    if let Some(issuer_name) = &params.issuer_name {
        query_builder = query_builder.bind(format!("%{}%", issuer_name));
    }
    
    // Add keyset parameters
    if let (Some(reception_date), Some(id)) = (cursor_position.reception_date, cursor_position.id) {
        query_builder = query_builder.bind(reception_date).bind(id);
    }
    
    // Add limit
    query_builder = query_builder.bind(limit + 1); // +1 to check if there are more records
    
    // Execute query
    let headers_result = query_builder.fetch_all(&state.db_pool).await.map_err(|e| {
        error!("Request {}: Database error in keyset query: {}", request_id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // Check if there are more records
    let has_more = headers_result.len() > limit as usize;
    let header_items: Vec<InvoiceHeaderItem> = headers_result
        .into_iter()
        .take(limit as usize) // Take only the requested amount
        .map(|row| InvoiceHeaderItem {
            id: row.get("id"),
            date: row.get("date"),
            tot_itbms: row.get("tot_itbms"),
            issuer_name: row.get("issuer_name"),
            no: row.get("no"),
            reception_date: row.get("reception_date"),
            tot_amount: row.get("tot_amount"),
            r#type: row.get("type"),
            cufe: row.get("cufe"),
            process_date: row.get("process_date"),
            url: row.get("url"),
            // Legacy fields for compatibility
            time: row.get("time"),
            auth_date: row.get("auth_date"),
            issuer_ruc: row.get("issuer_ruc"),
            issuer_dv: row.get("issuer_dv"),
            issuer_address: row.get("issuer_address"),
            issuer_phone: row.get("issuer_phone"),
            receptor_name: row.get("receptor_name"),
            details_count: row.get("details_count"),
            payments_count: row.get("payments_count"),
        })
        .collect();
    
    // Generate next/previous cursors
    let (next_cursor, previous_cursor) = if header_items.is_empty() {
        (None, None)
    } else {
        let next_cursor = if has_more {
            let last_item = header_items.last().unwrap();
            if let Some(reception_date) = last_item.reception_date {
                Some(CursorPosition::new(
                    None, // date not used for headers
                    None, // amount not used for headers
                    Some(last_item.id),
                    Some(reception_date),
                ).encode())
            } else {
                None
            }
        } else {
            None
        };
        
        let previous_cursor = if direction == "next" {
            // For next page, previous cursor is the first item
            let first_item = header_items.first().unwrap();
            if let Some(reception_date) = first_item.reception_date {
                Some(CursorPosition::new(
                    None, // date not used for headers
                    None, // amount not used for headers
                    Some(first_item.id),
                    Some(reception_date),
                ).encode())
            } else {
                None
            }
        } else {
            None
        };
        
        (next_cursor, previous_cursor)
    };
    
    let processing_time = start_time.elapsed().as_millis() as u64;
    info!("Request {}: Retrieved {} invoice headers with keyset pagination in {}ms", 
          request_id, header_items.len(), processing_time);
    
    // Create cursor pagination metadata
    let cursor_pagination = CursorPaginationMeta {
        has_next_page: has_more,
        has_previous_page: direction == "next", // Simple heuristic
        next_cursor,
        previous_cursor,
        page_size: limit,
        direction: direction.to_string(),
    };
    
    // Create summary
    let summary = InvoiceSummary {
        total_invoices: -1, // Not available in cursor pagination
        total_amount: header_items.iter().map(|h| h.tot_amount.unwrap_or(0.0)).sum(),
        unique_issuers: header_items.iter().filter_map(|h| h.issuer_name.as_ref()).collect::<std::collections::HashSet<_>>().len() as i64,
        date_range: crate::api::templates::invoice_query_templates::DateRange {
            earliest: header_items.iter().filter_map(|h| h.reception_date).min(),
            latest: header_items.iter().filter_map(|h| h.reception_date).max(),
        },
        amount_range: crate::api::templates::invoice_query_templates::AmountRange {
            minimum: header_items.iter().filter_map(|h| h.tot_amount).fold(f64::INFINITY, f64::min),
            maximum: header_items.iter().filter_map(|h| h.tot_amount).fold(f64::NEG_INFINITY, f64::max),
            average: if header_items.is_empty() { 0.0 } else { 
                header_items.iter().map(|h| h.tot_amount.unwrap_or(0.0)).sum::<f64>() / header_items.len() as f64 
            },
        },
    };
    
    // Create page info with cursor pagination
    let page_info = PageInfo::new_with_cursor(cursor_pagination);
    
    // Create optimized response
    let response = InvoiceHeadersResponse::success(
        header_items,
        -1, // total_count not available in cursor pagination
        page_info,
        summary,
    );
    
    Ok(Json(response))
}

/// Handle keyset pagination for invoice details
async fn handle_keyset_pagination(
    state: &Arc<AppState>,
    user_id: i64,
    params: &InvoiceDetailsRequest,
    request_id: String,
    start_time: std::time::Instant,
) -> Result<(HeaderMap, Json<InvoiceDetailsResponse>), StatusCode> {
    let cursor = params.cursor.as_ref().unwrap();
    let limit = params.get_limit();
    let order_by = params.get_order_by();
    let order_direction = params.get_order_direction();
    let direction = params.direction.as_deref().unwrap_or("next");
    
    info!("Request {}: Using keyset pagination with cursor for user {} order_by {} direction {} page_direction {}", 
          request_id, user_id, order_by, order_direction, direction);
    
    // Decode cursor to get position values
    let cursor_position = match CursorPosition::decode(cursor) {
        Ok(pos) => pos,
        Err(e) => {
            error!("Request {}: Invalid cursor format: {}", request_id, e);
            return Err(StatusCode::BAD_REQUEST);
        }
    };
    
    info!("Request {}: Decoded cursor - date: {:?}, amount: {:?}, id: {:?}", 
          request_id, cursor_position.date, cursor_position.amount, cursor_position.id);
    
    // Build keyset query based on order_by field and direction
    let (keyset_condition, keyset_params, effective_order) = build_keyset_condition_with_params(
        &order_by, 
        &order_direction, 
        direction, 
        &cursor_position
    );
    
    // Get invoice types filter
    let invoice_types = params.get_invoice_types();
    let has_type_filter = !invoice_types.is_empty();
    
    // Build date range filters
    let mut date_filters = Vec::new();
    let mut bind_index = 2; // Start after user_id
    
    date_filters.push(format!("d.reception_date >= ${}", bind_index));
    bind_index += 1;
    
    if let Some(_to_date) = params.to_date {
        date_filters.push(format!("d.reception_date <= ${}", bind_index));
        bind_index += 1;
    }
    
    // Build amount filters
    let mut amount_filters = Vec::new();
    if let Some(_min_amount) = params.min_amount {
        amount_filters.push(format!("d.amount >= ${}", bind_index));
        bind_index += 1;
    }
    if let Some(_max_amount) = params.max_amount {
        amount_filters.push(format!("d.amount <= ${}", bind_index));
        bind_index += 1;
    }
    
    // Build dynamic query with keyset condition
    let base_select = r#"
        SELECT 
            ROW_NUMBER() OVER (ORDER BY d.date DESC) as id,
            d.cufe, d.quantity, d.code, d.description, d.unit_price, d.amount,
            d.unit_discount, d.date, d.total, d.issuer_name, d.reception_date
        FROM public.invoice_with_details d
        WHERE d.user_id = $1
    "#;
    
    // Build WHERE conditions
    let mut where_conditions = Vec::new();
    where_conditions.extend(date_filters);
    where_conditions.extend(amount_filters);
    
    // Add keyset condition with proper parameter indices
    let keyset_condition_with_indices = substitute_keyset_params(&keyset_condition, bind_index);
    where_conditions.push(format!("({})", keyset_condition_with_indices));
    bind_index += keyset_params.len() as i32;
    
    if has_type_filter {
        where_conditions.push(format!("d.type = ANY(${})", bind_index));
        bind_index += 1;
    }
    
    // Construct final query
    let where_clause = format!(" AND {}", where_conditions.join(" AND "));
    let final_query = format!(
        "{}{} ORDER BY {} LIMIT ${}",
        base_select, where_clause, effective_order, bind_index
    );
    
    info!("Request {}: Executing keyset query: {}", request_id, final_query);
    
    // Prepare query parameters
    let mut query_builder = sqlx::query(&final_query).bind(user_id);
    
    // Add date parameters
    query_builder = query_builder.bind(params.from_date);
    
    if let Some(to_date) = params.to_date {
        query_builder = query_builder.bind(to_date);
    }
    
    // Add amount parameters
    if let Some(min_amount) = params.min_amount {
        query_builder = query_builder.bind(min_amount);
    }
    if let Some(max_amount) = params.max_amount {
        query_builder = query_builder.bind(max_amount);
    }
    
    // Add keyset parameters - use proper types for binding
    match order_by {
        "reception_date" => {
            if let (Some(reception_date), Some(id)) = (cursor_position.reception_date, cursor_position.id) {
                query_builder = query_builder.bind(reception_date).bind(id);
            }
        },
        "amount" => {
            if let (Some(amount), Some(id)) = (cursor_position.amount, cursor_position.id) {
                query_builder = query_builder.bind(amount).bind(id);
            }
        },
        _ => { // default: date
            if let (Some(date), Some(id)) = (cursor_position.date, cursor_position.id) {
                query_builder = query_builder.bind(date).bind(id);
            }
        }
    }
    
    // Add type filter if present
    if has_type_filter {
        query_builder = query_builder.bind(&invoice_types);
    }
    
    // Add limit
    query_builder = query_builder.bind(limit + 1); // +1 to check if there are more records
    
    // Execute query
    let details_result = query_builder.fetch_all(&state.db_pool).await.map_err(|e| {
        error!("Request {}: Database error in keyset query: {}", request_id, e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
    
    // Check if there are more records
    let has_more = details_result.len() > limit as usize;
    let detail_items: Vec<InvoiceDetailItem> = details_result
        .into_iter()
        .take(limit as usize) // Take only the requested amount
        .map(|row| InvoiceDetailItem {
            id: row.get("id"),
            cufe: row.get("cufe"),
            quantity: row.get("quantity"),
            code: row.get("code"),
            date: row.get("date"),
            total: row.get("total"),
            unit_price: row.get("unit_price"),
            amount: row.get("amount"),
            unit_discount: row.get("unit_discount"),
            description: row.get("description"),
            issuer_name: row.get("issuer_name"),
            reception_date: row.get("reception_date"),
        })
        .collect();
    
    // Generate next/previous cursors
    let (next_cursor, previous_cursor) = if detail_items.is_empty() {
        (None, None)
    } else {
        let next_cursor = if has_more {
            let last_item = detail_items.last().unwrap();
            Some(CursorPosition::new(
                last_item.date.map(|d| d.and_utc()),
                last_item.amount.map(|a| rust_decimal::Decimal::from_f64_retain(a).unwrap_or_default()),
                Some(last_item.id),
                last_item.reception_date,
            ).encode())
        } else {
            None
        };
        
        let previous_cursor = if direction == "next" {
            // For next page, previous cursor is the first item
            let first_item = detail_items.first().unwrap();
            Some(CursorPosition::new(
                first_item.date.map(|d| d.and_utc()),
                first_item.amount.map(|a| rust_decimal::Decimal::from_f64_retain(a).unwrap_or_default()),
                Some(first_item.id),
                first_item.reception_date,
            ).encode())
        } else {
            None
        };
        
        (next_cursor, previous_cursor)
    };
    
    let processing_time = start_time.elapsed().as_millis() as u64;
    info!("Request {}: Retrieved {} invoice details with keyset pagination in {}ms", 
          request_id, detail_items.len(), processing_time);
    
    // Create cursor pagination metadata
    let cursor_pagination = CursorPaginationMeta {
        has_next_page: has_more,
        has_previous_page: direction == "next", // Simple heuristic
        next_cursor,
        previous_cursor,
        page_size: limit,
        direction: direction.to_string(),
    };
    
    // Create optimized response with cursor pagination
    let response = InvoiceDetailsResponse::new_with_cursor(
        detail_items,
        cursor_pagination,
        processing_time,
        false, // Not cached in this implementation
    );
    
    // Create response headers
    let mut headers = HeaderMap::new();
    headers.insert("X-Pagination-Type", HeaderValue::from_str("cursor").unwrap());
    headers.insert("X-Has-Next-Page", HeaderValue::from_str(&has_more.to_string()).unwrap());
    
    if let Some(next_cursor) = &response.pagination.cursor_pagination.as_ref().unwrap().next_cursor {
        let link_value = format!("</api/v4/invoices/details?cursor={}&direction=next&limit={}>; rel=\"next\"", 
                               next_cursor, limit);
        headers.insert("Link", HeaderValue::from_str(&link_value).unwrap());
    }
    
    Ok((headers, Json(response)))
}

/// Build keyset condition with proper parameters
fn build_keyset_condition_with_params(
    order_by: &str,
    order_direction: &str,
    page_direction: &str,
    cursor_position: &CursorPosition,
) -> (String, Vec<String>, String) {
    let is_asc = order_direction == "ASC";
    let is_next = page_direction == "next";
    
    // Determine the comparison operator
    let operator = match (is_asc, is_next) {
        (true, true) => ">",   // ASC + next = greater than
        (true, false) => "<",  // ASC + prev = less than
        (false, true) => "<",  // DESC + next = less than
        (false, false) => ">", // DESC + prev = greater than
    };
    
    let mut param_values = Vec::new();
    
    let condition = match order_by {
        "reception_date" => {
            if let (Some(reception_date), Some(id)) = (cursor_position.reception_date, cursor_position.id) {
                param_values.push(reception_date.format("%Y-%m-%d %H:%M:%S%.f").to_string());
                param_values.push(id.to_string());
                format!(
                    "d.reception_date {} $PARAM1 OR (d.reception_date = $PARAM1 AND d.id {} $PARAM2)",
                    operator, operator
                )
            } else {
                "1=1".to_string()
            }
        },
        "amount" => {
            if let (Some(amount), Some(id)) = (cursor_position.amount, cursor_position.id) {
                param_values.push(amount.to_string());
                param_values.push(id.to_string());
                format!(
                    "d.amount {} $PARAM1 OR (d.amount = $PARAM1 AND d.id {} $PARAM2)",
                    operator, operator
                )
            } else {
                "1=1".to_string()
            }
        },
        _ => { // default: date
            if let (Some(date), Some(id)) = (cursor_position.date, cursor_position.id) {
                param_values.push(date.format("%Y-%m-%d %H:%M:%S%.f").to_string());
                param_values.push(id.to_string());
                format!(
                    "d.date {} $PARAM1 OR (d.date = $PARAM1 AND d.id {} $PARAM2)",
                    operator, operator
                )
            } else {
                "1=1".to_string()
            }
        }
    };
    
    let effective_order = match order_by {
        "reception_date" => format!("d.reception_date {}, d.id {}", order_direction, order_direction),
        "amount" => format!("d.amount {}, d.id {}", order_direction, order_direction),
        _ => format!("d.date {}, d.id {}", order_direction, order_direction),
    };
    
    (condition, param_values, effective_order)
}

/// Substitute parameter placeholders with actual indices
fn substitute_keyset_params(condition: &str, start_index: i32) -> String {
    condition
        .replace("$PARAM1", &format!("${}", start_index))
        .replace("$PARAM2", &format!("${}", start_index + 1))
}

/// Build keyset condition for headers with proper parameters
fn build_keyset_condition_headers(
    _order_by: &str,
    order_direction: &str,
    page_direction: &str,
    cursor_position: &CursorPosition,
) -> (String, Vec<String>, String) {
    let is_asc = order_direction == "ASC";
    let is_next = page_direction == "next";
    
    // Determine the comparison operator
    let operator = match (is_asc, is_next) {
        (true, true) => ">",   // ASC + next = greater than
        (true, false) => "<",  // ASC + prev = less than
        (false, true) => "<",  // DESC + next = less than
        (false, false) => ">", // DESC + prev = greater than
    };
    
    let mut param_values = Vec::new();
    
    // For headers, we only use reception_date and id as ordering fields
    let condition = if let (Some(reception_date), Some(id)) = (cursor_position.reception_date, cursor_position.id) {
        param_values.push(reception_date.format("%Y-%m-%d %H:%M:%S%.f").to_string());
        param_values.push(id.to_string());
        format!(
            "h.reception_date {} $PARAM1 OR (h.reception_date = $PARAM1 AND h.id {} $PARAM2)",
            operator, operator
        )
    } else {
        "1=1".to_string()
    };
    
    let effective_order = format!("h.reception_date {}, h.id {}", order_direction, order_direction);
    
    (condition, param_values, effective_order)
}

// ============================================================================
// ROUTER CREATION
// ============================================================================

pub fn create_invoice_query_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/details", get(get_invoice_details))
        .route("/headers", get(get_invoice_headers))
}
