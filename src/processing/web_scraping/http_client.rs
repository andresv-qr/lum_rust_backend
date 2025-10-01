use anyhow::{Context, Result};
use reqwest::Client;
use tracing::{info, debug};

pub async fn fetch_url_content(client: &Client, url: &str) -> Result<String> {
    client
        .get(url)
        .send()
        .await
        .context("Failed to send request to URL")?
        .text()
        .await
        .context("Failed to read response text")
}

/// Fetch URL content and return both content and final URL after redirections
pub async fn fetch_url_content_with_final_url(client: &Client, url: &str) -> Result<(String, String)> {
    debug!("🌐 Fetching content from URL: {}", url);
    
    let response = client
        .get(url)
        .send()
        .await
        .context("Failed to send request to URL")?;
    
    let final_url = response.url().to_string();
    
    // Log redirection if URLs are different
    if final_url != url {
        info!("🔄 URL redirection detected: {} → {}", url, final_url);
    } else {
        debug!("✅ No redirection needed for URL: {}", url);
    }
    
    let content = response
        .text()
        .await
        .context("Failed to read response text")?;
    
    debug!("📄 Successfully fetched {} chars from final URL: {}", content.len(), final_url);
    
    Ok((content, final_url))
}

/// Get the final URL after following redirections (using HEAD request for efficiency)
pub async fn get_final_url(client: &Client, url: &str) -> Result<String> {
    debug!("🔍 Getting final URL for: {}", url);
    
    let response = client
        .head(url)
        .send()
        .await
        .context("Failed to send HEAD request to URL")?;
    
    let final_url = response.url().to_string();
    
    if final_url != url {
        info!("🔄 Redirection found: {} → {}", url, final_url);
    } else {
        debug!("✅ No redirection for URL: {}", url);
    }
    
    Ok(final_url)
}
