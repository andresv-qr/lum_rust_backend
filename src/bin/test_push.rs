use lum_rust_ws::services::push_notification_service::{PushNotification, PushNotificationService, NotificationPriority};
use lum_rust_ws::state::AppState;
use serde_json::json;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();
    let subscriber = FmtSubscriber::builder().with_max_level(Level::INFO).finish();
    tracing::subscriber::set_global_default(subscriber)?;

    let app_state = AppState::new().await?;
    let user_id: i64 = 1;

    info!("📱 Checking FCM tokens for user {}...", user_id);
    
    // Get all tokens for user
    let tokens: Vec<(i64, String, bool, Option<chrono::DateTime<chrono::Utc>>)> = sqlx::query_as(
        r#"
        SELECT id, fcm_token, is_active, last_used_at
        FROM device_tokens 
        WHERE user_id = $1
        ORDER BY last_used_at DESC NULLS LAST, id DESC
        "#
    )
    .bind(user_id)
    .fetch_all(&app_state.db_pool)
    .await?;

    if tokens.is_empty() {
        info!("❌ No tokens found for user {}", user_id);
        return Ok(());
    }

    info!("Found {} tokens for user {}:", tokens.len(), user_id);
    for (id, token, active, last_used) in &tokens {
        let status = if *active { "✅ ACTIVE" } else { "❌ inactive" };
        info!("  ID: {} | {} | Token: {}... | Last: {:?}", id, status, &token[..token.len().min(30)], last_used);
    }

    // Find the most recent active token, or activate the most recent one
    let active_token = tokens.iter().find(|(_, _, active, _)| *active);
    
    let token_to_use = if let Some((id, token, _, _)) = active_token {
        info!("\n✅ Using active token ID: {}", id);
        token.clone()
    } else {
        info!("\n⚠️ No active tokens found. Activating the most recent one...");
        let (id, token, _, _) = &tokens[0];
        
        // Activate it
        sqlx::query("UPDATE device_tokens SET is_active = true, last_used_at = NOW() WHERE id = $1")
            .bind(*id)
            .execute(&app_state.db_pool)
            .await?;
        
        info!("✅ Activated token ID: {}", id);
        token.clone()
    };

    info!("\n🚀 Sending push notification to user {}...", user_id);
    info!("   Using token: {}...", &token_to_use[..token_to_use.len().min(50)]);
    
    let push_service = PushNotificationService::new(app_state.db_pool.clone());

    if !push_service.is_configured() {
        info!("❌ Push service is NOT configured. Check .env variables.");
        return Ok(());
    }

    let notification = PushNotification {
        user_id: user_id as i32,
        title: "🎉 Notificación de Prueba".to_string(),
        body: "¡Hola! Esta es una notificación de prueba desde el backend de Lüm.".to_string(),
        data: json!({
            "type": "test",
            "click_action": "FLUTTER_NOTIFICATION_CLICK",
            "timestamp": chrono::Utc::now().to_rfc3339()
        }),
        priority: NotificationPriority::High,
    };

    match push_service.send_notification(notification).await {
        Ok(_) => {
            info!("\n✅ ¡Notificación enviada exitosamente!");
            info!("   El usuario debería recibir el push en su dispositivo.");
        }
        Err(e) => {
            info!("\n❌ Error al enviar notificación: {}", e);
            if e.to_string().contains("UNREGISTERED") {
                info!("\n⚠️ El token está UNREGISTERED en Firebase.");
                info!("   Esto significa que:");
                info!("   1. La app fue desinstalada del dispositivo");
                info!("   2. El token expiró");
                info!("   3. La app necesita re-registrar su token FCM");
                info!("\n💡 Solución: El usuario debe abrir la app para que se registre un nuevo token.");
            }
        }
    }

    Ok(())
}
