use axum::{
    body::Body,
    http::{header, HeaderValue, Request},
    middleware::{self, Next},
    response::Response,
    routing::delete,
    routing::get,
    routing::post,
    Router,
};
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::predict::{GroundStation, TleLoader};
use crate::scheduler::Storage;
use crate::tracker::Tracker;

use super::api::predict;
use super::api::schedules;
use super::api::tracker;
use super::api_doc::ApiDoc;
use super::auth::AppState;
use super::config::Config;
use super::ui::handlers as ui_handlers;

/// Middleware to add Cache-Control: no-cache header to responses
async fn add_cache_control(req: Request<Body>, next: Next) -> Response {
    let mut response = next.run(req).await;
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    response
}

pub async fn run_server(config: Config) -> std::io::Result<()> {
    let bind_addr = config.web.bind.clone();
    let storage = Storage::new(config.schedules.base.clone());
    let station = GroundStation::from_coordinates(
        &config.station.coordinates,
        Some(config.station.altitude_m),
    )
    .unwrap_or_default();
    let tracker = Tracker::new(station);

    // Initialize TLE loader if predict config is present
    let tle_loader = if let Some(ref predict_config) = config.predict {
        let mut loader = TleLoader::new(predict_config.tle_folder.clone());
        if let Err(e) = loader.load_all() {
            log::warn!("Failed to initialize TLE loader: {}", e);
        }
        Some(Arc::new(RwLock::new(loader)))
    } else {
        None
    };

    let state = AppState {
        config: Arc::new(config),
        storage: Arc::new(storage),
        tracker: Arc::new(Mutex::new(tracker)),
        tle_loader,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // API routes with cache-control middleware
    let api_routes = Router::new()
        // Schedule API endpoints
        .route("/schedules", post(schedules::submit_schedule))
        .route("/schedules", get(schedules::list_schedules))
        .route("/schedules/templates", get(schedules::list_templates))
        .route("/schedules/template/{name}", get(schedules::get_template))
        .route("/schedules/{id}", get(schedules::get_schedule))
        .route("/schedules/{id}", delete(schedules::delete_schedule))
        .route("/schedules/{id}/approve", post(schedules::approve_schedule))
        .route("/schedules/{id}/reject", post(schedules::reject_schedule))
        .route("/schedules/validate", post(schedules::validate_schedule))
        // Tracker API endpoints
        .route("/tracker/run", post(tracker::run))
        .route("/tracker/stop", post(tracker::stop))
        .route("/tracker/status/mode", get(tracker::status_mode))
        .route("/tracker/status/sample", get(tracker::status_sample))
        .route(
            "/tracker/status/trajectory",
            get(tracker::status_trajectory),
        )
        // Predict API endpoints
        .route("/predict", get(predict::list_predictions))
        // Add Cache-Control: no-cache to all API responses
        .layer(middleware::from_fn(add_cache_control));

    let app = Router::new()
        // UI routes
        .route("/", get(ui_handlers::dashboard))
        .route("/timeline", get(ui_handlers::timeline))
        // API routes with cache control
        .nest("/api", api_routes)
        // Static files
        .nest_service("/static", ServeDir::new("src/web/static"))
        // OpenAPI / Swagger
        .merge(SwaggerUi::new("/swagger-ui").url("/api-doc/openapi.json", ApiDoc::openapi()))
        // Middleware
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    log::info!("Starting server on {}", bind_addr);

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    axum::serve(listener, app).await
}
