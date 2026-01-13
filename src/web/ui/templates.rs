use askama::Template;
use askama_web::WebTemplate;

#[derive(Template, WebTemplate)]
#[template(path = "dashboard.html")]
pub struct DashboardTemplate {}

#[derive(Template, WebTemplate)]
#[template(path = "timeline.html")]
pub struct TimelineTemplate {}
