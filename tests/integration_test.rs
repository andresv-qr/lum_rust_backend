#[cfg(test)]
mod integration_tests {
    use lum_rust_ws::processing::web_scraping::ocr_extractor;
    use lum_rust_ws::processing::web_scraping::data_parser;
    use std::fs;

    #[test]
    fn test_full_extraction_and_parsing_flow() {
        println!("\n🚀 Running full integration test: HTML -> ExtractedData -> Parsed Models");

        // 1. Read HTML content from a sample file
        let html_content = fs::read_to_string("webscrapy_htmlsample1.html")
            .expect("Failed to read webscrapy_htmlsample1.html");
        println!("✅ HTML file loaded successfully.");

        // 2. Call the main extractor
        let extracted_data = ocr_extractor::extract_main_info(&html_content)
            .expect("Failed to extract data using ocr_extractor");
        println!("✅ Data extracted successfully into ExtractedData struct.");
        assert!(!extracted_data.header.is_empty(), "Header should not be empty");
        assert!(!extracted_data.details.is_empty(), "Details should not be empty");

        // 3. Define a dummy URL for the parser
        let url = "https://dgi-fep.mef.gob.pa/Consultas/FacturasPorCUFE?CUFE=TEST_CUFE";

        // 4. Call the data parser
        let (header, details, _payments) = data_parser::parse_invoice_data(&extracted_data, url)
            .expect("Failed to parse extracted data into models");
        println!("✅ ExtractedData parsed successfully into final models.");

        // 5. Assertions to validate the parsed data
        println!("\n🔍 Validating parsed InvoiceHeader...");

        // Validate CUFE
        assert_eq!(header.cufe, "FE01200002679372-1-844914-7300002025051500311570140020317481978892", "CUFE does not match");
        println!("  ✅ CUFE: OK");

        // Validate Invoice Number
        assert_eq!(header.no_factura.as_deref(), Some("0031157014"), "Invoice number does not match");
        println!("  ✅ Invoice Number: OK");

        // Validate Date and Time
        let expected_datetime = chrono::NaiveDate::from_ymd_opt(2025, 5, 15).unwrap()
            .and_hms_opt(9, 50, 4).unwrap();
        assert_eq!(header.date, Some(expected_datetime), "Invoice date does not match");
        println!("  ✅ Date and Time: OK");

        // Validate Issuer
        assert_eq!(header.issuer_name.as_deref(), Some("Lum Corporation"), "Issuer name does not match");
        assert_eq!(header.issuer_ruc.as_deref(), Some("155622190-2-2017"), "Issuer RUC does not match");
        println!("  ✅ Issuer Info: OK");
        
        // Validate Totals
        assert_eq!(header.tot_amount, Some(rust_decimal::Decimal::new(10700, 2)), "Total amount does not match");
        assert_eq!(header.tot_itbms, Some(rust_decimal::Decimal::new(700, 2)), "Total ITBMS does not match");
        println!("  ✅ Totals: OK");

        // Validate Details (Line Items)
        assert_eq!(details.len(), 1, "Should have one line item");
        let item = &details[0];
        assert_eq!(item.description.as_deref(), Some("Asesoría y Desarrollo"), "Item description does not match");
        assert_eq!(item.quantity, Some(rust_decimal::Decimal::new(1, 0)), "Item quantity does not match");
        assert_eq!(item.unit_price, Some(rust_decimal::Decimal::new(10000, 2)), "Item unit price does not match");
        println!("  ✅ Line Items: OK");

        println!("\n🎉 Full integration test passed successfully!");
    }
}
