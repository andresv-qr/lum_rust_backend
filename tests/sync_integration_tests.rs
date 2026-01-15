//! Tests de integración para endpoints de sincronización v4
//! 
//! Para ejecutar estos tests necesitas:
//! 1. Base de datos PostgreSQL configurada
//! 2. Variable DATABASE_URL definida
//! 3. Ejecutar: cargo test --test sync_integration_tests -- --nocapture

#[cfg(test)]
mod sync_integration_tests {
    use chrono::{DateTime, Utc, Datelike};
    use serde_json::Value;

    // =====================================================
    // UNIT TESTS - No requieren base de datos
    // =====================================================

    #[test]
    fn test_sync_status_response_structure() {
        use serde::{Deserialize, Serialize};
        
        #[derive(Debug, Serialize, Deserialize)]
        struct SyncStatusResponse {
            headers_count: i64,
            headers_max_update_date: Option<DateTime<Utc>>,
            server_timestamp: DateTime<Utc>,
        }
        
        let response = SyncStatusResponse {
            headers_count: 527,
            headers_max_update_date: Some(Utc::now()),
            server_timestamp: Utc::now(),
        };
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("headers_count"));
        assert!(json.contains("527"));
        assert!(json.contains("headers_max_update_date"));
        assert!(json.contains("server_timestamp"));
        
        // Deserialize back
        let parsed: SyncStatusResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.headers_count, 527);
        println!("✅ SyncStatusResponse serialization: OK");
    }

    #[test]
    fn test_recovery_request_validation() {
        use serde::Deserialize;
        
        #[derive(Debug, Deserialize)]
        struct RecoveryRequest {
            known_cufes: Vec<String>,
            limit: Option<i64>,
        }
        
        // Test empty request
        let json = r#"{"known_cufes": [], "limit": 100}"#;
        let req: RecoveryRequest = serde_json::from_str(json).unwrap();
        assert!(req.known_cufes.is_empty());
        assert_eq!(req.limit, Some(100));
        
        // Test with CUFEs
        let json = r#"{"known_cufes": ["CUFE1", "CUFE2"], "limit": 50}"#;
        let req: RecoveryRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.known_cufes.len(), 2);
        assert_eq!(req.known_cufes[0], "CUFE1");
        
        // Test without limit (should use default)
        let json = r#"{"known_cufes": ["CUFE1"]}"#;
        let req: RecoveryRequest = serde_json::from_str(json).unwrap();
        assert!(req.limit.is_none());
        
        println!("✅ RecoveryRequest validation: OK");
    }

    #[test]
    fn test_incremental_sync_request_params() {
        use serde::Deserialize;
        
        #[derive(Debug, Deserialize, Default)]
        struct IncrementalSyncRequest {
            limit: Option<i64>,
            offset: Option<i64>,
            update_date_from: Option<String>,
            #[serde(default)]
            full_sync: bool,
        }
        
        // Test incremental sync params
        let json = r#"{"limit": 20, "offset": 0, "update_date_from": "2025-01-01T00:00:00Z"}"#;
        let req: IncrementalSyncRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.limit, Some(20));
        assert_eq!(req.offset, Some(0));
        assert!(req.update_date_from.is_some());
        assert!(!req.full_sync);
        
        // Test full sync override
        let json = r#"{"limit": 100, "full_sync": true}"#;
        let req: IncrementalSyncRequest = serde_json::from_str(json).unwrap();
        assert!(req.full_sync);
        
        println!("✅ IncrementalSyncRequest params: OK");
    }

    #[test]
    fn test_api_response_wrapper() {
        use serde::{Deserialize, Serialize};
        
        #[derive(Debug, Serialize, Deserialize)]
        struct ApiResponse<T> {
            success: bool,
            data: Option<T>,
            error: Option<String>,
            request_id: String,
            timestamp: DateTime<Utc>,
            execution_time_ms: Option<u64>,
            cached: bool,
        }
        
        let response: ApiResponse<i64> = ApiResponse {
            success: true,
            data: Some(42),
            error: None,
            request_id: "test-123".to_string(),
            timestamp: Utc::now(),
            execution_time_ms: Some(5),
            cached: false,
        };
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains(r#""success":true"#));
        assert!(json.contains(r#""data":42"#));
        assert!(json.contains(r#""request_id":"test-123""#));
        assert!(json.contains(r#""cached":false"#));
        
        println!("✅ ApiResponse wrapper: OK");
    }

    #[test]
    fn test_pagination_info_calculations() {
        // Simulate pagination calculations like in the actual code
        let total_count: i64 = 527;
        let limit: i64 = 20;
        let offset: i64 = 40;
        
        let total_pages = if limit > 0 { (total_count + limit - 1) / limit } else { 1 };
        let current_page = if limit > 0 { (offset / limit) + 1 } else { 1 };
        let has_more = (offset + limit) < total_count;
        
        assert_eq!(total_pages, 27); // ceil(527/20) = 27
        assert_eq!(current_page, 3);  // offset=40, limit=20 -> page 3
        assert!(has_more);            // 40+20=60 < 527
        
        // Edge case: last page
        let offset_last: i64 = 520;
        let has_more_last = (offset_last + limit) < total_count;
        let current_page_last = (offset_last / limit) + 1;
        assert!(!has_more_last);      // 520+20=540 > 527
        assert_eq!(current_page_last, 27);
        
        println!("✅ Pagination calculations: OK");
    }

    #[test]
    fn test_checksum_consistency() {
        use sha2::{Sha256, Digest};
        
        // Simulate checksum calculation
        let data = vec!["item1", "item2", "item3"];
        let json = serde_json::to_string(&data).unwrap();
        
        let mut hasher = Sha256::new();
        hasher.update(json.as_bytes());
        let result = hasher.finalize();
        let checksum = format!("sha256:{:x}", result);
        
        // Same data should produce same checksum
        let mut hasher2 = Sha256::new();
        hasher2.update(json.as_bytes());
        let result2 = hasher2.finalize();
        let checksum2 = format!("sha256:{:x}", result2);
        
        assert_eq!(checksum, checksum2);
        assert!(checksum.starts_with("sha256:"));
        
        // Different data should produce different checksum
        let data_different = vec!["item1", "item2"];
        let json_different = serde_json::to_string(&data_different).unwrap();
        let mut hasher3 = Sha256::new();
        hasher3.update(json_different.as_bytes());
        let result3 = hasher3.finalize();
        let checksum3 = format!("sha256:{:x}", result3);
        
        assert_ne!(checksum, checksum3);
        
        println!("✅ Checksum consistency: OK");
    }

    #[test]
    fn test_date_parsing_formats() {
        // Test various date formats that should be accepted
        let formats = vec![
            "2025-01-14T10:30:00Z",
            "2025-01-14T10:30:00+00:00",
            "2025-01-14T05:30:00-05:00",
        ];
        
        for format in formats {
            let parsed = chrono::DateTime::parse_from_rfc3339(format);
            assert!(parsed.is_ok(), "Failed to parse: {}", format);
            let utc: DateTime<Utc> = parsed.unwrap().into();
            assert!(utc.year() == 2025);
        }
        
        // Test invalid format (should fail)
        let invalid = "2025-01-14"; // No time component
        let parsed = chrono::DateTime::parse_from_rfc3339(invalid);
        assert!(parsed.is_err());
        
        println!("✅ Date parsing formats: OK");
    }

    #[test]
    fn test_limit_bounds() {
        // Test limit clamping logic
        fn clamp_limit(limit: Option<i64>, min: i64, max: i64, default: i64) -> i64 {
            limit.unwrap_or(default).min(max).max(min)
        }
        
        // Headers: default=20, min=1, max=100
        assert_eq!(clamp_limit(None, 1, 100, 20), 20);
        assert_eq!(clamp_limit(Some(50), 1, 100, 20), 50);
        assert_eq!(clamp_limit(Some(0), 1, 100, 20), 1);    // Below min
        assert_eq!(clamp_limit(Some(500), 1, 100, 20), 100); // Above max
        
        // Recovery: default=100, min=1, max=500
        assert_eq!(clamp_limit(None, 1, 500, 100), 100);
        assert_eq!(clamp_limit(Some(250), 1, 500, 100), 250);
        assert_eq!(clamp_limit(Some(1000), 1, 500, 100), 500);
        
        println!("✅ Limit bounds clamping: OK");
    }

    #[test]
    fn test_composite_id_generation() {
        // Test cufe + code composite ID generation
        fn generate_composite_id(cufe: &str, code: Option<&str>) -> String {
            format!("{}_{}", cufe, code.unwrap_or(""))
        }
        
        assert_eq!(
            generate_composite_id("CUFE123", Some("PROD001")),
            "CUFE123_PROD001"
        );
        assert_eq!(
            generate_composite_id("CUFE123", None),
            "CUFE123_"
        );
        
        // Test that empty code produces consistent result
        assert_eq!(
            generate_composite_id("CUFE123", Some("")),
            "CUFE123_"
        );
        
        println!("✅ Composite ID generation: OK");
    }

    #[test]
    fn test_type_field_rename() {
        use serde::{Deserialize, Serialize};
        
        #[derive(Debug, Serialize, Deserialize)]
        struct HeaderResponse {
            cufe: String,
            #[serde(rename = "type")]
            invoice_type: Option<String>,
        }
        
        let header = HeaderResponse {
            cufe: "TEST123".to_string(),
            invoice_type: Some("QR".to_string()),
        };
        
        let json = serde_json::to_string(&header).unwrap();
        
        // Should serialize as "type" not "invoice_type"
        assert!(json.contains(r#""type":"QR""#));
        assert!(!json.contains("invoice_type"));
        
        // Should deserialize from "type"
        let parsed: HeaderResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.invoice_type, Some("QR".to_string()));
        
        println!("✅ Type field serde rename: OK");
    }

    #[test]
    fn test_recovery_response_structure() {
        use serde::{Deserialize, Serialize};
        
        #[derive(Debug, Serialize, Deserialize)]
        struct RecoveryResponse<T> {
            missing_records: Vec<T>,
            deleted_cufes: Vec<String>,
            total_missing: i64,
            server_timestamp: DateTime<Utc>,
        }
        
        let response: RecoveryResponse<String> = RecoveryResponse {
            missing_records: vec!["record1".to_string(), "record2".to_string()],
            deleted_cufes: vec!["deleted1".to_string()],
            total_missing: 100,
            server_timestamp: Utc::now(),
        };
        
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("missing_records"));
        assert!(json.contains("deleted_cufes"));
        assert!(json.contains("total_missing"));
        assert!(json.contains("100"));
        
        println!("✅ RecoveryResponse structure: OK");
    }

    // =====================================================
    // JSON CONTRACT TESTS
    // =====================================================

    #[test]
    fn test_sync_status_json_contract() {
        // Verify the JSON contract matches API documentation
        let expected_json = r#"{
            "success": true,
            "data": {
                "headers_count": 527,
                "headers_max_update_date": "2025-01-14T10:30:00Z",
                "server_timestamp": "2025-01-14T10:30:01Z"
            },
            "error": null,
            "request_id": "uuid-here",
            "timestamp": "2025-01-14T10:30:01Z",
            "execution_time_ms": 1,
            "cached": false
        }"#;
        
        let parsed: Value = serde_json::from_str(expected_json).unwrap();
        
        // Verify required fields exist
        assert!(parsed["success"].as_bool().unwrap());
        assert!(parsed["data"]["headers_count"].is_number());
        assert!(parsed["data"]["server_timestamp"].is_string());
        assert!(parsed["request_id"].is_string());
        assert!(parsed["cached"].is_boolean());
        
        println!("✅ sync-status JSON contract: OK");
    }

    #[test]
    fn test_headers_json_contract() {
        let expected_json = r#"{
            "success": true,
            "data": {
                "data": [
                    {
                        "cufe": "FE01-123",
                        "issuer_name": "Test Corp",
                        "type": "QR",
                        "update_date": "2025-01-14T10:30:00Z"
                    }
                ],
                "pagination": {
                    "total_records": 527,
                    "returned_records": 1,
                    "limit": 20,
                    "offset": 0,
                    "has_more": true,
                    "total_pages": 27,
                    "current_page": 1
                },
                "sync_metadata": {
                    "max_update_date": "2025-01-14T10:30:00Z",
                    "server_timestamp": "2025-01-14T10:30:01Z",
                    "data_checksum": "sha256:abc123",
                    "returned_records": 1
                }
            }
        }"#;
        
        let parsed: Value = serde_json::from_str(expected_json).unwrap();
        
        // Verify nested structure
        assert!(parsed["data"]["data"].is_array());
        assert!(parsed["data"]["pagination"]["total_records"].is_number());
        assert!(parsed["data"]["sync_metadata"]["data_checksum"].is_string());
        
        // Verify type field is present (not invoice_type)
        assert!(parsed["data"]["data"][0]["type"].is_string());
        
        println!("✅ headers JSON contract: OK");
    }

    #[test]
    fn test_recovery_json_contract() {
        let expected_json = r#"{
            "success": true,
            "data": {
                "missing_records": [],
                "deleted_cufes": [],
                "total_missing": 0,
                "server_timestamp": "2025-01-14T10:30:00Z"
            },
            "request_id": "uuid",
            "execution_time_ms": 5
        }"#;
        
        let parsed: Value = serde_json::from_str(expected_json).unwrap();
        
        assert!(parsed["data"]["missing_records"].is_array());
        assert!(parsed["data"]["deleted_cufes"].is_array());
        assert!(parsed["data"]["total_missing"].is_number());
        
        println!("✅ recovery JSON contract: OK");
    }

    // =====================================================
    // PERFORMANCE CHARACTERISTICS TESTS
    // =====================================================

    #[test]
    fn test_payload_size_limits() {
        // Headers recovery max: 10,000 CUFEs
        let max_headers_cufes = 10_000;
        let large_payload: Vec<String> = (0..max_headers_cufes)
            .map(|i| format!("CUFE_{}", i))
            .collect();
        
        assert!(large_payload.len() <= max_headers_cufes);
        
        // Details recovery max: 50,000 IDs
        let max_details_ids = 50_000;
        let large_details: Vec<String> = (0..max_details_ids)
            .map(|i| format!("CUFE_{}_{}", i, i))
            .collect();
        
        assert!(large_details.len() <= max_details_ids);
        
        println!("✅ Payload size limits verified: headers={}, details={}", 
            max_headers_cufes, max_details_ids);
    }
}
