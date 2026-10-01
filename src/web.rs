use crate::parser::{parse_scan_target, ScanTarget};
use crate::scanner::{scan_github_org, scan_github_repo, ScanResult, ScannerOptions};
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
    pub repo: Option<String>,
    pub target: Option<String>,
    pub targets: Option<Vec<String>>,
    pub branch: Option<String>,
    pub no_cache: Option<bool>,
    pub refresh: Option<bool>,
    pub token: Option<String>,
    pub filter: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct StarPayload {
    pub url: String,
    pub starred: Option<bool>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StarResponse {
    pub url: String,
    pub starred: bool,
    pub success: bool,
}

#[derive(Debug, Deserialize)]
pub struct SkillsQuery {
    pub q: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsResponse {
    pub skills: Vec<crate::scanner::Skill>,
    pub total: usize,
}

#[derive(Debug, Deserialize)]
pub struct SimilarPayload {
    pub repo: Option<String>,
    pub branch: Option<String>,
    pub no_cache: Option<bool>,
    pub refresh: Option<bool>,
    pub token: Option<String>,
    pub target: Option<String>,
    pub name: Option<String>,
    pub path: Option<String>,
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
        .route("/api/skills", get(skills_handler))
        .route("/api/starred", get(starred_skills_handler))
        .route("/api/star", post(star_skill_handler))
        .route("/api/skills/star", post(star_skill_handler))
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

async fn skills_handler(
    State(state): State<Arc<AppState>>,
    Query(query): Query<SkillsQuery>,
) -> Result<Json<SkillsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let db_path = state.default_options.db_path.as_deref();
    let conn = open_db(db_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to open database: {}", e),
            }),
        )
    })?;

    let mut skills = crate::storage::get_all_cached_skills(&conn).unwrap_or_default();
    let total = skills.len();
    if let Some(q) = query.q {
        if !q.trim().is_empty() {
            skills = crate::similarity::filter_skills(&skills, &q);
        }
    }

    Ok(Json(SkillsResponse { skills, total }))
}

async fn starred_skills_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SkillsResponse>, (StatusCode, Json<ErrorResponse>)> {
    let db_path = state.default_options.db_path.as_deref();
    let conn = open_db(db_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to open database: {}", e),
            }),
        )
    })?;

    let skills = crate::storage::get_starred_skills(&conn).unwrap_or_default();
    let total = skills.len();
    Ok(Json(SkillsResponse { skills, total }))
}

async fn star_skill_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<StarPayload>,
) -> Result<Json<StarResponse>, (StatusCode, Json<ErrorResponse>)> {
    let db_path = state.default_options.db_path.as_deref();
    let conn = open_db(db_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to open database: {}", e),
            }),
        )
    })?;

    let dummy_skill = crate::scanner::Skill {
        name: payload.name.unwrap_or_else(|| "skill".to_string()),
        description: payload.description.unwrap_or_default(),
        path: payload.path.unwrap_or_default(),
        url: payload.url.clone(),
        starred: false,
    };

    let new_status = if let Some(st) = payload.starred {
        crate::storage::set_skill_starred(&conn, &dummy_skill, st).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to update star: {}", e),
                }),
            )
        })?
    } else {
        crate::storage::toggle_skill_starred(&conn, &dummy_skill).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("Failed to toggle star: {}", e),
                }),
            )
        })?
    };

    Ok(Json(StarResponse {
        url: payload.url,
        starred: new_status,
        success: true,
    }))
}

async fn execute_scan(
    payload: ScanPayload,
    state: &AppState,
) -> Result<ScanResult, (StatusCode, String)> {
    let raw_targets: Vec<String> = if let Some(targets) = payload.targets {
        targets
            .into_iter()
            .flat_map(|t| {
                t.split(|c: char| c == ',' || c == ';' || c.is_whitespace())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
            .collect()
    } else if let Some(target) = payload.target {
        target
            .split(|c: char| c == ',' || c == ';' || c.is_whitespace())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else if let Some(repo) = payload.repo {
        repo.split(|c: char| c == ',' || c == ';' || c.is_whitespace())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    };

    if raw_targets.is_empty() {
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

    if raw_targets.len() == 1 {
        let target_str = &raw_targets[0];
        let parsed = parse_scan_target(target_str);

        let mut scan_result = match parsed {
            Ok(ScanTarget::Org { org }) => {
                let org_results = scan_github_org(&org, &scan_opts).await.map_err(|err| {
                    let status = match err {
                        crate::scanner::ScannerError::Parse(_) => StatusCode::BAD_REQUEST,
                        crate::scanner::ScannerError::NotFound(_, _) => StatusCode::NOT_FOUND,
                        crate::scanner::ScannerError::Forbidden(_, _, _) => StatusCode::FORBIDDEN,
                        _ => StatusCode::INTERNAL_SERVER_ERROR,
                    };
                    (status, err.to_string())
                })?;

                let mut all_skills = Vec::new();
                let mut any_from_cache = true;
                let mut any_truncated = false;
                for res in org_results {
                    if !res.from_cache {
                        any_from_cache = false;
                    }
                    if res.truncated {
                        any_truncated = true;
                    }
                    all_skills.extend(res.skills);
                }

                ScanResult {
                    owner: org,
                    repo: "all-repositories".to_string(),
                    branch: scan_opts
                        .branch
                        .clone()
                        .unwrap_or_else(|| "HEAD".to_string()),
                    skills: all_skills,
                    from_cache: any_from_cache,
                    commit_sha: None,
                    truncated: any_truncated,
                }
            }
            _ => scan_github_repo(target_str, &scan_opts)
                .await
                .map_err(|err| {
                    let status = match err {
                        crate::scanner::ScannerError::Parse(_) => StatusCode::BAD_REQUEST,
                        crate::scanner::ScannerError::NotFound(_, _) => StatusCode::NOT_FOUND,
                        crate::scanner::ScannerError::Forbidden(_, _, _) => StatusCode::FORBIDDEN,
                        _ => StatusCode::INTERNAL_SERVER_ERROR,
                    };
                    (status, err.to_string())
                })?,
        };

        if let Some(f) = payload.filter {
            if !f.trim().is_empty() {
                scan_result.skills = crate::similarity::filter_skills(&scan_result.skills, &f);
            }
        }

        Ok(scan_result)
    } else {
        let mut all_skills = Vec::new();
        let mut last_owner = String::new();
        let mut last_repo = String::new();
        let mut last_branch = "HEAD".to_string();
        let mut any_from_cache = true;
        let mut any_truncated = false;

        for target_str in &raw_targets {
            let parsed = parse_scan_target(target_str);
            match parsed {
                Ok(ScanTarget::Org { org }) => {
                    if let Ok(org_results) = scan_github_org(&org, &scan_opts).await {
                        for mut r in org_results {
                            last_owner = r.owner;
                            last_repo = r.repo;
                            last_branch = r.branch;
                            if !r.from_cache {
                                any_from_cache = false;
                            }
                            if r.truncated {
                                any_truncated = true;
                            }
                            all_skills.append(&mut r.skills);
                        }
                    }
                }
                _ => match scan_github_repo(target_str, &scan_opts).await {
                    Ok(mut res) => {
                        last_owner = res.owner;
                        last_repo = res.repo;
                        last_branch = res.branch;
                        if !res.from_cache {
                            any_from_cache = false;
                        }
                        if res.truncated {
                            any_truncated = true;
                        }
                        all_skills.append(&mut res.skills);
                    }
                    Err(err) => {
                        if raw_targets.len() == 1 {
                            let status = match err {
                                crate::scanner::ScannerError::Parse(_) => StatusCode::BAD_REQUEST,
                                crate::scanner::ScannerError::NotFound(_, _) => {
                                    StatusCode::NOT_FOUND
                                }
                                crate::scanner::ScannerError::Forbidden(_, _, _) => {
                                    StatusCode::FORBIDDEN
                                }
                                _ => StatusCode::INTERNAL_SERVER_ERROR,
                            };
                            return Err((status, err.to_string()));
                        }
                    }
                },
            }
        }

        if let Some(f) = payload.filter {
            if !f.trim().is_empty() {
                all_skills = crate::similarity::filter_skills(&all_skills, &f);
            }
        }

        Ok(ScanResult {
            owner: if raw_targets.len() == 1 {
                last_owner
            } else {
                "multiple".to_string()
            },
            repo: if raw_targets.len() == 1 {
                last_repo
            } else {
                "repositories".to_string()
            },
            branch: last_branch,
            commit_sha: None,
            skills: all_skills,
            from_cache: any_from_cache,
            truncated: any_truncated,
        })
    }
}

async fn execute_similar(
    payload: SimilarPayload,
    state: &AppState,
) -> Result<SimilarResponse, (StatusCode, String)> {
    let repo_opt = payload.repo.as_deref().filter(|r| !r.trim().is_empty());

    let (skills, repo_label, branch_label) = if let Some(repo) = repo_opt {
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

        (
            scan_result.skills,
            format!("{}/{}", scan_result.owner, scan_result.repo),
            scan_result.branch,
        )
    } else {
        let db_path = state.default_options.db_path.as_deref();
        let conn = open_db(db_path).map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to open DB: {}", e),
            )
        })?;
        let all_skills = crate::storage::get_all_cached_skills(&conn).unwrap_or_default();
        (all_skills, "all-cached".to_string(), "HEAD".to_string())
    };

    let threshold = payload
        .min_similarity
        .unwrap_or(DEFAULT_SIMILARITY_THRESHOLD);
    let pairs = find_all_similar_pairs(&skills, threshold);

    let target_query = payload
        .target
        .or(payload.name)
        .or(payload.path)
        .filter(|t| !t.trim().is_empty());

    let target_matches = if let Some(query) = target_query {
        let lower = query.trim().to_lowercase();
        let target_skill = skills
            .iter()
            .find(|s| s.name.to_lowercase() == lower)
            .or_else(|| skills.iter().find(|s| s.path.to_lowercase() == lower))
            .or_else(|| {
                skills
                    .iter()
                    .find(|s| s.name.to_lowercase().contains(&lower))
            });

        if let Some(target) = target_skill {
            find_similar_skills(target, &skills, threshold)
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    Ok(SimilarResponse {
        repo: repo_label,
        branch: branch_label,
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
