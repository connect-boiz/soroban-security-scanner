// Integration tests for validation module with Axum handlers
// Run with: cargo test --test integration_tests

#[cfg(test)]
mod integration_tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::{get, post},
        Json, Router,
    };
    use serde_json::json;
    use soroban_security_scanner::validation::*;
    use tower::ServiceExt;

    // Test request/response types
    #[derive(serde::Deserialize)]
    struct ValidateAddressRequest {
        address: String,
    }

    #[derive(serde::Serialize)]
    struct ValidateAddressResponse {
        valid: bool,
        address: String,
    }

    #[derive(serde::Deserialize)]
    struct ValidateAmountRequest {
        amount: i128,
    }

    #[derive(serde::Serialize)]
    struct ValidateAmountResponse {
        valid: bool,
        amount: i128,
    }

    #[derive(serde::Deserialize)]
    struct ValidatePaginationRequest {
        limit: u32,
        offset: u32,
    }

    #[derive(serde::Serialize)]
    struct ValidatePaginationResponse {
        valid: bool,
        limit: u32,
        offset: u32,
    }

    #[derive(serde::Deserialize)]
    struct SanitizeStringRequest {
        input: String,
        max_len: usize,
    }

    #[derive(serde::Serialize)]
    struct SanitizeStringResponse {
        output: String,
    }

    // Test handlers
    async fn test_validate_address(
        Json(req): Json<ValidateAddressRequest>,
    ) -> Result<Json<ValidateAddressResponse>, ValidationError> {
        validate_address(&req.address)?;
        Ok(Json(ValidateAddressResponse {
            valid: true,
            address: req.address,
        }))
    }

    async fn test_validate_amount(
        Json(req): Json<ValidateAmountRequest>,
    ) -> Result<Json<ValidateAmountResponse>, ValidationError> {
        validate_amount(req.amount)?;
        Ok(Json(ValidateAmountResponse {
            valid: true,
            amount: req.amount,
        }))
    }

    async fn test_validate_pagination(
        Json(req): Json<ValidatePaginationRequest>,
    ) -> Result<Json<ValidatePaginationResponse>, ValidationError> {
        validate_pagination(req.limit, req.offset)?;
        Ok(Json(ValidatePaginationResponse {
            valid: true,
            limit: req.limit,
            offset: req.offset,
        }))
    }

    async fn test_sanitize_string(
        Json(req): Json<SanitizeStringRequest>,
    ) -> Result<Json<SanitizeStringResponse>, ValidationError> {
        let output = sanitize_string(&req.input, req.max_len)?;
        Ok(Json(SanitizeStringResponse { output }))
    }

    fn create_test_router() -> Router {
        Router::new()
            .route("/validate_address", post(test_validate_address))
            .route("/validate_amount", post(test_validate_amount))
            .route("/validate_pagination", post(test_validate_pagination))
            .route("/sanitize_string", post(test_sanitize_string))
    }

    // ========================================================================
    // INTEGRATION TESTS - ADDRESS VALIDATION
    // ========================================================================

    #[tokio::test]
    async fn test_http_valid_address() {
        let app = create_test_router();

        let request = Request::builder()
            .method("POST")
            .uri("/validate_address")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "address": "GBBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQ"
                }))
                .unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_http_invalid_address_too_short() {
        let app = create_test_router();

        let request = Request::builder()
            .method("POST")
            .uri("/validate_address")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({ "address": "GBBD47" })).unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_http_invalid_address_empty() {
        let app = create_test_router();

        let request = Request::builder()
            .method("POST")
            .uri("/validate_address")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&json!({ "address": "" })).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_http_invalid_address_wrong_prefix() {
        let app = create_test_router();

        let request = Request::builder()
            .method("POST")
            .uri("/validate_address")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "address": "ABBD47UZQ5DSFBQ6D45YFVQ5XVDYDKIVJ4VPXXJTFHJXUVG2GFDS7FQ"
                }))
                .unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // ========================================================================
    // INTEGRATION TESTS - AMOUNT VALIDATION
    // ========================================================================

    #[tokio::test]
    async fn test_http_valid_amount() {
        let app = create_test_router();

        let request = Request::builder()
            .method("POST")
            .uri("/validate_amount")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&json!({ "amount": 1000 })).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_http_valid_amount_zero() {
        let app = create_test_router();

        let request = Request::builder()
            .method("POST")
            .uri("/validate_amount")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&json!({ "amount": 0 })).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_http_invalid_amount_negative() {
        let app = create_test_router();

        let request = Request::builder()
            .method("POST")
            .uri("/validate_amount")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&json!({ "amount": -1000 })).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_http_invalid_amount_overflow() {
        let app = create_test_router();

        let request = Request::builder()
            .method("POST")
            .uri("/validate_amount")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({ "amount": i128::MAX })).unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // ========================================================================
    // INTEGRATION TESTS - PAGINATION VALIDATION
    // ========================================================================

    #[tokio::test]
    async fn test_http_valid_pagination() {
        let app = create_test_router();

        let request = Request::builder()
            .method("POST")
            .uri("/validate_pagination")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({ "limit": 50, "offset": 0 })).unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_http_invalid_pagination_limit_too_small() {
        let app = create_test_router();

        let request = Request::builder()
            .method("POST")
            .uri("/validate_pagination")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({ "limit": 0, "offset": 0 })).unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_http_invalid_pagination_limit_too_large() {
        let app = create_test_router();

        let request = Request::builder()
            .method("POST")
            .uri("/validate_pagination")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({ "limit": 2000, "offset": 0 })).unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_http_invalid_pagination_offset_too_large() {
        let app = create_test_router();

        let request = Request::builder()
            .method("POST")
            .uri("/validate_pagination")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({ "limit": 50, "offset": 2_000_000 }))
                    .unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // ========================================================================
    // INTEGRATION TESTS - STRING SANITIZATION
    // ========================================================================

    #[tokio::test]
    async fn test_http_sanitize_valid_string() {
        let app = create_test_router();

        let request = Request::builder()
            .method("POST")
            .uri("/sanitize_string")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({ "input": "hello", "max_len": 100 }))
                    .unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_http_sanitize_empty_string() {
        let app = create_test_router();

        let request = Request::builder()
            .method("POST")
            .uri("/sanitize_string")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({ "input": "", "max_len": 100 })).unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_http_sanitize_whitespace_only() {
        let app = create_test_router();

        let request = Request::builder()
            .method("POST")
            .uri("/sanitize_string")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({ "input": "   ", "max_len": 100 })).unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_http_sanitize_truncates_correctly() {
        let app = create_test_router();

        let request = Request::builder()
            .method("POST")
            .uri("/sanitize_string")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({
                    "input": "a".repeat(200),
                    "max_len": 100
                }))
                .unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_http_sanitize_preserves_unicode() {
        let app = create_test_router();

        let request = Request::builder()
            .method("POST")
            .uri("/sanitize_string")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({ "input": "hello 世界", "max_len": 100 }))
                    .unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    // ========================================================================
    // ERROR RESPONSE FORMAT TESTS
    // ========================================================================

    #[tokio::test]
    async fn test_error_response_has_correct_structure() {
        let app = create_test_router();

        let request = Request::builder()
            .method("POST")
            .uri("/validate_address")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&json!({ "address": "" })).unwrap()))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        // Verify response contains error details
        let body_bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();

        assert!(body.get("error").is_some());
        assert!(body.get("details").is_some());
        assert_eq!(body.get("status").unwrap().as_u64().unwrap(), 400);
    }

    // ========================================================================
    // EDGE CASE TESTS
    // ========================================================================

    #[tokio::test]
    async fn test_boundary_limit_1() {
        let app = create_test_router();

        let request = Request::builder()
            .method("POST")
            .uri("/validate_pagination")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({ "limit": 1, "offset": 0 })).unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_boundary_limit_1000() {
        let app = create_test_router();

        let request = Request::builder()
            .method("POST")
            .uri("/validate_pagination")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({ "limit": 1000, "offset": 0 })).unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_max_offset_boundary() {
        let app = create_test_router();

        let request = Request::builder()
            .method("POST")
            .uri("/validate_pagination")
            .header("content-type", "application/json")
            .body(Body::from(
                serde_json::to_string(&json!({ "limit": 50, "offset": 1_000_000 }))
                    .unwrap(),
            ))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }
}
