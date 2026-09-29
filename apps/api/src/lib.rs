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

pub use db::default_db_path;

pub fn app(db_path: &std::path::Path) -> Result<Router, Box<dyn std::error::Error>> {
    let conn = db::init_db(db_path)?;
    Ok(router(AppState {
        db: Arc::new(Mutex::new(conn)),
    }))
}

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
