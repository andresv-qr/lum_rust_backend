use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use chrono::{DateTime, Utc};
use serde_json::Value;

/// Modelo para la tabla rewards.user_invoice_summary
#[derive(Debug, Clone, FromRow, Serialize, Deserialize)]
pub struct UserInvoiceSummary {
    pub user_id: i32,
    pub total_facturas: Option<i32>,
    pub total_monto: Option<f64>,
    pub total_items: Option<i32>,
    pub n_descuentos: Option<i32>,
    pub total_descuento: Option<f64>,
    pub top_emisores: Option<Value>, // JSONB field
    pub top_categorias: Option<Value>, // JSONB field
    pub serie_mensual: Option<Value>, // JSONB field
    pub updated_at: Option<DateTime<Utc>>,
    pub comparativo_categoria: Option<Value>, // JSONB field
}

#[derive(Debug, Serialize)]
pub struct UserSummaryResponse {
    pub summary: UserInvoiceSummary,
    pub performance_metrics: PerformanceMetrics,
    pub trends: TrendAnalysis,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct PerformanceMetrics {
    pub month_over_month_growth: f64,
    pub invoice_frequency_score: f64,
    pub spending_tier: String,
    pub lumis_efficiency: f64,
}

#[derive(Debug, Serialize, Default)]
pub struct TrendAnalysis {
    pub monthly_trend: String, // "increasing", "decreasing", "stable"
    pub avg_monthly_invoices: f64,
    pub seasonal_pattern: String,
    pub projected_next_month: f64,
}

#[derive(Debug, Deserialize, Default)]
pub struct UserSummaryQuery {
    pub include_trends: Option<bool>,
    pub include_projections: Option<bool>,
    pub currency: Option<String>,
}
