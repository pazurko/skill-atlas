use crate::scanner::{scan_github_repo, ScanResult, ScannerOptions};
use crate::similarity::{
    find_all_similar_pairs, find_similar_skills, SimilarPair, SimilarSkillMatch,
    DEFAULT_SIMILARITY_THRESHOLD,
};
use crate::storage::{list_all_cached_repositories, open_db, CachedRepo};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, Json},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::net::TcpListener;

#[derive(Debug, Clone)]
pub struct WebOptions {
    pub host: String,
    pub port: u16,
    pub open_browser: bool,
    pub scanner_options: ScannerOptions,
}

impl Default for WebOptions {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3000,
            open_browser: true,
            scanner_options: ScannerOptions::default(),
        }
    }
}

pub struct AppState {
    pub default_options: ScannerOptions,
}

#[derive(Debug, Deserialize)]
pub struct ScanPayload {
    pub repo: String,
    pub branch: Option<String>,
    pub no_cache: Option<bool>,
    pub refresh: Option<bool>,
    pub token: Option<String>,
    pub filter: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SimilarPayload {
    pub repo: String,
    pub branch: Option<String>,
    pub no_cache: Option<bool>,
    pub refresh: Option<bool>,
    pub token: Option<String>,
    pub target: Option<String>,
    pub min_similarity: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarResponse {
    pub repo: String,
    pub branch: String,
    pub threshold: f64,
    pub pairs: Vec<SimilarPair>,
    pub target_matches: Vec<SimilarSkillMatch>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
}

pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/api/health", get(health_handler))
        .route("/api/cached", get(cached_repos_handler))
        .route("/api/scan", post(scan_post_handler).get(scan_get_handler))
        .route(
            "/api/similar",
            post(similar_post_handler).get(similar_get_handler),
        )
        .with_state(state)
}

pub async fn start_web_server(options: WebOptions) -> Result<(), Box<dyn std::error::Error>> {
    let host = &options.host;
    let port = options.port;
    let addr_str = format!("{}:{}", host, port);
    let listener = TcpListener::bind(&addr_str).await?;
    let local_addr = listener.local_addr()?;
    let url = format!("http://{}", local_addr);

    println!("⚡ Skill Atlas Web Server started at {}", url);
    println!("📖 Open {} in your browser to start scanning.", url);
    println!("🛑 Press Ctrl+C to stop the server.\n");

    if options.open_browser {
        let open_url = url.clone();
        tokio::spawn(async move {
            tokio::time::sleep(tokio::time::Duration::from_millis(150)).await;
            let _ = open::that(&open_url);
        });
    }

    let state = Arc::new(AppState {
        default_options: options.scanner_options,
    });

    let app = create_router(state);
    axum::serve(listener, app).await?;
    Ok(())
}

async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

async fn cached_repos_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<CachedRepo>>, (StatusCode, Json<ErrorResponse>)> {
    let db_path = state.default_options.db_path.as_deref();
    let conn = open_db(db_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to open database: {}", e),
            }),
        )
    })?;

    let repos = list_all_cached_repositories(&conn).unwrap_or_default();
    Ok(Json(repos))
}

async fn execute_scan(
    payload: ScanPayload,
    state: &AppState,
) -> Result<ScanResult, (StatusCode, String)> {
    let repo = payload.repo.trim();
    if repo.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Repository identifier cannot be empty".to_string(),
        ));
    }

    let no_cache = payload
        .no_cache
        .or(payload.refresh)
        .unwrap_or(state.default_options.no_cache);

    let branch = payload
        .branch
        .filter(|b| !b.trim().is_empty())
        .or_else(|| state.default_options.branch.clone());

    let token = payload
        .token
        .filter(|t| !t.trim().is_empty())
        .or_else(|| state.default_options.token.clone());

    let scan_opts = ScannerOptions {
        token,
        branch,
        base_api_url: state.default_options.base_api_url.clone(),
        base_raw_url: state.default_options.base_raw_url.clone(),
        no_cache,
        db_path: state.default_options.db_path.clone(),
    };

    let mut scan_result = scan_github_repo(repo, &scan_opts).await.map_err(|err| {
        let status = match err {
            crate::scanner::ScannerError::Parse(_) => StatusCode::BAD_REQUEST,
            crate::scanner::ScannerError::NotFound(_, _) => StatusCode::NOT_FOUND,
            crate::scanner::ScannerError::Forbidden(_, _, _) => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, err.to_string())
    })?;

    if let Some(f) = payload.filter {
        if !f.trim().is_empty() {
            scan_result.skills = crate::similarity::filter_skills(&scan_result.skills, &f);
        }
    }

    Ok(scan_result)
}

async fn execute_similar(
    payload: SimilarPayload,
    state: &AppState,
) -> Result<SimilarResponse, (StatusCode, String)> {
    let repo = payload.repo.trim();
    if repo.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Repository identifier cannot be empty".to_string(),
        ));
    }

    let no_cache = payload
        .no_cache
        .or(payload.refresh)
        .unwrap_or(state.default_options.no_cache);

    let branch = payload
        .branch
        .filter(|b| !b.trim().is_empty())
        .or_else(|| state.default_options.branch.clone());

    let token = payload
        .token
        .filter(|t| !t.trim().is_empty())
        .or_else(|| state.default_options.token.clone());

    let scan_opts = ScannerOptions {
        token,
        branch,
        base_api_url: state.default_options.base_api_url.clone(),
        base_raw_url: state.default_options.base_raw_url.clone(),
        no_cache,
        db_path: state.default_options.db_path.clone(),
    };

    let scan_result = scan_github_repo(repo, &scan_opts).await.map_err(|err| {
        let status = match err {
            crate::scanner::ScannerError::Parse(_) => StatusCode::BAD_REQUEST,
            crate::scanner::ScannerError::NotFound(_, _) => StatusCode::NOT_FOUND,
            crate::scanner::ScannerError::Forbidden(_, _, _) => StatusCode::FORBIDDEN,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, err.to_string())
    })?;

    let threshold = payload
        .min_similarity
        .unwrap_or(DEFAULT_SIMILARITY_THRESHOLD);
    let pairs = find_all_similar_pairs(&scan_result.skills, threshold);

    let target_matches = if let Some(target_query) = payload.target.filter(|t| !t.trim().is_empty())
    {
        let lower = target_query.trim().to_lowercase();
        let target_skill = scan_result
            .skills
            .iter()
            .find(|s| s.name.to_lowercase() == lower)
            .or_else(|| {
                scan_result
                    .skills
                    .iter()
                    .find(|s| s.name.to_lowercase().contains(&lower))
            });

        if let Some(target) = target_skill {
            find_similar_skills(target, &scan_result.skills, threshold)
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    Ok(SimilarResponse {
        repo: format!("{}/{}", scan_result.owner, scan_result.repo),
        branch: scan_result.branch,
        threshold,
        pairs,
        target_matches,
    })
}

async fn scan_post_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ScanPayload>,
) -> Result<Json<ScanResult>, (StatusCode, Json<ErrorResponse>)> {
    match execute_scan(payload, &state).await {
        Ok(result) => Ok(Json(result)),
        Err((status, error)) => Err((status, Json(ErrorResponse { error }))),
    }
}

async fn scan_get_handler(
    State(state): State<Arc<AppState>>,
    Query(payload): Query<ScanPayload>,
) -> Result<Json<ScanResult>, (StatusCode, Json<ErrorResponse>)> {
    match execute_scan(payload, &state).await {
        Ok(result) => Ok(Json(result)),
        Err((status, error)) => Err((status, Json(ErrorResponse { error }))),
    }
}

async fn similar_post_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<SimilarPayload>,
) -> Result<Json<SimilarResponse>, (StatusCode, Json<ErrorResponse>)> {
    match execute_similar(payload, &state).await {
        Ok(result) => Ok(Json(result)),
        Err((status, error)) => Err((status, Json(ErrorResponse { error }))),
    }
}

async fn similar_get_handler(
    State(state): State<Arc<AppState>>,
    Query(payload): Query<SimilarPayload>,
) -> Result<Json<SimilarResponse>, (StatusCode, Json<ErrorResponse>)> {
    match execute_similar(payload, &state).await {
        Ok(result) => Ok(Json(result)),
        Err((status, error)) => Err((status, Json(ErrorResponse { error }))),
    }
}

async fn index_handler() -> Html<&'static str> {
    Html(INDEX_HTML)
}

pub const INDEX_HTML: &str = include_str!("index.html");
