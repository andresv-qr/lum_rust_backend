use std::sync::Arc;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Serialize, Deserialize};
use axum::{
    extract::Request,
    middleware::Next,
    response::Response,
    http::{HeaderValue, header::CACHE_CONTROL},
};
use tracing::info;

/// Prometheus-style metrics collector
#[derive(Debug, Clone)]
pub struct MetricsCollector {
    pub counters: Arc<RwLock<HashMap<String, u64>>>,
    pub histograms: Arc<RwLock<HashMap<String, Vec<f64>>>>,
    pub gauges: Arc<RwLock<HashMap<String, f64>>>,
    pub request_durations: Arc<RwLock<HashMap<String, Vec<Duration>>>>,
}

impl MetricsCollector {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            request_durations: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Increment a counter metric
    pub async fn increment_counter(&self, name: &str, labels: Option<&str>) {
        let key = if let Some(labels) = labels {
            format!("{}_{}", name, labels)
        } else {
            name.to_string()
        };
        
        let mut counters = self.counters.write().await;
        *counters.entry(key).or_insert(0) += 1;
    }

    /// Record a histogram value
    pub async fn record_histogram(&self, name: &str, value: f64, labels: Option<&str>) {
        let key = if let Some(labels) = labels {
            format!("{}_{}", name, labels)
        } else {
            name.to_string()
        };
        
        let mut histograms = self.histograms.write().await;
        histograms.entry(key.clone()).or_insert_with(Vec::new).push(value);
        
        // Keep only last 1000 values for memory efficiency
        let values = histograms.get_mut(&key).unwrap();
        if values.len() > 1000 {
            values.remove(0);
        }
    }

    /// Set a gauge value
    pub async fn set_gauge(&self, name: &str, value: f64, labels: Option<&str>) {
        let key = if let Some(labels) = labels {
            format!("{}_{}", name, labels)
        } else {
            name.to_string()
        };
        
        let mut gauges = self.gauges.write().await;
        gauges.insert(key, value);
    }

    /// Record request duration
    pub async fn record_request_duration(&self, method: &str, path: &str, duration: Duration) {
        let key = format!("{}_{}", method, sanitize_path(path));
        
        let mut durations = self.request_durations.write().await;
        durations.entry(key.clone()).or_insert_with(Vec::new).push(duration);
        
        // Keep only last 500 values for memory efficiency
        let values = durations.get_mut(&key).unwrap();
        if values.len() > 500 {
            values.remove(0);
        }
    }

    /// Get metrics summary in Prometheus format
    pub async fn get_prometheus_metrics(&self) -> String {
        let mut output = String::new();
        
        // Counters
        let counters = self.counters.read().await;
        for (name, value) in counters.iter() {
            output.push_str(&format!("# TYPE {} counter\n", name));
            output.push_str(&format!("{} {}\n", name, value));
        }
        
        // Gauges
        let gauges = self.gauges.read().await;
        for (name, value) in gauges.iter() {
            output.push_str(&format!("# TYPE {} gauge\n", name));
            output.push_str(&format!("{} {}\n", name, value));
        }
        
        // Histograms (simplified as summary statistics)
        let histograms = self.histograms.read().await;
        for (name, values) in histograms.iter() {
            if !values.is_empty() {
                let sum: f64 = values.iter().sum();
                let count = values.len();
                let avg = sum / count as f64;
                
                output.push_str(&format!("# TYPE {}_sum counter\n", name));
                output.push_str(&format!("{}_sum {}\n", name, sum));
                output.push_str(&format!("# TYPE {}_count counter\n", name));
                output.push_str(&format!("{}_count {}\n", name, count));
                output.push_str(&format!("# TYPE {}_avg gauge\n", name));
                output.push_str(&format!("{}_avg {}\n", name, avg));
            }
        }
        
        // Request durations
        let durations = self.request_durations.read().await;
        for (name, values) in durations.iter() {
            if !values.is_empty() {
                let sum_ms: f64 = values.iter().map(|d| d.as_millis() as f64).sum();
                let count = values.len();
                let avg_ms = sum_ms / count as f64;
                
                output.push_str(&format!("# TYPE http_request_duration_seconds_sum counter\n"));
                output.push_str(&format!("http_request_duration_seconds_sum{{method_path=\"{}\"}} {}\n", 
                    name, sum_ms / 1000.0));
                output.push_str(&format!("# TYPE http_request_duration_seconds_count counter\n"));
                output.push_str(&format!("http_request_duration_seconds_count{{method_path=\"{}\"}} {}\n", 
                    name, count));
                output.push_str(&format!("# TYPE http_request_duration_seconds_avg gauge\n"));
                output.push_str(&format!("http_request_duration_seconds_avg{{method_path=\"{}\"}} {}\n", 
                    name, avg_ms / 1000.0));
            }
        }
        
        output
    }

    /// Get JSON metrics summary
    pub async fn get_json_metrics(&self) -> serde_json::Value {
        let counters = self.counters.read().await;
        let gauges = self.gauges.read().await;
        let histograms = self.histograms.read().await;
        let durations = self.request_durations.read().await;
        
        serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "counters": *counters,
            "gauges": *gauges,
            "histograms": histograms.iter().map(|(k, v)| {
                (k, serde_json::json!({
                    "count": v.len(),
                    "sum": v.iter().sum::<f64>(),
                    "avg": if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 },
                    "min": v.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
                    "max": v.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b))
                }))
            }).collect::<HashMap<_, _>>(),
            "request_durations": durations.iter().map(|(k, v)| {
                let ms_values: Vec<f64> = v.iter().map(|d| d.as_millis() as f64).collect();
                (k, serde_json::json!({
                    "count": ms_values.len(),
                    "sum_ms": ms_values.iter().sum::<f64>(),
                    "avg_ms": if ms_values.is_empty() { 0.0 } else { ms_values.iter().sum::<f64>() / ms_values.len() as f64 },
                    "min_ms": ms_values.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
                    "max_ms": ms_values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)),
                    "p95_ms": percentile(&ms_values, 0.95),
                    "p99_ms": percentile(&ms_values, 0.99)
                }))
            }).collect::<HashMap<_, _>>()
        })
    }
}

/// Advanced metrics middleware
pub async fn metrics_middleware(
    req: Request,
    next: Next,
) -> Response {
    let start_time = Instant::now();
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    
    // Process request
    let response = next.run(req).await;
    
    let duration = start_time.elapsed();
    let status = response.status().as_u16();
    
    // Extract metrics collector from response extensions if available
    // (This would be injected via state in real implementation)
    
    info!(
        method = %method,
        path = %path,
        status = %status,
        duration_ms = %duration.as_millis(),
        "HTTP request completed"
    );
    
    // Add performance headers
    let mut response = response;
    if let Ok(duration_header) = HeaderValue::from_str(&duration.as_millis().to_string()) {
        response.headers_mut().insert("X-Response-Time-Ms", duration_header);
    }
    
    response
}

/// Cache-aware response middleware
pub async fn cache_middleware(
    req: Request,
    next: Next,
) -> Response {
    let if_none_match = req.headers()
        .get("if-none-match")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());
    
    let mut response = next.run(req).await;
    
    // Add cache headers for GET requests
    if response.status().is_success() {
        let cache_control = if response.headers().contains_key("etag") {
            "private, max-age=300, must-revalidate" // 5 minutes with ETag
        } else {
            "private, max-age=60, must-revalidate" // 1 minute without ETag
        };
        
        if let Ok(cache_header) = HeaderValue::from_str(cache_control) {
            response.headers_mut().insert(CACHE_CONTROL, cache_header);
        }
        
        // If client sent If-None-Match and we have ETag, check for 304
        if let (Some(client_etag), Some(response_etag)) = (
            if_none_match,
            response.headers().get("etag").and_then(|h| h.to_str().ok())
        ) {
            if client_etag.trim_matches('"') == response_etag.trim_matches('"') {
                info!("ETag match, returning 304 Not Modified");
                return Response::builder()
                    .status(304)
                    .header("etag", response_etag)
                    .header(CACHE_CONTROL, cache_control)
                    .body(axum::body::Body::empty())
                    .unwrap();
            }
        }
    }
    
    response
}

// Helper functions
fn sanitize_path(path: &str) -> String {
    path.replace(['/', '-'], "_")
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '_')
        .collect()
}

fn percentile(values: &[f64], p: f64) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    
    let index = ((values.len() as f64 - 1.0) * p) as usize;
    sorted[index.min(sorted.len() - 1)]
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheck {
    pub status: String,
    pub timestamp: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub database: DatabaseHealth,
    pub redis: RedisHealth,
    pub memory_usage: MemoryUsage,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DatabaseHealth {
    pub status: String,
    pub connection_pool_size: u32,
    pub active_connections: u32,
    pub last_query_duration_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RedisHealth {
    pub status: String,
    pub connection_count: u32,
    pub last_ping_duration_ms: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MemoryUsage {
    pub allocated_bytes: u64,
    pub heap_size_bytes: u64,
    pub peak_allocated_bytes: u64,
}
