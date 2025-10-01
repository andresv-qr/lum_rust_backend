use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Default, Clone, Serialize, Deserialize, FromRow)]
pub struct InvoiceHeader {
    pub no: String,
    pub date: Option<NaiveDateTime>,
    pub cufe: String,
    pub issuer_name: String,
    pub issuer_ruc: String,
    pub issuer_dv: String,
    pub issuer_address: String,
    pub issuer_phone: String,
    pub tot_amount: f64,
    pub tot_itbms: f64,
    pub url: String,
    pub r#type: String,
    pub process_date: DateTime<Utc>,
    pub reception_date: DateTime<Utc>,
    pub user_id: i64,
    pub origin: String,
    pub user_email: String,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct InvoiceDetail {
    pub partkey: String,
    pub cufe: String,
    pub date: Option<NaiveDateTime>,
    pub quantity: String,
    pub code: String,
    pub description: String,
    pub unit_price: String,
    pub total: String,
    pub amount: String,
    pub information_of_interest: String,
    pub linea: String,
    pub unit_discount: Option<String>,
    pub itbms: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow, Clone)]
pub struct InvoicePayment {
    pub cufe: String,
    pub vuelto: Option<String>,
    pub total_pagado: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct MefPending {
    pub id: i32,
    pub url: Option<String>,
    pub chat_id: Option<String>,
    pub reception_date: Option<DateTime<Utc>>,
    pub message_id: Option<String>,
    pub type_document: Option<String>,
    pub user_email: Option<String>,
    pub user_id: Option<i64>,
    pub error_message: Option<String>,
    pub origin: Option<String>,
    pub ws_id: Option<String>,
}
