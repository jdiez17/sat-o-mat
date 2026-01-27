use utoipa::{
    openapi::security::{Http, HttpAuthScheme, SecurityScheme},
    Modify, OpenApi,
};

use crate::web::api::error::ErrorResponse;

use super::api::predict::{PredictQuery, PredictResponse};
use super::api::schedules::{ListSchedulesQuery, ScheduleDetailResponse, ScheduleVariable};
use crate::tracker::RunCommand;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::web::api::schedules::submit_schedule,
        crate::web::api::schedules::list_schedules,
        crate::web::api::schedules::get_schedule,
        crate::web::api::schedules::delete_schedule,
        crate::web::api::schedules::approve_schedule,
        crate::web::api::schedules::reject_schedule,
        crate::web::api::tracker::run,
        crate::web::api::tracker::stop,
        crate::web::api::tracker::status_mode,
        crate::web::api::tracker::status_sample,
        crate::web::api::tracker::status_trajectory,
        crate::web::api::predict::list_predictions,
    ),
    components(
        schemas(
            ScheduleDetailResponse,
            ScheduleVariable,
            ErrorResponse,
            ListSchedulesQuery,
            crate::scheduler::storage::ScheduleEntry,
            crate::scheduler::storage::ScheduleState,
            crate::scheduler::approval::ApprovalResult,
            RunCommand,
            crate::tracker::TrackerMode,
            crate::tracker::TrackerSample,
            PredictQuery,
            PredictResponse,
            crate::predict::Pass,
        )
    ),
    modifiers(&SecurityAddon),
    info(
        title = "Sat-O-Mat Schedule API",
        description = "API for managing satellite pass schedules",
        version = "0.1.0"
    ),
    tags(
        (name = "schedules", description = "Schedule management"),
        (name = "tracker", description = "Tracker control"),
        (name = "predict", description = "Satellite pass predictions")
    )
)]
pub struct ApiDoc;

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "api_key",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            );
        }
    }
}
