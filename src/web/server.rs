use axum::{routing::delete, routing::get, routing::post, Router};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::scheduler::Storage;
use crate::tracker::{GroundStation, Tracker};

use super::api::schedules as schedule_handlers;
use super::api::tracker as tracker_handlers;
use super::api_doc::ApiDoc;
use super::auth::AppState;
use super::config::Config;
use super::ui::handlers as ui_handlers;

pub async fn run_server(config: Config) -> std::io::Result<()> {
    let bind_addr = config.web.bind.clone();
    let storage = Storage::new(config.schedules.base_folder.clone());
    let station = GroundStation::from_coordinates(
        &config.station.coordinates,
        Some(config.station.altitude_m),
    )
    .unwrap_or_default();
    let tracker = Tracker::new(station);

    let state = AppState {
        config: Arc::new(config),
        storage: Arc::new(storage),
        tracker: Arc::new(Mutex::new(tracker)),
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // UI routes
        .route("/", get(ui_handlers::dashboard))
        .route("/timeline", get(ui_handlers::timeline))
        // Schedule API endpoints
        .route("/api/schedules", post(schedule_handlers::submit_schedule))
        .route("/api/schedules", get(schedule_handlers::list_schedules))
        .route("/api/schedules/{id}", get(schedule_handlers::get_schedule))
        .route(
            "/api/schedules/{id}",
            delete(schedule_handlers::delete_schedule),
        )
        .route(
            "/api/schedules/{id}/approve",
            post(schedule_handlers::approve_schedule),
        )
        .route(
            "/api/schedules/{id}/reject",
            post(schedule_handlers::reject_schedule),
        )
        .route(
            "/api/schedules/validate",
            post(schedule_handlers::validate_schedule),
        )
        // Tracker API endpoints
        .route("/api/tracker/run", post(tracker_handlers::run))
        .route("/api/tracker/stop", post(tracker_handlers::stop))
        .route(
            "/api/tracker/status/mode",
            get(tracker_handlers::status_mode),
        )
        .route(
            "/api/tracker/status/sample",
            get(tracker_handlers::status_sample),
        )
        .route(
            "/api/tracker/status/trajectory",
            get(tracker_handlers::status_trajectory),
        )
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
