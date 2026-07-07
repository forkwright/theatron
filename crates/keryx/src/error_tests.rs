use super::*;

#[test]
fn server_error_displays_status_and_message() {
    let err = ApiError::Server {
        operation: "get_pulls",
        status: 500,
        message: "internal error".to_string(),
    };
    let s = format!("{err}");
    assert!(s.contains("get_pulls"), "operation in display: {s}");
    assert!(s.contains("500"), "status in display: {s}");
    assert!(s.contains("internal error"), "message in display: {s}");
}

#[test]
fn auth_error_displays_distinct_message() {
    let err = ApiError::Auth;
    let s = format!("{err}");
    assert!(s.contains("authentication"), "auth in display: {s}");
}

#[test]
fn invalid_token_error_displays_distinct_message() {
    let err = ApiError::InvalidToken;
    let s = format!("{err}");
    assert!(s.contains("invalid token"), "invalid token in display: {s}");
}

#[test]
fn timeout_error_displays_operation_and_seconds() {
    let err = ApiError::Timeout {
        operation: "get_runs",
        timeout_secs: 30,
    };
    let s = format!("{err}");
    assert!(s.contains("get_runs"), "operation in display: {s}");
    assert!(s.contains("30"), "timeout secs in display: {s}");
    assert!(s.contains("timed out"), "verb in display: {s}");
}

#[test]
fn rate_limited_error_displays_retry_after_when_present() {
    let err = ApiError::RateLimited {
        operation: "list_prs",
        retry_after_secs: Some(60),
    };
    let s = format!("{err}");
    assert!(s.contains("list_prs"), "operation in display: {s}");
    assert!(s.contains("429"), "status in display: {s}");
    assert!(s.contains("retry after 60s"), "retry-after in display: {s}");
}

#[test]
fn rate_limited_error_omits_retry_after_when_absent() {
    let err = ApiError::RateLimited {
        operation: "list_prs",
        retry_after_secs: None,
    };
    let s = format!("{err}");
    assert!(s.contains("list_prs"), "operation in display: {s}");
    assert!(s.contains("429"), "status in display: {s}");
    assert!(!s.contains("retry after"), "no retry-after when None: {s}");
}

#[test]
fn bad_response_error_displays_operation_and_source() {
    let json_err = serde_json::from_str::<i32>("not a number").unwrap_err();
    let err = ApiError::BadResponse {
        operation: "get_health",
        source: json_err,
    };
    let s = format!("{err}");
    assert!(s.contains("get_health"), "operation in display: {s}");
    assert!(s.contains("bad response body"), "verb in display: {s}");
}

#[test]
fn api_error_is_send_sync_std_error() {
    // WHY: the generic bound enforces Send + Sync + Error at
    // compile time (required for async tasks and `?` error
    // propagation); the runtime assertion verifies the Display
    // path through the Error impl.
    fn display_via_error<T: std::error::Error + Send + Sync>(err: &T) -> String {
        err.to_string()
    }
    assert_eq!(
        display_via_error(&ApiError::Auth),
        "authentication failed: token expired or invalid"
    );
}

#[test]
fn operation_returns_some_for_operation_carrying_variants() {
    let http = ApiError::Http {
        operation: "fetch_pulls",
        source: build_dummy_reqwest_error(),
    };
    let timeout = ApiError::Timeout {
        operation: "fetch_runs",
        timeout_secs: 30,
    };
    let server = ApiError::Server {
        operation: "list_prs",
        status: 500,
        message: String::new(),
    };
    let rate_limited = ApiError::RateLimited {
        operation: "list_users",
        retry_after_secs: None,
    };
    let bad_response = ApiError::BadResponse {
        operation: "get_repo",
        source: serde_json::from_str::<i32>("nope").unwrap_err(),
    };
    let body_too_large = ApiError::BodyTooLarge {
        operation: "get_diff",
        limit: 1024,
    };
    assert_eq!(http.operation(), Some("fetch_pulls"));
    assert_eq!(timeout.operation(), Some("fetch_runs"));
    assert_eq!(server.operation(), Some("list_prs"));
    assert_eq!(rate_limited.operation(), Some("list_users"));
    assert_eq!(bad_response.operation(), Some("get_repo"));
    assert_eq!(body_too_large.operation(), Some("get_diff"));
}

#[test]
fn body_too_large_error_displays_operation_and_limit() {
    let err = ApiError::BodyTooLarge {
        operation: "get_diff",
        limit: 1024,
    };
    let s = format!("{err}");
    assert!(s.contains("get_diff"), "operation in display: {s}");
    assert!(s.contains("1024"), "limit in display: {s}");
}

#[test]
fn operation_returns_none_for_context_free_variants() {
    assert_eq!(ApiError::Auth.operation(), None);
    assert_eq!(ApiError::InvalidToken.operation(), None);
}

#[test]
fn is_retryable_true_for_timeout() {
    let err = ApiError::Timeout {
        operation: "x",
        timeout_secs: 5,
    };
    assert!(err.is_retryable());
}

#[test]
fn is_retryable_true_for_rate_limited() {
    let with_retry = ApiError::RateLimited {
        operation: "x",
        retry_after_secs: Some(60),
    };
    let without_retry = ApiError::RateLimited {
        operation: "x",
        retry_after_secs: None,
    };
    assert!(with_retry.is_retryable());
    assert!(without_retry.is_retryable());
}

#[test]
fn is_retryable_true_for_5xx_server() {
    for status in [500, 502, 503, 504, 599] {
        let err = ApiError::Server {
            operation: "x",
            status,
            message: String::new(),
        };
        assert!(
            err.is_retryable(),
            "5xx status {status} should be retryable"
        );
    }
}

#[test]
fn is_retryable_false_for_4xx_server() {
    for status in [400, 403, 404, 422, 499] {
        let err = ApiError::Server {
            operation: "x",
            status,
            message: String::new(),
        };
        assert!(
            !err.is_retryable(),
            "4xx status {status} should not be retryable"
        );
    }
}

#[test]
fn is_retryable_false_for_terminal_variants() {
    let bad_response = ApiError::BadResponse {
        operation: "x",
        source: serde_json::from_str::<i32>("nope").unwrap_err(),
    };
    let body_too_large = ApiError::BodyTooLarge {
        operation: "x",
        limit: 1024,
    };
    assert!(!bad_response.is_retryable());
    assert!(!body_too_large.is_retryable());
    assert!(!ApiError::Auth.is_retryable());
    assert!(!ApiError::InvalidToken.is_retryable());
}

#[test]
fn status_code_returns_wire_status_for_server() {
    for status in [400, 404, 500, 503] {
        let err = ApiError::Server {
            operation: "x",
            status,
            message: String::new(),
        };
        assert_eq!(err.status_code(), Some(status));
    }
}

#[test]
fn status_code_returns_429_for_rate_limited() {
    let with = ApiError::RateLimited {
        operation: "x",
        retry_after_secs: Some(60),
    };
    let without = ApiError::RateLimited {
        operation: "x",
        retry_after_secs: None,
    };
    assert_eq!(with.status_code(), Some(429));
    assert_eq!(without.status_code(), Some(429));
}

#[test]
fn status_code_returns_none_for_response_less_variants() {
    let http = ApiError::Http {
        operation: "x",
        source: build_dummy_reqwest_error(),
    };
    let timeout = ApiError::Timeout {
        operation: "x",
        timeout_secs: 5,
    };
    let bad_response = ApiError::BadResponse {
        operation: "x",
        source: serde_json::from_str::<i32>("nope").unwrap_err(),
    };
    let body_too_large = ApiError::BodyTooLarge {
        operation: "x",
        limit: 1024,
    };
    assert_eq!(http.status_code(), None);
    assert_eq!(timeout.status_code(), None);
    assert_eq!(bad_response.status_code(), None);
    assert_eq!(body_too_large.status_code(), None);
    assert_eq!(ApiError::Auth.status_code(), None);
    assert_eq!(ApiError::InvalidToken.status_code(), None);
}

#[test]
fn retry_after_returns_some_for_rate_limited_with_header() {
    let err = ApiError::RateLimited {
        operation: "list_prs",
        retry_after_secs: Some(60),
    };
    assert_eq!(err.retry_after(), Some(60));
}

#[test]
fn retry_after_returns_none_for_rate_limited_without_header() {
    let err = ApiError::RateLimited {
        operation: "list_prs",
        retry_after_secs: None,
    };
    assert_eq!(err.retry_after(), None);
}

#[test]
fn retry_after_returns_none_for_other_variants() {
    let http = ApiError::Http {
        operation: "x",
        source: build_dummy_reqwest_error(),
    };
    let timeout = ApiError::Timeout {
        operation: "x",
        timeout_secs: 5,
    };
    let server = ApiError::Server {
        operation: "x",
        status: 503,
        message: String::new(),
    };
    let bad_response = ApiError::BadResponse {
        operation: "x",
        source: serde_json::from_str::<i32>("nope").unwrap_err(),
    };
    let body_too_large = ApiError::BodyTooLarge {
        operation: "x",
        limit: 1024,
    };
    assert_eq!(http.retry_after(), None);
    assert_eq!(timeout.retry_after(), None);
    assert_eq!(server.retry_after(), None);
    assert_eq!(bad_response.retry_after(), None);
    assert_eq!(body_too_large.retry_after(), None);
    assert_eq!(ApiError::Auth.retry_after(), None);
    assert_eq!(ApiError::InvalidToken.retry_after(), None);
}

#[test]
fn is_client_error_true_for_4xx_server() {
    for status in [400, 401, 403, 404, 422, 429, 499] {
        let err = ApiError::Server {
            operation: "x",
            status,
            message: String::new(),
        };
        assert!(err.is_client_error(), "{status} should be client error");
        assert!(
            !err.is_server_error(),
            "{status} should not be server error"
        );
    }
}

#[test]
fn is_server_error_true_for_5xx_server() {
    for status in [500, 502, 503, 504, 599] {
        let err = ApiError::Server {
            operation: "x",
            status,
            message: String::new(),
        };
        assert!(err.is_server_error(), "{status} should be server error");
        assert!(
            !err.is_client_error(),
            "{status} should not be client error"
        );
    }
}

#[test]
fn rate_limited_is_client_error_not_server_error() {
    let err = ApiError::RateLimited {
        operation: "x",
        retry_after_secs: Some(60),
    };
    assert!(err.is_client_error());
    assert!(!err.is_server_error());
}

#[test]
fn class_predicates_false_for_response_less_variants() {
    let http = ApiError::Http {
        operation: "x",
        source: build_dummy_reqwest_error(),
    };
    let timeout = ApiError::Timeout {
        operation: "x",
        timeout_secs: 5,
    };
    let bad_response = ApiError::BadResponse {
        operation: "x",
        source: serde_json::from_str::<i32>("nope").unwrap_err(),
    };
    let body_too_large = ApiError::BodyTooLarge {
        operation: "x",
        limit: 1024,
    };
    for err in [
        http,
        timeout,
        bad_response,
        body_too_large,
        ApiError::Auth,
        ApiError::InvalidToken,
    ] {
        assert!(!err.is_client_error(), "no response → not 4xx ({err:?})");
        assert!(!err.is_server_error(), "no response → not 5xx ({err:?})");
    }
}

#[test]
fn class_predicates_partition_server_status_codes() {
    // Every 4xx is client_error xor server_error; same for 5xx.
    // Boundary statuses (399, 600) fall in neither.
    let make = |status| ApiError::Server {
        operation: "x",
        status,
        message: String::new(),
    };
    assert!(!make(399).is_client_error() && !make(399).is_server_error());
    assert!(make(400).is_client_error() && !make(400).is_server_error());
    assert!(make(499).is_client_error() && !make(499).is_server_error());
    assert!(!make(500).is_client_error() && make(500).is_server_error());
    assert!(!make(599).is_client_error() && make(599).is_server_error());
    assert!(!make(600).is_client_error() && !make(600).is_server_error());
}

#[tokio::test]
async fn http_error_display_redacts_url_credentials() {
    // Regression for #182.4: reqwest::Error's own Display echoes
    // the request URL verbatim, which would leak a credential
    // embedded in the URL into logs. Connecting to a refused
    // loopback port yields a real reqwest::Error that carries a
    // parsed (credentialed) URL without needing real network
    // access — the connection fails, but the URL parsed fine.
    crate::install_test_crypto_provider();
    let source = reqwest::Client::new()
        .get("http://user:hunter2@127.0.0.1:1/path")
        .send()
        .await
        .expect_err("connecting to a refused port must fail");
    assert!(
        source.url().is_some(),
        "the error must carry the parsed request url"
    );

    let err = ApiError::Http {
        operation: "op",
        source,
    };
    let rendered = err.to_string();
    assert!(
        !rendered.contains("hunter2"),
        "credential must not appear verbatim in display: {rendered}"
    );
    assert!(
        !rendered.contains("user:hunter2@"),
        "userinfo must not appear verbatim in display: {rendered}"
    );
    assert!(rendered.contains("op"), "operation in display: {rendered}");
}

#[test]
fn http_error_display_passes_through_url_without_credentials() {
    // Non-credentialed (in this case url-less) errors must render
    // unchanged — redaction only engages when userinfo is present.
    let source = build_dummy_reqwest_error();
    assert!(
        source.url().is_none(),
        "a build-time parse failure carries no url"
    );
    let err = ApiError::Http {
        operation: "op",
        source,
    };
    // No panic, no spurious redaction marker, for a url-less error.
    assert!(!err.to_string().contains("[redacted]"));
}

/// Build a `reqwest::Error` synchronously for the
/// `operation_returns_some` test. `reqwest::Error` has no
/// public constructor; the cheapest path is to call the async
/// client's builder with an invalid URL, which fails
/// pre-network at `build()` time and returns a real
/// `reqwest::Error`.
fn build_dummy_reqwest_error() -> reqwest::Error {
    crate::install_test_crypto_provider();
    reqwest::Client::new().get("not a url").build().unwrap_err()
}
