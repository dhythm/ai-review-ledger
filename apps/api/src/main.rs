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
}
