use crate::models::invoice::{InvoiceHeader, InvoiceDetail, InvoicePayment};
use anyhow::{Context, Result};
use chrono::NaiveDateTime;

fn to_f64(value: &str) -> Option<f64> {
    value.replace(',', "").trim().parse().ok()
}

pub fn parse_invoice_data(
    extracted_data: &crate::processing::web_scraping::ocr_extractor::ExtractedData,
    url: &str,
) -> Result<(InvoiceHeader, Vec<InvoiceDetail>, Vec<InvoicePayment>)> {
    let main_info = &extracted_data.header;
    let line_items = &extracted_data.details;

    let cufe = main_info
        .get("cufe")
        .context("CUFE not found in main info")?
        .clone();

    let _date_str = main_info.get("date").cloned().unwrap_or_default();
    
    let header = InvoiceHeader {
        no: main_info.get("no").cloned().unwrap_or_default(),
        date: main_info.get("date").and_then(|s| NaiveDateTime::parse_from_str(&s, "%d/%m/%Y %H:%M:%S").ok()),
        cufe: main_info.get("cufe").cloned().unwrap_or_default(),
        issuer_name: main_info.get("emisor_name").cloned().unwrap_or_default(),
        issuer_ruc: main_info.get("emisor_ruc").cloned().unwrap_or_default(),
        issuer_dv: main_info.get("emisor_dv").cloned().unwrap_or_default(),
        issuer_address: main_info.get("emisor_address").cloned().unwrap_or_default(),
        issuer_phone: main_info.get("emisor_phone").cloned().unwrap_or_default(),
        tot_amount: main_info.get("tot_amount").and_then(|s| to_f64(s)).unwrap_or(0.0),
        tot_itbms: main_info.get("tot_itbms").and_then(|s| to_f64(s)).unwrap_or(0.0),
        url: url.to_string(),
        r#type: "".to_string(), // Will be set based on URL analysis
        process_date: chrono::Utc::now(),
        reception_date: chrono::Utc::now(),
        user_id: 0, // To be filled later
        origin: "WHATSAPP".to_string(),
        user_email: "".to_string(), // To be filled later
    };

    let details: Vec<InvoiceDetail> = line_items
        .iter()
        .enumerate()
        .map(|(_i, item)| {
            let linea = item.get("linea").cloned().unwrap_or_default();
            let partkey = format!("{}_{}", &cufe, linea);
            InvoiceDetail {
                partkey,
                cufe: cufe.clone(),
                date: header.date, // Propagate from header
                quantity: item.get("quantity").cloned().unwrap_or_default(),
                code: item.get("code").cloned().unwrap_or_default(),
                description: item.get("description").cloned().unwrap_or_default(),
                unit_price: item.get("unit_price").cloned().unwrap_or_default(),
                unit_discount: item.get("unit_discount").cloned(),
                itbms: item.get("itbms").cloned(),
                total: item.get("total").cloned().unwrap_or_default(),
                amount: item.get("amount").cloned().unwrap_or_default(),
                information_of_interest: item.get("information_of_interest").cloned().unwrap_or_default(),
                linea,
            }
        })
        .collect();

    // Create payment record from header data that includes vuelto and total_pagado
    let payments = if main_info.contains_key("vuelto") || main_info.contains_key("total_pagado") {
        vec![InvoicePayment {
            cufe: cufe.clone(),
            vuelto: main_info.get("vuelto").cloned(),
            total_pagado: main_info.get("total_pagado").cloned(),
        }]
    } else {
        vec![]
    };

    Ok((header, details, payments))
}
