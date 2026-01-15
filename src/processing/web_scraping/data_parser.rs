use crate::models::invoice::{InvoiceHeader, InvoiceDetail, InvoicePayment};
use anyhow::{Context, Result};
use chrono::{NaiveDateTime, DateTime, Utc, TimeZone};
use chrono_tz::America::Panama;

fn to_f64(value: &str) -> Option<f64> {
    value.replace(',', "").trim().parse().ok()
}

/// Convierte fecha de Panamá (DD/MM/YYYY HH:MM:SS) a DateTime<Utc>
/// Las facturas de DGI/MEF vienen en hora local de Panamá (UTC-5)
fn parse_panama_datetime(date_str: &str) -> Result<DateTime<Utc>> {
    let naive_dt = NaiveDateTime::parse_from_str(date_str, "%d/%m/%Y %H:%M:%S")
        .context(format!("Invalid date format: '{}'. Expected format: DD/MM/YYYY HH:MM:SS", date_str))?;
    
    // Interpret as Panama timezone and convert to UTC
    match Panama.from_local_datetime(&naive_dt) {
        chrono::LocalResult::Single(panama_dt) => Ok(panama_dt.with_timezone(&Utc)),
        chrono::LocalResult::Ambiguous(earliest, _) => Ok(earliest.with_timezone(&Utc)),
        chrono::LocalResult::None => {
            anyhow::bail!("Invalid datetime for Panama timezone: {}", date_str)
        }
    }
}

pub fn parse_invoice_data(
    extracted_data: &crate::processing::web_scraping::ocr_extractor::ExtractedData,
    url: &str,
) -> Result<(InvoiceHeader, Vec<InvoiceDetail>, Vec<InvoicePayment>)> {
    let main_info = &extracted_data.header;
    let line_items = &extracted_data.details;

    // ✅ VALIDACIÓN ESTRICTA: CUFE es obligatorio y no puede estar vacío
    let cufe = main_info
        .get("cufe")
        .filter(|s| !s.is_empty())
        .context("CUFE not found or empty in main info")?
        .clone();

    // ✅ VALIDACIÓN ESTRICTA: Número de factura es obligatorio y no puede estar vacío
    let no = main_info
        .get("no")
        .filter(|s| !s.is_empty())
        .context("Invoice number (no) not found or empty")?
        .clone();

    // ✅ VALIDACIÓN ESTRICTA: Fecha es obligatoria y debe tener formato válido
    let date_str = main_info
        .get("date")
        .filter(|s| !s.is_empty())
        .context("Invoice date not found or empty")?;
    
    // Convertir fecha de Panamá a UTC (DGI usa hora local de Panamá, UTC-5)
    let date = parse_panama_datetime(date_str)?;

    // ✅ VALIDACIÓN ESTRICTA: Nombre del emisor es obligatorio y no puede estar vacío
    let issuer_name = main_info
        .get("emisor_name")
        .filter(|s| !s.is_empty())
        .context("Issuer name not found or empty")?
        .clone();

    // ✅ VALIDACIÓN ESTRICTA: RUC del emisor es obligatorio y no puede estar vacío
    let issuer_ruc = main_info
        .get("emisor_ruc")
        .filter(|s| !s.is_empty())
        .context("Issuer RUC not found or empty")?
        .clone();

    // ✅ VALIDACIÓN ESTRICTA: Monto total es obligatorio y debe ser > 0
    let tot_amount = main_info
        .get("tot_amount")
        .and_then(|s| to_f64(s))
        .filter(|&amount| amount > 0.0)
        .context("Total amount not found, invalid, or must be greater than 0")?;
    
    // Campos opcionales (pueden estar vacíos)
    let issuer_dv = main_info.get("emisor_dv").cloned().unwrap_or_default();
    let issuer_address = main_info.get("emisor_address").cloned().unwrap_or_default();
    let issuer_phone = main_info.get("emisor_phone").cloned().unwrap_or_default();
    let tot_itbms = main_info.get("tot_itbms").and_then(|s| to_f64(s)).unwrap_or(0.0);
    
    let header = InvoiceHeader {
        no,
        date: Some(date),
        cufe: cufe.clone(),
        issuer_name,
        issuer_ruc,
        issuer_dv,
        issuer_address,
        issuer_phone,
        tot_amount,
        tot_itbms,
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
