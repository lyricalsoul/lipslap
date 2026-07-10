use crate::template::Registry;
use crate::types::GenerateData;
use serde::Serialize;
use serde_json::json;
use std::convert::Infallible;
use std::sync::Arc;
use tokio_cron_scheduler::{Job, JobScheduler};
use warp::Filter;

pub struct ServeOptions {
    pub port: u16,
    pub server_url: String,
    pub instance_name: String,
    pub api_key: Option<String>,
}

impl ServeOptions {
    /// Reads PORT, SERVER_URL, INSTANCE_NAME and API_KEY from the environment,
    /// same knobs the Node version read via process.env.
    pub fn from_env() -> Self {
        let port = std::env::var("PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(4444);
        Self {
            port,
            server_url: std::env::var("SERVER_URL").unwrap_or_else(|_| format!("http://localhost:{port}")),
            instance_name: std::env::var("INSTANCE_NAME").unwrap_or_else(|_| "Kepler".into()),
            api_key: std::env::var("API_KEY").ok(),
        }
    }
}

#[derive(Serialize)]
struct GenerateResponse {
    error: bool,
    file: String,
    url: String,
    time: u128,
}

#[derive(Serialize)]
struct ErrorResponse {
    error: bool,
    message: String,
}

/// Boots the ditto HTTP server: /status, /metadata, /generate, and /results
/// static file serving, plus a daily cache-cleanup cron job. Call once with
/// the themes your consumer registered.
pub async fn serve(registry: Registry, options: ServeOptions) -> anyhow::Result<()> {
    crate::imaging::create_directories().await?;
    crate::fonts::load_fonts();

    let registry = Arc::new(registry);
    let http_client = reqwest::Client::new();

    let scheduler = JobScheduler::new().await?;
    scheduler
        .add(Job::new_async("0 0 3 * * *", |_uuid, _l| {
            Box::pin(async {
                if let Err(e) = crate::imaging::clear_cache().await {
                    eprintln!("clear_cache failed: {e}");
                }
            })
        })?)
        .await?;
    scheduler.start().await?;

    let status = warp::path("status").map(|| warp::reply::json(&json!({ "status": "ok" })));

    let metadata_registry = registry.clone();
    let instance_name = options.instance_name.clone();
    let metadata = warp::path("metadata").map(move || {
        warp::reply::json(&json!({
            "name": instance_name,
            "engine": "ditto",
            "scheme": 1.0,
            "themes": metadata_registry.theme_names(),
        }))
    });

    #[cfg(debug_assertions)]
    let preview_routes = crate::preview::routes(registry.clone(), http_client.clone());

    let base_url = options.server_url.clone();
    let api_key = options.api_key.clone();
    let generate = warp::path("generate")
        .and(warp::post())
        .and(warp::header::optional::<String>("x-api-key"))
        .and(warp::body::json())
        .and_then(move |provided_key: Option<String>, data: GenerateData| {
            let base_url = base_url.clone();
            let http_client = http_client.clone();
            let registry = registry.clone();
            let api_key = api_key.clone();
            async move {
                if let Some(expected) = &api_key
                    && provided_key.as_deref() != Some(expected.as_str())
                {
                    return Ok::<_, Infallible>(warp::reply::with_status(
                        warp::reply::json(&ErrorResponse {
                            error: true,
                            message: "Unauthorized".into(),
                        }),
                        warp::http::StatusCode::UNAUTHORIZED,
                    ));
                }

                match crate::generator::generate(&registry, &http_client, data).await {
                    Ok(outcome) => {
                        let file = format!("{}.{}", outcome.id, outcome.extension);
                        Ok(warp::reply::with_status(
                            warp::reply::json(&GenerateResponse {
                                error: false,
                                url: format!("{base_url}/results/{file}"),
                                file,
                                time: outcome.time_ms,
                            }),
                            warp::http::StatusCode::OK,
                        ))
                    }
                    Err(e) => Ok(warp::reply::with_status(
                        warp::reply::json(&ErrorResponse {
                            error: true,
                            message: e.to_string(),
                        }),
                        warp::http::StatusCode::BAD_REQUEST,
                    )),
                }
            }
        });

    let results = warp::path("results").and(warp::fs::dir(crate::imaging::generation_cache_dir()));

    let routes = status.or(metadata).or(generate).or(results);

    #[cfg(debug_assertions)]
    let routes = routes.or(preview_routes);

    println!("listening on port {}", options.port);
    warp::serve(routes).run(([0, 0, 0, 0], options.port)).await;

    Ok(())
}
