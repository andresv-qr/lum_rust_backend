// ============================================================================
// MODIFICACIÓN EN gamification_v4.rs PARA USAR DATOS REALES DE FACTURAS
// ============================================================================

// Reemplazar el query del dashboard por este nuevo que usa la vista materializada:

/// Get complete gamification dashboard for user - NUEVA VERSIÓN CON DATOS REALES
#[axum::debug_handler]
pub async fn get_dashboard(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
) -> ResponseJson<UserDashboard> {
    let start_time = Utc::now();
    
    // Dashboard basado en facturas reales + gamificación
    let dashboard = sqlx::query_as!(
        UserDashboard,
        r#"
        SELECT 
            lum.user_id::int4 as user_id,
            lum.email,
            lum.current_lumis::int4 as total_lumis,           -- 🔥 AHORA BASADO EN FACTURAS REALES
            lum.current_level::int4 as current_level,
            lum.level_name,
            COALESCE(lum.level_name || ' - Basado en ' || lum.total_invoices || ' facturas', 'Nivel inicial') as level_description,
            lum.level_color,
            lum.level_benefits,
            CASE 
                WHEN lum.lumis_to_next_level > 0 
                THEN 'Necesitas ' || lum.lumis_to_next_level || ' Lümis más para ' || COALESCE(lum.next_level_name, 'siguiente nivel')
                ELSE 'Has alcanzado el nivel máximo'
            END as next_level_hint,
            lum.lumis_to_next_level::int4 as lumis_to_next_level,
            COALESCE(lum.next_level_name, 'Nivel Máximo') as next_level_name,
            
            -- Streaks activos (mantenemos de gamificación)
            COALESCE(
                (SELECT jsonb_agg(
                    jsonb_build_object(
                        'type', streak_type,
                        'current', current_count,
                        'max', max_count,
                        'last_date', last_activity_date
                    )
                ) FROM gamification.fact_user_streaks 
                WHERE user_id = lum.user_id AND is_active = true), 
                '[]'::jsonb
            ) as active_streaks,
            
            -- Misiones (de gamificación)
            COALESCE(
                (SELECT COUNT(*) FROM gamification.fact_user_missions 
                 WHERE user_id = lum.user_id AND status = 'active'), 0
            ) as active_missions_count,
            
            COALESCE(
                (SELECT COUNT(*) FROM gamification.fact_user_missions 
                 WHERE user_id = lum.user_id AND status = 'completed'), 0
            ) as completed_missions_count,
            
            -- Achievements (de gamificación) 
            COALESCE(
                (SELECT COUNT(*) FROM gamification.fact_user_achievements 
                 WHERE user_id = lum.user_id), 0
            ) as total_achievements,
            
            -- Actividad reciente REAL basada en facturas
            COALESCE(
                (SELECT jsonb_agg(
                    jsonb_build_object(
                        'action', 'invoice_upload',
                        'amount', tot_amount,
                        'merchant', issuer_name,
                        'date', reception_date,
                        'lumis_earned', FLOOR(COALESCE(tot_amount, 0) / 10) + 1  -- Lümis por esta factura
                    ) ORDER BY reception_date DESC
                ) FROM public.invoice_header 
                WHERE user_db_id = lum.user_id 
                AND reception_date >= NOW() - INTERVAL '30 days'
                LIMIT 10), 
                '[]'::jsonb
            ) as recent_activity
            
        FROM gamification.v_user_current_level lum
        WHERE lum.user_id = $1
        "#,
        current_user.user_id as i32
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| ApiError::database_error(&format!("Failed to fetch dashboard: {}", e)))?;
    
    let dashboard = dashboard.ok_or_else(|| {
        ApiError::not_found("User not found or no invoice history")
    })?;
    
    let execution_time = Utc::now().signed_duration_since(start_time).num_milliseconds();
    
    Ok(Json(ApiResponse::success(dashboard, Uuid::new_v4().to_string(), Some(execution_time.try_into().unwrap()), false)))
}

// ============================================================================
// NUEVA FUNCIÓN PARA FORZAR REFRESH DE NIVELES
// ============================================================================

/// Force refresh of user Lum levels (admin/debugging)
#[axum::debug_handler]
pub async fn refresh_lum_levels(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
) -> ResponseJson<serde_json::Value> {
    let start_time = Utc::now();
    
    // Solo permitir a usuarios admin o el mismo usuario
    // TODO: Agregar verificación de permisos admin si es necesario
    
    let result = sqlx::query!(
        "SELECT gamification.refresh_user_lum_levels() as success"
    )
    .fetch_one(&state.db_pool)
    .await
    .map_err(|e| ApiError::database_error(&format!("Failed to refresh levels: {}", e)))?;
    
    let execution_time = Utc::now().signed_duration_since(start_time).num_milliseconds();
    
    Ok(Json(ApiResponse::success(
        serde_json::json!({
            "message": "Lum levels refreshed successfully",
            "refreshed_at": Utc::now(),
            "success": true
        }), 
        Uuid::new_v4().to_string(), 
        Some(execution_time.try_into().unwrap()), 
        false
    )))
}

// ============================================================================
// NUEVO ENDPOINT PARA ESTADÍSTICAS DE FACTURAS
// ============================================================================

#[derive(Debug, Serialize, FromRow)]
pub struct InvoiceStats {
    pub total_invoices: Option<i64>,
    pub total_spent: Option<f64>,
    pub unique_merchants: Option<i64>,
    pub active_months: Option<i64>,
    pub first_invoice_date: Option<DateTime<Utc>>,
    pub last_invoice_date: Option<DateTime<Utc>>,
    pub restaurant_invoices: Option<i64>,
    pub supermarket_invoices: Option<i64>,
    pub pharmacy_invoices: Option<i64>,
    pub engagement_score: Option<i32>,
}

/// Get user's invoice statistics and Lum level details
#[axum::debug_handler]
pub async fn get_invoice_stats(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<CurrentUser>,
) -> ResponseJson<InvoiceStats> {
    let start_time = Utc::now();
    
    let stats = sqlx::query_as!(
        InvoiceStats,
        r#"
        SELECT 
            total_invoices,
            total_spent,
            unique_merchants,
            active_months,
            first_invoice_date,
            last_invoice_date,
            restaurant_invoices,
            supermarket_invoices,
            pharmacy_invoices,
            engagement_score
        FROM gamification.vw_user_lum_levels
        WHERE user_id = $1
        "#,
        current_user.user_id as i32
    )
    .fetch_optional(&state.db_pool)
    .await
    .map_err(|e| ApiError::database_error(&format!("Failed to fetch stats: {}", e)))?;
    
    let stats = stats.ok_or_else(|| {
        ApiError::not_found("User stats not found")
    })?;
    
    let execution_time = Utc::now().signed_duration_since(start_time).num_milliseconds();
    
    Ok(Json(ApiResponse::success(stats, Uuid::new_v4().to_string(), Some(execution_time.try_into().unwrap()), false)))
}

// ============================================================================
// ACTUALIZAR EL ROUTER PARA INCLUIR NUEVOS ENDPOINTS
// ============================================================================

/// Create router for gamification endpoints - VERSIÓN ACTUALIZADA
pub fn create_gamification_v4_router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/v4/gamification/track", post(track_action))
        .route("/api/v4/gamification/dashboard", get(get_dashboard))  // 🔥 AHORA CON DATOS REALES
        .route("/api/v4/gamification/missions", get(get_missions))
        .route("/api/v4/gamification/events", get(get_events))
        .route("/api/v4/gamification/achievements", get(get_achievements))
        .route("/api/v4/gamification/mechanics", get(get_mechanics_info))
        .route("/api/v4/gamification/leaderboard", get(get_leaderboard))
        .route("/api/v4/gamification/refresh-levels", post(refresh_lum_levels))  // 🆕 NUEVO
        .route("/api/v4/gamification/invoice-stats", get(get_invoice_stats))     // 🆕 NUEVO
}

// ============================================================================
// COMENTARIOS SOBRE LOS CAMBIOS
// ============================================================================

/*
🔥 CAMBIOS PRINCIPALES:

1. **Dashboard basado en facturas reales:**
   - total_lumis ahora viene de facturas reales
   - level_description muestra cuántas facturas tiene
   - recent_activity muestra facturas reales recientes
   - next_level_hint es más informativo

2. **Nuevos endpoints:**
   - POST /api/v4/gamification/refresh-levels: Fuerza refresh de niveles
   - GET /api/v4/gamification/invoice-stats: Estadísticas detalladas

3. **Compatibilidad mantenida:**
   - Los campos de respuesta mantienen los mismos nombres
   - API contracts no cambian para el frontend
   - Streaks y misiones siguen funcionando igual

4. **Beneficios:**
   ✅ Datos 100% reales basados en facturas
   ✅ Niveles actualizados automáticamente
   ✅ Performance optimizada con vista materializada
   ✅ Fácil debugging con endpoint de refresh
   ✅ Estadísticas más ricas para analytics

5. **Para implementar:**
   - Ejecutar el SQL de la vista materializada
   - Reemplazar función get_dashboard en gamification_v4.rs
   - Agregar nuevas funciones al router
   - Probar que los levels se calculan correctamente
*/
