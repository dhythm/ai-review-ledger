use std::sync::{Arc, Mutex};

use axum::extract::rejection::JsonRejection;
use axum::extract::{Path, Query, State};
use axum::http::{Method, StatusCode};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde::Serialize;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::db::ListFilter;
use crate::models::{normalize, Review, ReviewInput, DECISIONS, RISKS};

mod db;
mod models;

#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<rusqlite::Connection>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReviewListResponse {
    items: Vec<Review>,
    total: i64,
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn bad(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            message: message.into(),
        }
    }

    fn not_found() -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            message: "レビューが見つかりません".into(),
        }
    }
}

impl axum::response::IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let body = Json(ErrorBody {
            error: self.message,
        });
        (self.status, body).into_response()
    }
}

#[derive(Deserialize)]
struct ListParams {
    q: Option<String>,
    decision: Option<String>,
    risk: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let db_path = std::env::var("LEDGER_DB")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| db::default_db_path());
    let conn = db::init_db(&db_path)?;
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let addr = format!("127.0.0.1:{port}");

    tracing::info!("database {}", db_path.display());
    tracing::info!("listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        router(AppState {
            db: Arc::new(Mutex::new(conn)),
        }),
    )
    .with_graceful_shutdown(async {
        let _ = tokio::signal::ctrl_c().await;
    })
    .await?;
    Ok(())
}

fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(Any);

    Router::new()
        .route("/api/health", get(health))
        .route("/api/reviews", get(list_reviews).post(create_review))
        .route(
            "/api/reviews/:id",
            get(get_review).put(update_review).delete(delete_review),
        )
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .with_state(state)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "ok": true }))
}

async fn list_reviews(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<ReviewListResponse>, ApiError> {
    if let Some(decision) = params.decision.as_deref().map(str::trim) {
        if !decision.is_empty() && !DECISIONS.contains(&decision) {
            return Err(ApiError::bad("判断の値が不正です"));
        }
    }
    if let Some(risk) = params.risk.as_deref().map(str::trim) {
        if !risk.is_empty() && !RISKS.contains(&risk) {
            return Err(ApiError::bad("リスクの値が不正です"));
        }
    }

    let conn = state.db.lock().expect("db lock");
    let listed = db::list_reviews(
        &conn,
        ListFilter {
            query: params.q.as_deref(),
            decision: params.decision.as_deref(),
            risk: params.risk.as_deref(),
        },
    )
    .map_err(db_error)?;
    Ok(Json(ReviewListResponse {
        items: listed.items,
        total: listed.total,
    }))
}

async fn get_review(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Review>, ApiError> {
    let conn = state.db.lock().expect("db lock");
    db::get_review(&conn, &id)
        .map_err(db_error)?
        .map(Json)
        .ok_or_else(ApiError::not_found)
}

async fn create_review(
    State(state): State<AppState>,
    payload: Result<Json<ReviewInput>, JsonRejection>,
) -> Result<(StatusCode, Json<Review>), ApiError> {
    let Json(input) = payload.map_err(|_| ApiError::bad("入力の形式が正しくありません"))?;
    let review = normalize(input).map_err(ApiError::bad)?;
    let conn = state.db.lock().expect("db lock");
    let created = db::create_review(&conn, review).map_err(db_error)?;
    Ok((StatusCode::CREATED, Json(created)))
}

async fn update_review(
    State(state): State<AppState>,
    Path(id): Path<String>,
    payload: Result<Json<ReviewInput>, JsonRejection>,
) -> Result<Json<Review>, ApiError> {
    let Json(input) = payload.map_err(|_| ApiError::bad("入力の形式が正しくありません"))?;
    let review = normalize(input).map_err(ApiError::bad)?;
    let conn = state.db.lock().expect("db lock");
    db::update_review(&conn, &id, review)
        .map_err(db_error)?
        .map(Json)
        .ok_or_else(ApiError::not_found)
}

async fn delete_review(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let conn = state.db.lock().expect("db lock");
    let deleted = db::delete_review(&conn, &id).map_err(db_error)?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found())
    }
}

fn db_error(err: rusqlite::Error) -> ApiError {
    tracing::error!(error = %err, "database error");
    ApiError {
        status: StatusCode::INTERNAL_SERVER_ERROR,
        message: "データベースエラーが発生しました".into(),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use http_body_util::BodyExt;
    use tower::ServiceExt;

    use super::*;

    fn test_context() -> (Router, PathBuf) {
        let path = std::env::temp_dir().join(format!("ledger-test-{}.db", uuid::Uuid::new_v4()));
        let conn = db::init_db(&path).expect("init db");
        let app = router(AppState {
            db: Arc::new(Mutex::new(conn)),
        });
        (app, path)
    }

    fn cleanup(path: &std::path::Path) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(PathBuf::from(format!("{}-wal", path.display())));
        let _ = std::fs::remove_file(PathBuf::from(format!("{}-shm", path.display())));
    }

    async fn send(app: &Router, request: Request<Body>) -> axum::response::Response {
        app.clone().oneshot(request).await.unwrap()
    }

    async fn json_body(response: axum::response::Response) -> serde_json::Value {
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        serde_json::from_slice(&bytes).unwrap()
    }

    const ID_DELIVERY: &str = "11111111-1111-4111-8111-111111111111";
    const ID_CONTRACT: &str = "22222222-2222-4222-8222-222222222222";
    const ID_PAID_LEAVE: &str = "33333333-3333-4333-8333-333333333333";
    const ID_PRESS: &str = "44444444-4444-4444-8444-444444444444";
    const ID_INCIDENT: &str = "55555555-5555-4555-8555-555555555555";
    const ID_REVIEW_COMMENT: &str = "66666666-6666-4666-8666-666666666666";

    fn percent_encode(value: &str) -> String {
        let mut encoded = String::new();
        for byte in value.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                    encoded.push(byte as char);
                }
                _ => encoded.push_str(&format!("%{byte:02X}")),
            }
        }
        encoded
    }

    fn list_uri(query: Option<&str>, decision: Option<&str>, risk: Option<&str>) -> String {
        let mut params = Vec::new();
        if let Some(query) = query {
            params.push(format!("q={}", percent_encode(query)));
        }
        if let Some(decision) = decision {
            params.push(format!("decision={}", percent_encode(decision)));
        }
        if let Some(risk) = risk {
            params.push(format!("risk={}", percent_encode(risk)));
        }
        if params.is_empty() {
            "/api/reviews".to_string()
        } else {
            format!("/api/reviews?{}", params.join("&"))
        }
    }

    fn review_body() -> serde_json::Value {
        serde_json::json!({
            "purpose": "社内案内の下書き",
            "promptSummary": "金曜の全社案内を3文で",
            "answer": "金曜は全社で開発環境の切り替えを行います。",
            "decision": "needs_revision",
            "risk": "mid",
            "hasEvidence": false,
            "memo": null
        })
    }

    fn item_ids(body: &serde_json::Value) -> Vec<String> {
        body["items"]
            .as_array()
            .unwrap()
            .iter()
            .map(|item| item["id"].as_str().unwrap().to_string())
            .collect()
    }

    async fn exchange(
        app: &Router,
        method: &str,
        uri: &str,
        body: Option<Vec<u8>>,
    ) -> (StatusCode, Option<serde_json::Value>) {
        let mut builder = Request::builder().method(method).uri(uri);
        if body.is_some() {
            builder = builder.header("content-type", "application/json");
        }
        let request = builder
            .body(match body {
                Some(bytes) => Body::from(bytes),
                None => Body::empty(),
            })
            .unwrap();
        let response = send(app, request).await;
        let status = response.status();
        if status == StatusCode::NO_CONTENT {
            return (status, None);
        }
        (status, Some(json_body(response).await))
    }

    async fn exchange_json(
        app: &Router,
        method: &str,
        uri: &str,
        body: &serde_json::Value,
    ) -> (StatusCode, serde_json::Value) {
        let bytes = serde_json::to_vec(body).unwrap();
        let (status, parsed) = exchange(app, method, uri, Some(bytes)).await;
        (status, parsed.expect("json response"))
    }

    fn assert_api_error(
        status: StatusCode,
        body: &serde_json::Value,
        expected: StatusCode,
        message: &str,
    ) {
        assert_eq!(status, expected, "{body}");
        assert_eq!(body["error"], message);
    }

    #[tokio::test]
    async fn health_and_seeded_list() {
        let (app, path) = test_context();
        let response = send(
            &app,
            Request::builder()
                .uri("/api/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);

        let response = send(
            &app,
            Request::builder()
                .uri("/api/reviews")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["total"], 6);
        assert_eq!(body["items"].as_array().unwrap().len(), 6);
        assert_eq!(
            body["items"][0]["purpose"],
            "プルリクエストのレビューコメント"
        );
        cleanup(&path);
    }

    #[tokio::test]
    async fn filters_by_decision_risk_and_query() {
        let (app, path) = test_context();
        let response = send(
            &app,
            Request::builder()
                .uri("/api/reviews?decision=adopt&risk=low")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let body = json_body(response).await;
        assert_eq!(body["items"].as_array().unwrap().len(), 2);
        assert_eq!(body["total"], 6);

        let response = send(
            &app,
            Request::builder()
                .uri("/api/reviews?q=%E6%9C%89%E7%B5%A6")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        let body = json_body(response).await;
        assert_eq!(body["items"].as_array().unwrap().len(), 1);
        assert_eq!(body["items"][0]["decision"], "adopt");

        let response = send(
            &app,
            Request::builder()
                .uri("/api/reviews?decision=maybe")
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        cleanup(&path);
    }

    #[tokio::test]
    async fn creates_updates_and_deletes() {
        let (app, path) = test_context();
        let payload = serde_json::json!({
            "purpose": " 社内案内の下書き ",
            "promptSummary": "金曜の全社案内を3文で",
            "answer": "金曜は全社で開発環境の切り替えを行います。午前の作業は回避してください。詳細は別途共有します。",
            "decision": "needs_revision",
            "risk": "mid",
            "hasEvidence": false,
            "memo": "  "
        });
        let response = send(
            &app,
            Request::builder()
                .method("POST")
                .uri("/api/reviews")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&payload).unwrap()))
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::CREATED);
        let created = json_body(response).await;
        let id = created["id"].as_str().unwrap().to_string();
        assert_eq!(created["purpose"], "社内案内の下書き");
        assert!(created["memo"].is_null());

        let mut updated = payload;
        updated["decision"] = serde_json::json!("adopt");
        updated["hasEvidence"] = serde_json::json!(true);
        updated["memo"] = serde_json::json!("告知チャネルと時刻を確認した");
        let response = send(
            &app,
            Request::builder()
                .method("PUT")
                .uri(format!("/api/reviews/{id}"))
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_vec(&updated).unwrap()))
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = json_body(response).await;
        assert_eq!(body["decision"], "adopt");
        assert_eq!(body["hasEvidence"], true);
        assert_eq!(body["memo"], "告知チャネルと時刻を確認した");

        let response = send(
            &app,
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/reviews/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NO_CONTENT);

        let response = send(
            &app,
            Request::builder()
                .uri(format!("/api/reviews/{id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        cleanup(&path);
    }

    #[tokio::test]
    async fn rejects_incomplete_input() {
        let (app, path) = test_context();
        let response = send(
            &app,
            Request::builder()
                .method("POST")
                .uri("/api/reviews")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "purpose": "",
                        "promptSummary": "要約",
                        "answer": "回答",
                        "decision": "adopt",
                        "risk": "low",
                        "hasEvidence": false
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = json_body(response).await;
        assert_eq!(body["error"], "目的を入力してください");
        cleanup(&path);
    }

    #[tokio::test]
    async fn search_matches_purpose_prompt_summary_answer_and_memo() {
        let (app, path) = test_context();
        let cases = [
            ("プレスリリース草案", ID_PRESS),
            ("エスケープ漏れ", ID_REVIEW_COMMENT),
            ("マークアップ", ID_REVIEW_COMMENT),
            ("カスタマーサポート手順書", ID_DELIVERY),
        ];

        for (query, id) in cases {
            let (status, body) =
                exchange(&app, "GET", &list_uri(Some(query), None, None), None).await;
            assert_eq!(status, StatusCode::OK, "{query}");
            let body = body.unwrap();
            assert_eq!(body["total"], 6, "{query}");
            assert_eq!(item_ids(&body), vec![id.to_string()], "{query}");
        }
        cleanup(&path);
    }

    #[tokio::test]
    async fn search_combines_with_decision_and_risk_filters() {
        let (app, path) = test_context();

        let (status, body) = exchange(
            &app,
            "GET",
            &list_uri(None, Some("needs_revision"), Some("high")),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let body = body.unwrap();
        assert_eq!(body["total"], 6);
        assert_eq!(
            item_ids(&body),
            vec![ID_CONTRACT.to_string(), ID_INCIDENT.to_string()]
        );

        let (_, body) = exchange(
            &app,
            "GET",
            &list_uri(Some("配送遅延"), Some("adopt"), Some("low")),
            None,
        )
        .await;
        let body = body.unwrap();
        assert_eq!(item_ids(&body), vec![ID_DELIVERY.to_string()]);
        assert_eq!(body["total"], 6);

        let (_, body) = exchange(
            &app,
            "GET",
            &list_uri(Some("配送遅延"), Some("reject"), Some("low")),
            None,
        )
        .await;
        let body = body.unwrap();
        assert_eq!(item_ids(&body), Vec::<String>::new());
        assert_eq!(body["total"], 6);

        let (_, body) = exchange(
            &app,
            "GET",
            &list_uri(Some("マークアップ"), Some("adopt"), Some("high")),
            None,
        )
        .await;
        assert_eq!(item_ids(&body.unwrap()), Vec::<String>::new());

        let (_, body) = exchange(
            &app,
            "GET",
            &list_uri(Some("未公表のドラフト数値"), Some("reject"), Some("mid")),
            None,
        )
        .await;
        let body = body.unwrap();
        assert_eq!(item_ids(&body), vec![ID_PRESS.to_string()]);
        assert_eq!(body["items"][0]["decision"], "reject");
        assert_eq!(body["items"][0]["risk"], "mid");

        let (_, body) = exchange(&app, "GET", &list_uri(Some("faq"), None, None), None).await;
        let body = body.unwrap();
        assert_eq!(item_ids(&body), vec![ID_PAID_LEAVE.to_string()]);
        assert_eq!(body["items"][0]["purpose"], "有給休暇の申請手順（社内FAQ）");

        cleanup(&path);
    }

    #[tokio::test]
    async fn blank_query_and_filters_do_not_narrow_the_list() {
        let (app, path) = test_context();
        let (_, all) = exchange(&app, "GET", "/api/reviews", None).await;
        let all = all.unwrap();

        let (status, blank) = exchange(
            &app,
            "GET",
            &list_uri(Some("   "), Some("  "), Some("")),
            None,
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        let blank = blank.unwrap();
        assert_eq!(blank["total"], 6);
        assert_eq!(blank["items"], all["items"]);

        let (_, padded) = exchange(&app, "GET", &list_uri(None, Some(" adopt "), None), None).await;
        assert_eq!(
            item_ids(&padded.unwrap()),
            vec![
                ID_REVIEW_COMMENT.to_string(),
                ID_DELIVERY.to_string(),
                ID_PAID_LEAVE.to_string(),
            ]
        );
        cleanup(&path);
    }

    #[tokio::test]
    async fn search_treats_like_wildcards_as_literals() {
        let (app, path) = test_context();

        let (_, percent) = exchange(&app, "GET", &list_uri(Some("%"), None, None), None).await;
        let percent = percent.unwrap();
        assert_eq!(percent["total"], 6);
        assert_eq!(
            item_ids(&percent),
            vec![ID_PRESS.to_string(), ID_INCIDENT.to_string()]
        );

        let (_, underscore) = exchange(&app, "GET", &list_uri(Some("_"), None, None), None).await;
        let underscore = underscore.unwrap();
        assert_eq!(item_ids(&underscore), Vec::<String>::new());
        assert_eq!(underscore["total"], 6);

        let (_, slash) = exchange(&app, "GET", &list_uri(Some("\\"), None, None), None).await;
        assert_eq!(item_ids(&slash.unwrap()), Vec::<String>::new());
        cleanup(&path);
    }

    #[tokio::test]
    async fn list_rejects_invalid_decision_and_risk() {
        let (app, path) = test_context();

        let (status, body) =
            exchange(&app, "GET", &list_uri(None, None, Some("critical")), None).await;
        assert_api_error(
            status,
            &body.unwrap(),
            StatusCode::BAD_REQUEST,
            "リスクの値が不正です",
        );

        let (status, body) =
            exchange(&app, "GET", &list_uri(None, Some("ADOPT"), None), None).await;
        assert_api_error(
            status,
            &body.unwrap(),
            StatusCode::BAD_REQUEST,
            "判断の値が不正です",
        );

        let (_, list) = exchange(&app, "GET", "/api/reviews", None).await;
        assert_eq!(list.unwrap()["total"], 6);
        cleanup(&path);
    }

    #[tokio::test]
    async fn list_is_ordered_by_updated_at_descending() {
        let (app, path) = test_context();
        let (status, body) = exchange(&app, "GET", "/api/reviews", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            item_ids(&body.unwrap()),
            vec![
                ID_REVIEW_COMMENT.to_string(),
                ID_CONTRACT.to_string(),
                ID_DELIVERY.to_string(),
                ID_PRESS.to_string(),
                ID_PAID_LEAVE.to_string(),
                ID_INCIDENT.to_string(),
            ]
        );
        cleanup(&path);
    }

    #[tokio::test]
    async fn create_rejects_missing_fields_and_malformed_json() {
        let (app, path) = test_context();

        let (status, body) = exchange(&app, "POST", "/api/reviews", Some(b"{".to_vec())).await;
        assert_api_error(
            status,
            &body.unwrap(),
            StatusCode::BAD_REQUEST,
            "入力の形式が正しくありません",
        );

        let (status, body) = exchange(&app, "POST", "/api/reviews", Some(Vec::new())).await;
        assert_api_error(
            status,
            &body.unwrap(),
            StatusCode::BAD_REQUEST,
            "入力の形式が正しくありません",
        );

        for field in [
            "purpose",
            "promptSummary",
            "answer",
            "decision",
            "risk",
            "hasEvidence",
        ] {
            let mut payload = review_body();
            payload.as_object_mut().unwrap().remove(field);
            let (status, body) = exchange_json(&app, "POST", "/api/reviews", &payload).await;
            assert_api_error(
                status,
                &body,
                StatusCode::BAD_REQUEST,
                "入力の形式が正しくありません",
            );
        }

        let mut payload = review_body();
        payload["hasEvidence"] = serde_json::json!("yes");
        let (status, body) = exchange_json(&app, "POST", "/api/reviews", &payload).await;
        assert_api_error(
            status,
            &body,
            StatusCode::BAD_REQUEST,
            "入力の形式が正しくありません",
        );

        let (_, list) = exchange(&app, "GET", "/api/reviews", None).await;
        assert_eq!(list.unwrap()["total"], 6);
        cleanup(&path);
    }

    #[tokio::test]
    async fn create_rejects_blank_text_and_invalid_decision_or_risk() {
        let (app, path) = test_context();
        let cases = [
            (
                "promptSummary",
                serde_json::json!("   "),
                "プロンプト要約を入力してください",
            ),
            ("answer", serde_json::json!(""), "AI回答を入力してください"),
            ("decision", serde_json::json!("maybe"), "判断の値が不正です"),
            ("decision", serde_json::json!(""), "判断の値が不正です"),
            (
                "risk",
                serde_json::json!("critical"),
                "リスクの値が不正です",
            ),
            ("risk", serde_json::json!(" "), "リスクの値が不正です"),
        ];

        for (field, value, message) in cases {
            let mut payload = review_body();
            payload[field] = value;
            let (status, body) = exchange_json(&app, "POST", "/api/reviews", &payload).await;
            assert_api_error(status, &body, StatusCode::BAD_REQUEST, message);
        }

        let (_, list) = exchange(&app, "GET", "/api/reviews", None).await;
        assert_eq!(list.unwrap()["total"], 6);
        cleanup(&path);
    }

    #[tokio::test]
    async fn create_enforces_text_length_limits() {
        let (app, path) = test_context();

        let mut payload = review_body();
        payload["purpose"] = serde_json::json!("あ".repeat(200));
        let (status, created) = exchange_json(&app, "POST", "/api/reviews", &payload).await;
        assert_eq!(status, StatusCode::CREATED);
        assert_eq!(created["purpose"].as_str().unwrap().chars().count(), 200);
        let id = created["id"].as_str().unwrap().to_string();

        let mut payload = review_body();
        payload["purpose"] = serde_json::json!("あ".repeat(201));
        let (status, body) = exchange_json(&app, "POST", "/api/reviews", &payload).await;
        assert_api_error(
            status,
            &body,
            StatusCode::BAD_REQUEST,
            "目的は200文字以内で入力してください",
        );

        let mut payload = review_body();
        payload["promptSummary"] = serde_json::json!("p".repeat(4_001));
        let (status, body) = exchange_json(&app, "POST", "/api/reviews", &payload).await;
        assert_api_error(
            status,
            &body,
            StatusCode::BAD_REQUEST,
            "プロンプト要約は4000文字以内で入力してください",
        );

        let mut payload = review_body();
        payload["answer"] = serde_json::json!("a".repeat(20_001));
        let (status, body) = exchange_json(&app, "POST", "/api/reviews", &payload).await;
        assert_api_error(
            status,
            &body,
            StatusCode::BAD_REQUEST,
            "AI回答は20000文字以内で入力してください",
        );

        let mut payload = review_body();
        payload["memo"] = serde_json::json!("m".repeat(4_001));
        let (status, body) = exchange_json(&app, "POST", "/api/reviews", &payload).await;
        assert_api_error(
            status,
            &body,
            StatusCode::BAD_REQUEST,
            "メモは4000文字以内で入力してください",
        );

        let mut payload = review_body();
        payload["purpose"] = serde_json::json!("あ".repeat(200));
        payload["promptSummary"] = serde_json::json!("p".repeat(4_000));
        payload["answer"] = serde_json::json!("a".repeat(20_000));
        payload["memo"] = serde_json::json!("m".repeat(4_000));
        let (status, updated) =
            exchange_json(&app, "PUT", &format!("/api/reviews/{id}"), &payload).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            updated["promptSummary"].as_str().unwrap().chars().count(),
            4_000
        );
        assert_eq!(updated["answer"].as_str().unwrap().chars().count(), 20_000);
        assert_eq!(updated["memo"].as_str().unwrap().chars().count(), 4_000);

        let (status, fetched) = exchange(&app, "GET", &format!("/api/reviews/{id}"), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(fetched.unwrap(), updated);

        let (_, list) = exchange(&app, "GET", "/api/reviews", None).await;
        assert_eq!(list.unwrap()["total"], 7);
        cleanup(&path);
    }

    #[tokio::test]
    async fn create_then_get_returns_the_saved_review() {
        let (app, path) = test_context();
        let payload = serde_json::json!({
            "purpose": " 境界の確認 ",
            "promptSummary": " 要約テキスト ",
            "answer": " 回答テキスト ",
            "decision": " reject ",
            "risk": " high ",
            "hasEvidence": false,
            "memo": " 根拠メモ "
        });

        let (status, created) = exchange_json(&app, "POST", "/api/reviews", &payload).await;
        assert_eq!(status, StatusCode::CREATED);
        let id = created["id"].as_str().unwrap().to_string();
        let parsed = uuid::Uuid::parse_str(&id).unwrap();
        assert_eq!(parsed.get_version(), Some(uuid::Version::Random));
        assert_eq!(created["purpose"], "境界の確認");
        assert_eq!(created["promptSummary"], "要約テキスト");
        assert_eq!(created["answer"], "回答テキスト");
        assert_eq!(created["decision"], "reject");
        assert_eq!(created["risk"], "high");
        assert_eq!(created["hasEvidence"], false);
        assert_eq!(created["memo"], "根拠メモ");
        assert_eq!(created["createdAt"], created["updatedAt"]);
        assert!(created["createdAt"].as_str().unwrap().ends_with('Z'));

        let (status, fetched) = exchange(&app, "GET", &format!("/api/reviews/{id}"), None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(fetched.unwrap(), created);

        let (_, list) = exchange(&app, "GET", "/api/reviews", None).await;
        let list = list.unwrap();
        assert_eq!(list["total"], 7);
        assert!(item_ids(&list).contains(&id));
        cleanup(&path);
    }

    #[tokio::test]
    async fn blank_memo_is_stored_as_null() {
        let (app, path) = test_context();
        let mut seen = Vec::new();

        let mut omitted = review_body();
        omitted.as_object_mut().unwrap().remove("memo");
        let (status, created) = exchange_json(&app, "POST", "/api/reviews", &omitted).await;
        assert_eq!(status, StatusCode::CREATED);
        assert!(created["memo"].is_null());
        let omitted_id = created["id"].as_str().unwrap().to_string();
        let (_, fetched) = exchange(&app, "GET", &format!("/api/reviews/{omitted_id}"), None).await;
        assert_eq!(fetched.unwrap(), created);
        seen.push(omitted_id);

        for memo in [
            serde_json::json!(null),
            serde_json::json!(""),
            serde_json::json!(" \n\t "),
        ] {
            let mut payload = review_body();
            payload["memo"] = memo;
            let (status, created) = exchange_json(&app, "POST", "/api/reviews", &payload).await;
            assert_eq!(status, StatusCode::CREATED);
            assert!(created["memo"].is_null());
            let id = created["id"].as_str().unwrap().to_string();
            assert!(!seen.contains(&id));
            seen.push(id.clone());

            let (_, fetched) = exchange(&app, "GET", &format!("/api/reviews/{id}"), None).await;
            assert!(fetched.unwrap()["memo"].is_null());

            payload["memo"] = serde_json::json!("残すメモ");
            let (status, updated) =
                exchange_json(&app, "PUT", &format!("/api/reviews/{id}"), &payload).await;
            assert_eq!(status, StatusCode::OK);
            assert_eq!(updated["memo"], "残すメモ");
            assert_eq!(updated["createdAt"], created["createdAt"]);

            payload["memo"] = serde_json::json!("   ");
            let (status, cleared) =
                exchange_json(&app, "PUT", &format!("/api/reviews/{id}"), &payload).await;
            assert_eq!(status, StatusCode::OK);
            assert!(cleared["memo"].is_null());

            let (_, fetched) = exchange(&app, "GET", &format!("/api/reviews/{id}"), None).await;
            assert_eq!(fetched.unwrap(), cleared);
        }

        cleanup(&path);
    }

    #[tokio::test]
    async fn update_unknown_review_returns_not_found() {
        let (app, path) = test_context();
        let (status, body) = exchange_json(
            &app,
            "PUT",
            "/api/reviews/00000000-0000-4000-8000-000000000000",
            &review_body(),
        )
        .await;
        assert_api_error(
            status,
            &body,
            StatusCode::NOT_FOUND,
            "レビューが見つかりません",
        );

        let (_, list) = exchange(&app, "GET", "/api/reviews", None).await;
        assert_eq!(list.unwrap()["total"], 6);
        cleanup(&path);
    }

    #[tokio::test]
    async fn invalid_update_leaves_the_review_unchanged() {
        let (app, path) = test_context();
        let uri = format!("/api/reviews/{ID_REVIEW_COMMENT}");
        let (_, before) = exchange(&app, "GET", &uri, None).await;
        let before = before.unwrap();

        let mut payload = review_body();
        payload["risk"] = serde_json::json!("severe");
        let (status, body) = exchange_json(&app, "PUT", &uri, &payload).await;
        assert_api_error(
            status,
            &body,
            StatusCode::BAD_REQUEST,
            "リスクの値が不正です",
        );

        let (status, after) = exchange(&app, "GET", &uri, None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(after.unwrap(), before);
        cleanup(&path);
    }

    #[tokio::test]
    async fn get_and_delete_unknown_review_return_not_found() {
        let (app, path) = test_context();
        let uri = "/api/reviews/00000000-0000-4000-8000-000000000099";

        let (status, body) = exchange(&app, "GET", uri, None).await;
        assert_api_error(
            status,
            &body.unwrap(),
            StatusCode::NOT_FOUND,
            "レビューが見つかりません",
        );

        let (status, body) = exchange(&app, "DELETE", uri, None).await;
        assert_api_error(
            status,
            &body.unwrap(),
            StatusCode::NOT_FOUND,
            "レビューが見つかりません",
        );

        let (_, list) = exchange(&app, "GET", "/api/reviews", None).await;
        assert_eq!(list.unwrap()["total"], 6);
        cleanup(&path);
    }
}
