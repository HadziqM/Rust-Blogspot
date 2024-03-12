use crate::{
    model::{Content, Intro, Portfolio},
    setup::{AppState, HtmlOut},
};
use axum::{
    extract::{Path, State},
    response::Redirect,
    routing::get,
    Router,
};
use markdown::{Language, PostData, PostList, PostType};
use template::macros::PageRender;

#[derive(PageRender)]
pub enum MyPage {
    // 404 page
    #[error_page]
    #[location = "pages/404.html"]
    E404,
    // post page
    #[location = "pages/blog.html"]
    Post { post: PostData, language: Language },

    #[location = "pages/intro.html"]
    Intro { data: Intro, language: Language },

    #[location = "pages/portofolio.html"]
    Portofolio { data: Portfolio, language: Language },

    #[location = "pages/blog_list.html"]
    List {
        list: PostList,
        post: PostType,
        tag: Option<String>,
        language: Language,
    },
}

async fn index(app: AppState, language: Language) -> HtmlOut {
    let content = Content::new().await?.to_page(language);
    app.render(MyPage::Intro {
        data: content.intro,
        language,
    })
    .await
}
async fn portofolio(app: AppState, language: Language) -> HtmlOut {
    let content = Content::new().await?.to_page(language);
    app.render(MyPage::Portofolio {
        data: content.portfolio,
        language,
    })
    .await
}

async fn render_post(app: AppState, slug: String, post: PostType, language: Language) -> HtmlOut {
    app.render(MyPage::Post {
        post: app.markdown.get_post(language, post, slug).await?,
        language,
    })
    .await
}

async fn list(app: AppState, page: usize, post: PostType, language: Language) -> HtmlOut {
    app.render(MyPage::List {
        list: app.markdown.list(language, post, page).await,
        post,
        tag: None,
        language,
    })
    .await
}

pub async fn error(State(app): State<AppState>) -> HtmlOut {
    app.render(MyPage::E404).await
}

async fn list_tag(
    app: AppState,
    page: usize,
    post: PostType,
    language: Language,
    tag: String,
) -> HtmlOut {
    app.render(MyPage::List {
        list: app.markdown.list_from_tag(language, post, &tag, page).await,
        post,
        tag: None,
        language,
    })
    .await
}
async fn reload(State(app): State<AppState>) -> String {
    if app.template.reload().await.is_ok() {
        if app.markdown.reload().await.is_ok() {
            return "success".into();
        }
    }
    "failed".into()
}

async fn page_or_list(app: AppState, slug: String, post: PostType, language: Language) -> HtmlOut {
    if let Ok(page) = slug.parse::<usize>() {
        list(app, page, post, language).await
    } else {
        render_post(app, slug, post, language).await
    }
}

fn post_route(language: Language, post: PostType) -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(move |State(app): State<AppState>| page_or_list(app, "1".into(), post, language)),
        )
        .route(
            "/:slug",
            get(
                move |State(app): State<AppState>, Path(slug): Path<String>| {
                    page_or_list(app, slug, post, language)
                },
            ),
        )
        .route(
            "/tag/:tag/:page",
            get(
                move |State(app): State<AppState>, Path((tag, page)): Path<(String, usize)>| {
                    list_tag(app, page, post, language, tag)
                },
            ),
        )
}

fn lang_route(language: Language) -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(move |State(app): State<AppState>| index(app, language)),
        )
        .route("/reload", get(reload))
        .route(
            "/portfolio",
            get(move |State(app): State<AppState>| portofolio(app, language)),
        )
        .nest("/blog", post_route(language, PostType::Blog))
        .nest("/project", post_route(language, PostType::Project))
}

pub fn reg() -> Router<AppState> {
    Router::new()
        .route("/", get(|| async { Redirect::permanent("/en") }))
        .nest("/en", lang_route(Language::Eng))
        .nest("/id", lang_route(Language::Idn))
}
