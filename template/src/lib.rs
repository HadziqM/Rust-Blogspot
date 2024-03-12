use std::ops::Deref;
use std::sync::Arc;
use tera::{Context, Result, Tera};
use tokio::sync::RwLock;
pub use {macros, tera};

/// the tera model for SSR rendering, the output are string so warp them on HTML file when serving
#[derive(Clone)]
pub struct Templates(Arc<RwLock<Tera>>);

impl Templates {
    pub fn new(location: impl AsRef<str>) -> Self {
        Templates(Arc::new(RwLock::new(
            Tera::new(location.as_ref())
                .expect("the templates folder is not in current absolute path"),
        )))
    }
}

impl Default for Templates {
    fn default() -> Self {
        Self::new("./pages/templates/**/*.html")
    }
}

impl Deref for Templates {
    type Target = Arc<RwLock<Tera>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// trait to define rendering event
pub trait PageRender: Sized {
    /// path of the page relative on templates path ex. /templates/login.html is "login.html"
    fn path(&self) -> String;
    /// the context or the data that need to be injected to templates page/components, need to
    /// implement `serde::Serialize`
    fn context(&self) -> Context;
    fn err_page(&self) -> Option<Self>;
}

pub type Error = tera::Error;

impl Templates {
    async fn render_page<T: PageRender>(&self, page: &T) -> Result<String> {
        self.read().await.render(&page.path(), &page.context())
    }
    /// ### This what you mostly do to render/serve page
    /// render pages or components and serve `404` page if not found
    /// the return error only occure when serving the `404` page, so it safe to `unwrap` if you sure
    pub async fn render<T: PageRender>(&self, page: T) -> Result<String> {
        match self.render_page(&page).await {
            Ok(x) => Ok(x),
            Err(err) => {
                log::error!("error parsing current pages with err: {err:?}");
                if page.err_page().is_some() {
                    self.render_page(&page.err_page().unwrap()).await
                } else {
                    Err(err)
                }
            }
        }
    }

    /// blocking operation to reload the template to match the latest edit
    pub async fn reload(&self) -> Result<()> {
        self.write().await.full_reload()
    }
}
