use axum::{
    handler::Handler,
    response::{Html, IntoResponse},
    Router,
};
use markdown::Markdown;
use std::sync::Arc;
use template::{PageRender, Templates};
use thiserror::Error;
use tokio::sync::RwLock;
use tower_http::services::ServeDir;

use crate::routes::error;

#[derive(serde::Deserialize, Clone)]
pub struct Setting {
    pub listen_addr: String,
    // pub tag_list: Vec<String>,
    pub github_pull: String,
    pub oauth_url: String,
    pub user_id: String,
}

#[derive(Clone)]
pub struct AppState {
    pub template: Templates,
    pub markdown: Markdown,
    pub setting: Arc<RwLock<Setting>>,
}

#[derive(Error, Debug)]
pub enum Myerror {
    #[error("markdown rendering error")]
    Markdown(#[from] markdown::Error),
    #[error("template rendering error")]
    Template(#[from] template::Error),
    #[error("toml deserialize error")]
    Toml(#[from] toml::de::Error),
    #[error("tokio I/O error")]
    Tokio(#[from] tokio::io::Error),
}

impl IntoResponse for Myerror {
    fn into_response(self) -> axum::response::Response {
        log::error!("got higher order error: {:?}", &self);
        self.to_string().into_response()
    }
}

pub type ThisResult<T> = Result<T, Myerror>;
pub type HtmlOut = ThisResult<Html<String>>;

impl AppState {
    async fn new() -> ThisResult<Self> {
        let template = Templates::default();
        let markdown = Markdown::new().await?;
        let setting = Arc::new(RwLock::new(
            toml::from_str::<Setting>(
                &tokio::fs::read_to_string("./Setting.toml")
                    .await
                    .expect("cant locate Setting.toml on project folder"),
            )
            .expect("the content of Setting.toml are invalid"),
        ));
        Ok(Self {
            template,
            markdown,
            setting,
        })
    }
    pub async fn render(&self, page: impl PageRender) -> HtmlOut {
        Ok(Html(self.template.render(page).await?))
    }
}

pub struct Setup {
    route: Router<AppState>,
}

impl Setup {
    pub fn new(route: Router<AppState>) -> Self {
        Self { route }
    }
    pub fn add_route(self, path: impl ToString, route: Router<AppState>) -> Self {
        Self {
            route: self.route.nest(&path.to_string(), route),
        }
    }
    pub async fn initialize(self) {
        simple_logger::init().ok();
        let state = AppState::new().await.expect("cant start the server state");
        let app = self.route.with_state(state.clone()).fallback_service(
            ServeDir::new("./public").not_found_service(Handler::with_state(error, state.clone())),
        );
        let listener = tokio::net::TcpListener::bind(&state.setting.read().await.listen_addr)
            .await
            .expect("the ip or port are occupied");
        axum::serve(listener, app.into_make_service())
            .await
            .expect("the route conflicting");
    }
}
