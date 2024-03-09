use std::ops::Deref;
use tera::{Tera, Context, Result};
use tokio::sync::RwLock;
use macros::PageRender;
use std::sync::Arc;



/// the tera model for SSR rendering, the output are string so warp them on HTML file when serving
#[derive(Clone)]
pub struct Templates(Arc<RwLock<Tera>>);

impl Templates {
    pub fn new(location: impl AsRef<str>) -> Self {
        Templates(Arc::new(RwLock::new(Tera::new(location.as_ref())
            .expect("the templates folder is not in current absolute path"))))
    }
}

impl Default for Templates {
    fn default() -> Self {
        Self::new("./public/templates/**/*.html")
    }
}

impl Deref for Templates {
    type Target = Arc<RwLock<Tera>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


/// trait to define rendering event
pub trait PageRender {
    /// path of the page relative on templates path ex. /templates/login.html is "login.html"
    fn path(&self) -> String;
    /// the context or the data that need to be injected to templates page/components, need to
    /// implement `serde::Serialize`
    fn context(&self) -> Context;
}

pub type Error = tera::Error;


#[derive(PageRender)]
pub enum Pages {
    #[location = "404.html"]
    E404,
    #[location = "pages/login.html"]
    Login {title:String}
}


impl Templates {
    async fn render_page(&self,page:impl PageRender) -> Result<String> {
        self.read().await.render(&page.path(), &page.context())
    }
    /// ### This what you mostly do to render/serve page
    /// render pages or components and serve `404` page if not found
    /// the return error only occure when serving the `404` page, so it safe to `unwrap` if you sure
    pub async fn render<T:PageRender>(&self,page:T) -> Result<String> {
        match self.render_page(page).await {
            Ok(x) => Ok(x),
            Err(err) => {
                log::error!("error parsing current pages with err: {err:?}");
                self.render_page(Pages::E404).await
            }
        }
    }

    /// blocking operation to reload the template to match the latest edit
    pub async fn reload(&self) -> Result<()> {
        self.write().await.full_reload()
    }
}
