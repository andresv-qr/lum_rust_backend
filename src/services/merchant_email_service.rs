// ============================================================================
// MERCHANT EMAIL SERVICE - Reportes semanales y notificaciones
// ============================================================================

use anyhow::Result;
use chrono::{Duration, Utc};
use lettre::{
    message::{header::ContentType, MultiPart, SinglePart},
    transport::smtp::authentication::Credentials,
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use sqlx::PgPool;
use tracing::{error, info};
use uuid::Uuid;

pub struct MerchantEmailService {
    smtp_server: String,
    smtp_username: String,
    smtp_password: String,
}

impl MerchantEmailService {
    pub fn new() -> Result<Self> {
        Ok(Self {
            smtp_server: std::env::var("SMTP_SERVER")
                .unwrap_or_else(|_| "smtp.gmail.com".to_string()),
            smtp_username: std::env::var("SMTP_USERNAME")
                .map_err(|_| anyhow::anyhow!("SMTP_USERNAME not configured"))?,
            smtp_password: std::env::var("SMTP_PASSWORD")
                .map_err(|_| anyhow::anyhow!("SMTP_PASSWORD not configured"))?,
        })
    }

    /// Enviar reporte semanal a un comercio
    pub async fn send_weekly_report(
        &self,
        merchant_id: Uuid,
        merchant_name: &str,
        merchant_email: &str,
        pool: &PgPool,
    ) -> Result<()> {
        info!("📧 Generating weekly report for merchant: {}", merchant_name);

        // Obtener estadísticas de la semana
        let stats = self.get_weekly_stats(merchant_id, pool).await?;

        let html_body = self.generate_html_report(merchant_name, &stats);
        let plain_body = self.generate_plain_report(merchant_name, &stats);

        self.send_email(
            merchant_email,
            &format!("📊 Reporte Semanal - {}", merchant_name),
            &html_body,
            &plain_body,
        )
        .await
    }

    /// Obtener estadísticas de la semana pasada
    async fn get_weekly_stats(&self, merchant_id: Uuid, pool: &PgPool) -> Result<WeeklyStats> {
        let now = Utc::now();
        let week_ago = now - Duration::days(7);

        let result = sqlx::query!(
            r#"
            SELECT 
                COUNT(*) as total_redemptions,
                COUNT(CASE WHEN redemption_status = 'confirmed' THEN 1 END) as confirmed,
                COUNT(CASE WHEN redemption_status = 'pending' THEN 1 END) as pending,
                COUNT(CASE WHEN redemption_status = 'cancelled' THEN 1 END) as cancelled,
                COALESCE(SUM(CASE WHEN redemption_status = 'confirmed' THEN lumis_spent ELSE 0 END), 0) as total_lumis
            FROM rewards.user_redemptions ur
            JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
            WHERE ro.merchant_id = $1
              AND ur.created_at >= $2
              AND ur.created_at < $3
            "#,
            merchant_id,
            week_ago,
            now
        )
        .fetch_one(pool)
        .await?;

        // Top ofertas de la semana
        let top_offers = sqlx::query!(
            r#"
            SELECT 
                ro.name_friendly,
                COUNT(*) as redemptions
            FROM rewards.user_redemptions ur
            JOIN rewards.redemption_offers ro ON ur.offer_id = ro.offer_id
            WHERE ro.merchant_id = $1
              AND ur.created_at >= $2
              AND ur.created_at < $3
              AND ur.redemption_status = 'confirmed'
            GROUP BY ro.name_friendly
            ORDER BY redemptions DESC
            LIMIT 3
            "#,
            merchant_id,
            week_ago,
            now
        )
        .fetch_all(pool)
        .await?;

        Ok(WeeklyStats {
            total_redemptions: result.total_redemptions.unwrap_or(0),
            confirmed: result.confirmed.unwrap_or(0),
            pending: result.pending.unwrap_or(0),
            cancelled: result.cancelled.unwrap_or(0),
            total_lumis: result.total_lumis
                .map(|d| d.to_string().parse::<i64>().unwrap_or(0))
                .unwrap_or(0),
            top_offers: top_offers
                .into_iter()
                .map(|r| TopOffer {
                    name: r.name_friendly.unwrap_or_default(),
                    redemptions: r.redemptions.unwrap_or(0),
                })
                .collect(),
            week_start: week_ago.format("%d/%m/%Y").to_string(),
            week_end: now.format("%d/%m/%Y").to_string(),
        })
    }

    /// Generar HTML del reporte
    fn generate_html_report(&self, merchant_name: &str, stats: &WeeklyStats) -> String {
        let top_offers_html = if stats.top_offers.is_empty() {
            "<p style='color: #666;'>No hubo redenciones esta semana.</p>".to_string()
        } else {
            stats
                .top_offers
                .iter()
                .enumerate()
                .map(|(i, offer)| {
                    format!(
                        r#"<div style="padding: 10px; background: #f8f9fa; margin: 5px 0; border-radius: 5px;">
                            <span style="font-weight: bold; color: #6B46C1;">#{}</span> {} 
                            <span style="float: right; color: #666;">{} canjes</span>
                        </div>"#,
                        i + 1,
                        offer.name,
                        offer.redemptions
                    )
                })
                .collect::<Vec<_>>()
                .join("")
        };

        format!(
            r#"
<!DOCTYPE html>
<html>
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
</head>
<body style="font-family: Arial, sans-serif; max-width: 600px; margin: 0 auto; padding: 20px; background-color: #f5f5f5;">
    <div style="background: white; padding: 30px; border-radius: 10px; box-shadow: 0 2px 10px rgba(0,0,0,0.1);">
        <!-- Header -->
        <div style="text-align: center; margin-bottom: 30px;">
            <h1 style="color: #6B46C1; margin: 0;">📊 Reporte Semanal</h1>
            <p style="color: #666; margin: 10px 0 0 0;">{} - {}</p>
            <h2 style="color: #333; margin: 10px 0 0 0;">{}</h2>
        </div>

        <!-- Stats Grid -->
        <div style="display: grid; grid-template-columns: 1fr 1fr; gap: 15px; margin-bottom: 30px;">
            <div style="background: linear-gradient(135deg, #667eea 0%, #764ba2 100%); padding: 20px; border-radius: 8px; text-align: center; color: white;">
                <div style="font-size: 32px; font-weight: bold;">{}</div>
                <div style="font-size: 14px; opacity: 0.9;">Redenciones Totales</div>
            </div>
            <div style="background: linear-gradient(135deg, #f093fb 0%, #f5576c 100%); padding: 20px; border-radius: 8px; text-align: center; color: white;">
                <div style="font-size: 32px; font-weight: bold;">{}</div>
                <div style="font-size: 14px; opacity: 0.9;">Confirmadas</div>
            </div>
            <div style="background: linear-gradient(135deg, #4facfe 0%, #00f2fe 100%); padding: 20px; border-radius: 8px; text-align: center; color: white;">
                <div style="font-size: 32px; font-weight: bold;">{}</div>
                <div style="font-size: 14px; opacity: 0.9;">Pendientes</div>
            </div>
            <div style="background: linear-gradient(135deg, #43e97b 0%, #38f9d7 100%); padding: 20px; border-radius: 8px; text-align: center; color: white;">
                <div style="font-size: 32px; font-weight: bold;">{}</div>
                <div style="font-size: 14px; opacity: 0.9;">Lümis Generados</div>
            </div>
        </div>

        <!-- Top Offers -->
        <div style="margin-bottom: 30px;">
            <h3 style="color: #333; margin-bottom: 15px;">🏆 Ofertas Más Populares</h3>
            {}
        </div>

        <!-- Footer -->
        <div style="text-align: center; padding-top: 20px; border-top: 1px solid #eee; color: #666; font-size: 14px;">
            <p>¿Necesitas ayuda? Contáctanos en <a href="mailto:soporte@lumapp.org" style="color: #6B46C1;">soporte@lumapp.org</a></p>
            <p style="margin-top: 10px;">
                <a href="https://comercios.lumapp.org" style="color: #6B46C1; text-decoration: none;">Visitar Portal de Comercios →</a>
            </p>
        </div>
    </div>
</body>
</html>
            "#,
            stats.week_start,
            stats.week_end,
            merchant_name,
            stats.total_redemptions,
            stats.confirmed,
            stats.pending,
            stats.total_lumis,
            top_offers_html
        )
    }

    /// Generar versión plain text del reporte
    fn generate_plain_report(&self, merchant_name: &str, stats: &WeeklyStats) -> String {
        let top_offers_text = if stats.top_offers.is_empty() {
            "No hubo redenciones esta semana.".to_string()
        } else {
            stats
                .top_offers
                .iter()
                .enumerate()
                .map(|(i, offer)| format!("  {}. {} ({} canjes)", i + 1, offer.name, offer.redemptions))
                .collect::<Vec<_>>()
                .join("\n")
        };

        format!(
            r#"
REPORTE SEMANAL - {}
Período: {} - {}

ESTADÍSTICAS:
- Redenciones Totales: {}
- Confirmadas: {}
- Pendientes: {}
- Lümis Generados: {}

OFERTAS MÁS POPULARES:
{}

---
¿Necesitas ayuda? Contáctanos en soporte@lumapp.org
Visita: https://comercios.lumapp.org
            "#,
            merchant_name,
            stats.week_start,
            stats.week_end,
            stats.total_redemptions,
            stats.confirmed,
            stats.pending,
            stats.total_lumis,
            top_offers_text
        )
    }

    /// Enviar email usando SMTP
    async fn send_email(
        &self,
        to: &str,
        subject: &str,
        html_body: &str,
        plain_body: &str,
    ) -> Result<()> {
        let email_message = Message::builder()
            .from(self.smtp_username.parse()?)
            .to(to.parse()?)
            .subject(subject)
            .multipart(
                MultiPart::alternative()
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_PLAIN)
                            .body(plain_body.to_string()),
                    )
                    .singlepart(
                        SinglePart::builder()
                            .header(ContentType::TEXT_HTML)
                            .body(html_body.to_string()),
                    ),
            )?;

        let creds = Credentials::new(self.smtp_username.clone(), self.smtp_password.clone());

        let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay(&self.smtp_server)?
            .credentials(creds)
            .build();

        mailer.send(email_message).await?;

        info!("✅ Weekly report sent to: {}", to);
        Ok(())
    }
}

#[derive(Debug)]
struct WeeklyStats {
    total_redemptions: i64,
    confirmed: i64,
    pending: i64,
    #[allow(dead_code)]
    cancelled: i64,
    total_lumis: i64,
    top_offers: Vec<TopOffer>,
    week_start: String,
    week_end: String,
}

#[derive(Debug)]
struct TopOffer {
    name: String,
    redemptions: i64,
}

/// Tarea programada para enviar reportes semanales
pub async fn send_weekly_reports_task(pool: PgPool) -> Result<()> {
    info!("📧 Starting weekly reports task");

    let email_service = match MerchantEmailService::new() {
        Ok(service) => service,
        Err(e) => {
            error!("Failed to initialize email service: {}", e);
            return Err(e);
        }
    };

    // Obtener todos los comercios activos con email
    let merchants = sqlx::query!(
        r#"
        SELECT merchant_id, merchant_name, contact_email
        FROM rewards.merchants
        WHERE is_active = true
          AND contact_email IS NOT NULL
          AND contact_email != ''
        "#
    )
    .fetch_all(&pool)
    .await?;

    info!("📧 Sending weekly reports to {} merchants", merchants.len());

    for merchant in merchants {
        if let Err(e) = email_service
            .send_weekly_report(
                merchant.merchant_id,
                &merchant.merchant_name,
                &merchant.contact_email.unwrap_or_default(),
                &pool,
            )
            .await
        {
            error!(
                "Failed to send weekly report to {}: {}",
                merchant.merchant_name, e
            );
        }

        // Pequeña pausa entre emails para no saturar el servidor SMTP
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }

    info!("✅ Weekly reports task completed");
    Ok(())
}
