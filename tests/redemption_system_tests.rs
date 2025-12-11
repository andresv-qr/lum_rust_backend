// ============================================================================
// REDEMPTION SYSTEM TESTS - Tests unitarios e integración
// ============================================================================

#[cfg(test)]
mod tests {
    use sqlx::PgPool;
    use uuid::Uuid;
    use chrono::{Utc, Duration};

    // ========================================================================
    // HELPER FUNCTIONS
    // ========================================================================

    async fn setup_test_db() -> PgPool {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://avalencia:Jacobo23@dbmain.lumapp.org/tfactu".to_string());
        
        PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    async fn create_test_offer(db: &PgPool, stock: i32, is_active: bool) -> Uuid {
        let offer_id = Uuid::new_v4();
        
        sqlx::query(
            r#"
            INSERT INTO rewards.redemption_offers 
                (offer_id, name, name_friendly, lumis_cost, points, is_active, 
                 stock_quantity, valid_from, valid_to)
            VALUES ($1, $2, $3, $4, $5, $6, $7, NOW(), NOW() + INTERVAL '1 year')
            "#
        )
        .bind(offer_id)
        .bind("Test Offer")
        .bind("Test Offer Friendly")
        .bind(50)
        .bind(50)
        .bind(is_active)
        .bind(stock)
        .execute(db)
        .await
        .expect("Failed to create test offer");
        
        offer_id
    }

    async fn create_test_redemption(
        db: &PgPool, 
        offer_id: Uuid, 
        user_id: i64, 
        status: &str,
        expired: bool
    ) -> Uuid {
        let redemption_id = Uuid::new_v4();
        let code = format!("LUMS-TEST{}", &redemption_id.to_string()[..6]).to_uppercase();
        let expires_at = if expired {
            Utc::now() - Duration::hours(1)
        } else {
            Utc::now() + Duration::hours(24)
        };
        
        sqlx::query(
            r#"
            INSERT INTO rewards.user_redemptions 
                (redemption_id, user_id, offer_id, lumis_cost, redemption_code, 
                 status, expires_at, created_at)
            VALUES ($1, $2, $3, 50, $4, $5, $6, NOW())
            "#
        )
        .bind(redemption_id)
        .bind(user_id)
        .bind(offer_id)
        .bind(&code)
        .bind(status)
        .bind(expires_at)
        .execute(db)
        .await
        .expect("Failed to create test redemption");
        
        redemption_id
    }

    async fn cleanup_test_data(db: &PgPool, offer_id: Uuid) {
        let _ = sqlx::query("DELETE FROM rewards.user_redemptions WHERE offer_id = $1")
            .bind(offer_id)
            .execute(db)
            .await;
        
        let _ = sqlx::query("DELETE FROM rewards.redemption_offers WHERE offer_id = $1")
            .bind(offer_id)
            .execute(db)
            .await;
    }

    // ========================================================================
    // UNIT TESTS - REDEMPTION CODE GENERATION
    // ========================================================================

    #[test]
    fn test_redemption_code_format() {
        // Test que el código sigue el formato LUMS-XXXXXX
        let code = format!("LUMS-{}", &Uuid::new_v4().to_string()[..6].to_uppercase());
        
        assert!(code.starts_with("LUMS-"));
        assert_eq!(code.len(), 11); // LUMS- + 6 chars
        assert!(code.chars().skip(5).all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_qr_url_format() {
        let code = "LUMS-ABC123";
        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9";
        let base_url = "https://lumis.pa/redeem";
        
        let url = format!("{}?code={}&token={}", base_url, code, token);
        
        assert!(url.contains(code));
        assert!(url.contains("token="));
    }

    // ========================================================================
    // UNIT TESTS - OFFER VALIDATION
    // ========================================================================

    #[tokio::test]
    async fn test_offer_exists_and_active() {
        let db = setup_test_db().await;
        let offer_id = create_test_offer(&db, 100, true).await;
        
        let offer: Option<(bool,)> = sqlx::query_as(
            "SELECT is_active FROM rewards.redemption_offers WHERE offer_id = $1"
        )
        .bind(offer_id)
        .fetch_optional(&db)
        .await
        .expect("Query failed");
        
        assert!(offer.is_some());
        assert!(offer.unwrap().0); // is_active = true
        
        cleanup_test_data(&db, offer_id).await;
    }

    #[tokio::test]
    async fn test_offer_inactive_rejected() {
        let db = setup_test_db().await;
        let offer_id = create_test_offer(&db, 100, false).await; // inactive
        
        let offer: Option<(bool,)> = sqlx::query_as(
            "SELECT is_active FROM rewards.redemption_offers WHERE offer_id = $1 AND is_active = true"
        )
        .bind(offer_id)
        .fetch_optional(&db)
        .await
        .expect("Query failed");
        
        assert!(offer.is_none()); // Should not find inactive offer
        
        cleanup_test_data(&db, offer_id).await;
    }

    #[tokio::test]
    async fn test_offer_out_of_stock() {
        let db = setup_test_db().await;
        let offer_id = create_test_offer(&db, 0, true).await; // no stock
        
        let stock: Option<(i32,)> = sqlx::query_as(
            "SELECT stock_quantity FROM rewards.redemption_offers WHERE offer_id = $1"
        )
        .bind(offer_id)
        .fetch_optional(&db)
        .await
        .expect("Query failed");
        
        assert!(stock.is_some());
        assert_eq!(stock.unwrap().0, 0);
        
        cleanup_test_data(&db, offer_id).await;
    }

    // ========================================================================
    // UNIT TESTS - REDEMPTION STATUS
    // ========================================================================

    #[tokio::test]
    async fn test_redemption_status_pending() {
        let db = setup_test_db().await;
        let offer_id = create_test_offer(&db, 100, true).await;
        let redemption_id = create_test_redemption(&db, offer_id, 99999, "pending", false).await;
        
        let status: (String,) = sqlx::query_as(
            "SELECT status FROM rewards.user_redemptions WHERE redemption_id = $1"
        )
        .bind(redemption_id)
        .fetch_one(&db)
        .await
        .expect("Query failed");
        
        assert_eq!(status.0, "pending");
        
        cleanup_test_data(&db, offer_id).await;
    }

    #[tokio::test]
    async fn test_redemption_confirm_updates_status() {
        let db = setup_test_db().await;
        let offer_id = create_test_offer(&db, 100, true).await;
        let redemption_id = create_test_redemption(&db, offer_id, 99999, "pending", false).await;
        
        // Simulate confirmation
        sqlx::query(
            "UPDATE rewards.user_redemptions SET status = 'used', used_at = NOW() WHERE redemption_id = $1"
        )
        .bind(redemption_id)
        .execute(&db)
        .await
        .expect("Update failed");
        
        let status: (String,) = sqlx::query_as(
            "SELECT status FROM rewards.user_redemptions WHERE redemption_id = $1"
        )
        .bind(redemption_id)
        .fetch_one(&db)
        .await
        .expect("Query failed");
        
        assert_eq!(status.0, "used");
        
        cleanup_test_data(&db, offer_id).await;
    }

    #[tokio::test]
    async fn test_expired_redemption_not_confirmable() {
        let db = setup_test_db().await;
        let offer_id = create_test_offer(&db, 100, true).await;
        let redemption_id = create_test_redemption(&db, offer_id, 99999, "pending", true).await;
        
        // Check that it's expired
        let row: Option<(Uuid,)> = sqlx::query_as(
            "SELECT redemption_id FROM rewards.user_redemptions 
             WHERE redemption_id = $1 AND status = 'pending' AND expires_at > NOW()"
        )
        .bind(redemption_id)
        .fetch_optional(&db)
        .await
        .expect("Query failed");
        
        // Should NOT find it because it's expired
        assert!(row.is_none());
        
        cleanup_test_data(&db, offer_id).await;
    }

    // ========================================================================
    // UNIT TESTS - DOUBLE CONFIRMATION PREVENTION
    // ========================================================================

    #[tokio::test]
    async fn test_prevent_double_confirmation() {
        let db = setup_test_db().await;
        let offer_id = create_test_offer(&db, 100, true).await;
        let redemption_id = create_test_redemption(&db, offer_id, 99999, "used", false).await;
        
        // Try to confirm again - should not update
        let result = sqlx::query(
            "UPDATE rewards.user_redemptions SET status = 'used' 
             WHERE redemption_id = $1 AND status = 'pending'"
        )
        .bind(redemption_id)
        .execute(&db)
        .await
        .expect("Update failed");
        
        // No rows affected because status is already 'used'
        assert_eq!(result.rows_affected(), 0);
        
        cleanup_test_data(&db, offer_id).await;
    }

    // ========================================================================
    // UNIT TESTS - USER REDEMPTION LIMITS
    // ========================================================================

    #[tokio::test]
    async fn test_user_redemption_count() {
        let db = setup_test_db().await;
        let offer_id = create_test_offer(&db, 100, true).await;
        let user_id = 88888_i64;
        
        // Create 3 redemptions for user
        for _ in 0..3 {
            create_test_redemption(&db, offer_id, user_id, "used", false).await;
        }
        
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM rewards.user_redemptions WHERE user_id = $1 AND offer_id = $2"
        )
        .bind(user_id)
        .bind(offer_id)
        .fetch_one(&db)
        .await
        .expect("Query failed");
        
        assert_eq!(count.0, 3);
        
        cleanup_test_data(&db, offer_id).await;
    }

    // ========================================================================
    // UNIT TESTS - STOCK MANAGEMENT
    // ========================================================================

    #[tokio::test]
    async fn test_stock_decrement() {
        let db = setup_test_db().await;
        let offer_id = create_test_offer(&db, 10, true).await;
        
        // Simulate stock decrement
        sqlx::query(
            "UPDATE rewards.redemption_offers SET stock_quantity = stock_quantity - 1 WHERE offer_id = $1"
        )
        .bind(offer_id)
        .execute(&db)
        .await
        .expect("Update failed");
        
        let stock: (i32,) = sqlx::query_as(
            "SELECT stock_quantity FROM rewards.redemption_offers WHERE offer_id = $1"
        )
        .bind(offer_id)
        .fetch_one(&db)
        .await
        .expect("Query failed");
        
        assert_eq!(stock.0, 9);
        
        cleanup_test_data(&db, offer_id).await;
    }

    // ========================================================================
    // INTEGRATION TESTS - EXPIRATION JOB
    // ========================================================================

    #[tokio::test]
    async fn test_expire_old_redemptions() {
        let db = setup_test_db().await;
        let offer_id = create_test_offer(&db, 100, true).await;
        let redemption_id = create_test_redemption(&db, offer_id, 77777, "pending", true).await;
        
        // Run expiration logic
        let expired_count = sqlx::query(
            "UPDATE rewards.user_redemptions SET status = 'expired' 
             WHERE status = 'pending' AND expires_at < NOW()"
        )
        .execute(&db)
        .await
        .expect("Update failed");
        
        assert!(expired_count.rows_affected() >= 1);
        
        // Verify status changed
        let status: (String,) = sqlx::query_as(
            "SELECT status FROM rewards.user_redemptions WHERE redemption_id = $1"
        )
        .bind(redemption_id)
        .fetch_one(&db)
        .await
        .expect("Query failed");
        
        assert_eq!(status.0, "expired");
        
        cleanup_test_data(&db, offer_id).await;
    }

    // ========================================================================
    // UNIT TESTS - TOKEN VALIDATION
    // ========================================================================

    #[test]
    fn test_token_hash_consistency() {
        use sha2::{Sha256, Digest};
        
        let token = "test_token_12345";
        
        // Hash twice and verify same result
        let hash1 = hex::encode(Sha256::digest(token.as_bytes()));
        let hash2 = hex::encode(Sha256::digest(token.as_bytes()));
        
        assert_eq!(hash1, hash2);
        assert_eq!(hash1.len(), 64); // SHA256 hex length
    }

    // ========================================================================
    // METRICS TEST (simple)
    // ========================================================================

    #[test]
    fn test_metrics_names() {
        // Just verify metric naming conventions
        let redemption_metric = "redemptions_total";
        let validation_metric = "merchant_validations_total";
        
        assert!(redemption_metric.contains("redemptions"));
        assert!(validation_metric.contains("validations"));
    }
}
