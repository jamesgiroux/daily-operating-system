//! I651: Product Classification from Glean integration tests.
//!
//! Tests for extracting and upserting product classification data from Glean
//! Financial dimension responses to the account_products table.

#[cfg(test)]
mod tests {
    use dailyos_lib::intelligence::glean_provider::extract_products_from_response;
    use dailyos_lib::intelligence::io::{IntelligenceJson, ProductClassification, ProductInfo};

    // Helper: create a test IntelligenceJson with product classification
    fn intel_with_products(products: Vec<ProductInfo>) -> IntelligenceJson {
        IntelligenceJson {
            product_classification: Some(ProductClassification { products }),
            ..Default::default()
        }
    }

    // =========================================================================
    // Unit tests: extract_products_from_response
    // =========================================================================

    #[test]
    fn test_extract_products_from_response_cms_enhanced() {
        // Parse a response with CMS Enhanced tier
        let intel = intel_with_products(vec![ProductInfo {
            type_: Some("cms".to_string()),
            tier: Some("enhanced".to_string()),
            arr: Some(185400.0),
            billing_terms: Some("annual".to_string()),
        }]);

        let result = extract_products_from_response(&intel);
        assert!(result.is_ok());
        let products = result.unwrap();
        assert!(products.is_some());

        let products = products.unwrap();
        assert_eq!(products.len(), 1);
        let (product_type, tier, arr, billing_terms) = &products[0];
        assert_eq!(product_type, "cms");
        assert_eq!(tier.as_ref().unwrap(), "enhanced");
        assert_eq!(*arr, Some(185400.0));
        assert_eq!(billing_terms.as_ref().unwrap(), "annual");
    }

    #[test]
    fn test_extract_products_cms_and_analytics() {
        // Multiple products: CMS + Analytics
        let intel = intel_with_products(vec![
            ProductInfo {
                type_: Some("cms".to_string()),
                tier: Some("enhanced".to_string()),
                arr: Some(185400.0),
                billing_terms: Some("annual".to_string()),
            },
            ProductInfo {
                type_: Some("analytics".to_string()),
                tier: Some("standard".to_string()),
                arr: Some(50000.0),
                billing_terms: Some("annual".to_string()),
            },
        ]);

        let result = extract_products_from_response(&intel);
        assert!(result.is_ok());
        let products = result.unwrap();
        assert!(products.is_some());

        let products = products.unwrap();
        assert_eq!(products.len(), 2);
    }

    #[test]
    fn test_extract_products_missing_tier() {
        // Product with null tier
        let intel = intel_with_products(vec![ProductInfo {
            type_: Some("cms".to_string()),
            tier: None,
            arr: Some(100000.0),
            billing_terms: Some("monthly".to_string()),
        }]);

        let result = extract_products_from_response(&intel);
        assert!(result.is_ok());
        let products = result.unwrap();
        assert!(products.is_some());

        let products = products.unwrap();
        assert_eq!(products.len(), 1);
        let (_, tier, _, _) = &products[0];
        assert!(tier.is_none());
    }

    #[test]
    fn test_extract_products_empty_array() {
        // Empty products array
        let intel = intel_with_products(vec![]);

        let result = extract_products_from_response(&intel);
        assert!(result.is_ok());
        let products = result.unwrap();
        assert!(products.is_none()); // Empty → None (best-effort)
    }

    #[test]
    fn test_extract_products_none() {
        // No product_classification section
        let intel = IntelligenceJson::default();

        let result = extract_products_from_response(&intel);
        assert!(result.is_ok());
        let products = result.unwrap();
        assert!(products.is_none());
    }

    #[test]
    fn test_extract_products_missing_type() {
        // Product with no type_ (should be skipped)
        let intel = intel_with_products(vec![
            ProductInfo {
                type_: Some("cms".to_string()),
                tier: Some("enhanced".to_string()),
                arr: Some(100000.0),
                billing_terms: None,
            },
            ProductInfo {
                type_: None, // This one has no type
                tier: Some("standard".to_string()),
                arr: Some(50000.0),
                billing_terms: None,
            },
        ]);

        let result = extract_products_from_response(&intel);
        assert!(result.is_ok());
        let products = result.unwrap();
        assert!(products.is_some());

        let products = products.unwrap();
        assert_eq!(products.len(), 1); // Only the one with type_
        assert_eq!(&products[0].0, "cms");
    }

    // =========================================================================
    // Financial dimension prompt tests (document expected schema)
    // =========================================================================

    #[test]
    fn test_financial_dimension_product_classification_schema() {
        // Verify the productClassification field is structured correctly
        // This is primarily a documentation test

        let product = ProductInfo {
            type_: Some("cms".to_string()),
            tier: Some("enhanced".to_string()),
            arr: Some(185400.0),
            billing_terms: Some("annual".to_string()),
        };

        // Should serialize to JSON with correct field names
        let json = serde_json::to_string(&product).unwrap();
        assert!(json.contains("\"type\":\"cms\""));
        assert!(json.contains("\"tier\":\"enhanced\""));
        assert!(json.contains("\"arr\":185400"));
        assert!(json.contains("\"billingTerms\":\"annual\""));
    }

    // =========================================================================
    // Meeting prep context integration
    // =========================================================================

    #[test]
    fn test_product_classification_in_intelligence_context() {
        // Verify products appear in intelligence JSON serialization
        let intel = IntelligenceJson {
            product_classification: Some(ProductClassification {
                products: vec![ProductInfo {
                    type_: Some("cms".to_string()),
                    tier: Some("enhanced".to_string()),
                    arr: Some(185400.0),
                    billing_terms: Some("annual".to_string()),
                }],
            }),
            ..Default::default()
        };

        let json = serde_json::to_string(&intel).unwrap();
        assert!(json.contains("productClassification"));
        assert!(json.contains("\"type\":\"cms\""));
        assert!(json.contains("\"tier\":\"enhanced\""));
    }

    // =========================================================================
    // Best-effort behavior tests
    // =========================================================================

    #[test]
    fn test_extract_products_partial_data() {
        // All optional fields can be missing
        let intel = intel_with_products(vec![ProductInfo {
            type_: Some("cms".to_string()),
            tier: None,
            arr: None,
            billing_terms: None,
        }]);

        let result = extract_products_from_response(&intel);
        assert!(result.is_ok());
        let products = result.unwrap();
        assert!(products.is_some());

        let products = products.unwrap();
        assert_eq!(products.len(), 1);
        let (product_type, tier, arr, billing_terms) = &products[0];
        assert_eq!(product_type, "cms");
        assert!(tier.is_none());
        assert!(arr.is_none());
        assert!(billing_terms.is_none());
    }

    #[test]
    fn test_extract_products_preserves_arr_precision() {
        // ARR values should be preserved as floats
        let intel = intel_with_products(vec![ProductInfo {
            type_: Some("analytics".to_string()),
            tier: Some("standard".to_string()),
            arr: Some(47250.75),
            billing_terms: Some("annual".to_string()),
        }]);

        let result = extract_products_from_response(&intel);
        let products = result.unwrap().unwrap();
        let (_, _, arr, _) = &products[0];
        assert_eq!(*arr, Some(47250.75));
    }
}
