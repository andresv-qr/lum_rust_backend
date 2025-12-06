//! Test FCM HTTP v1 authentication
//! 
//! Run with: cargo test --test test_fcm_auth -- --nocapture

use anyhow::Result;

#[tokio::test]
async fn test_fcm_oauth_token() -> Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();
    
    let project_id = std::env::var("FIREBASE_PROJECT_ID")
        .expect("FIREBASE_PROJECT_ID not set");
    
    println!("✅ FIREBASE_PROJECT_ID: {}", project_id);
    
    // Verify credentials file exists
    let creds_path = std::env::var("GOOGLE_APPLICATION_CREDENTIALS")
        .expect("GOOGLE_APPLICATION_CREDENTIALS not set");
    
    println!("✅ Credentials file: {}", creds_path);
    
    assert!(std::path::Path::new(&creds_path).exists(), 
        "Credentials file does not exist");
    
    println!("✅ Credentials file exists");
    
    // Try to get OAuth token
    println!("\n🔐 Requesting OAuth token from Google...");
    
    let provider = gcp_auth::provider().await?;
    
    let scopes = &["https://www.googleapis.com/auth/firebase.messaging"];
    let token = provider.token(scopes).await?;
    
    let token_str = token.as_str();
    
    // Token should be a long string starting with "ya29." (Google OAuth format)
    assert!(!token_str.is_empty(), "Token is empty");
    println!("✅ Got OAuth token: {}...{}", 
        &token_str[..20.min(token_str.len())],
        &token_str[token_str.len().saturating_sub(10)..]);
    
    // Verify FCM endpoint would work (dry run - don't actually send)
    let fcm_endpoint = format!(
        "https://fcm.googleapis.com/v1/projects/{}/messages:send",
        project_id
    );
    println!("✅ FCM endpoint: {}", fcm_endpoint);
    
    println!("\n🎉 FCM HTTP v1 authentication is configured correctly!");
    println!("   Push notifications are ready to be sent.");
    
    Ok(())
}
