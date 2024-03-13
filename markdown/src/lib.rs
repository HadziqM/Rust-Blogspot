use gray_matter::{engine::YAML, Matter};
pub use indexmap::IndexMap;
use pulldown_cmark::{Options, Parser};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    ffi::OsStr,
    ops::{AddAssign, Deref},
    path::{Path, PathBuf},
    sync::Arc,
};
use thiserror::Error;
use tokio::sync::RwLock;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct MetaData {
    pub title: String,
    pub date: String,
    pub description: String,
    pub image: String,
    pub tags: Vec<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PostData {
    pub metadata: MetaData,
    pub content: String,
}

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy, Serialize)]
pub enum PostType {
    #[serde(rename = "project")]
    Project,
    #[serde(rename = "blog")]
    Blog,
}

#[derive(Hash, PartialEq, Eq, Debug, Clone, Copy, Serialize)]
pub enum Language {
    #[serde(rename = "en")]
    Eng,
    #[serde(rename = "id")]
    Idn,
}

/// pagination struct
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Pagination {
    pub current: usize,
    pub end: Vec<usize>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PostList {
    pub data: IndexMap<String, MetaData>,
    pub pagination: Pagination,
}

#[derive(Error, Debug)]
pub enum Error {
    #[error("cant get access to filesystem or not found")]
    FileError,
    #[error("invalid Yaml Meta format")]
    MetaError,
}

pub type MyResult<T> = Result<T, Error>;
type MarkdownData = HashMap<Language, HashMap<PostType, IndexMap<String, PostData>>>;

#[derive(Clone, Debug)]
pub struct Markdown(Arc<RwLock<MarkdownData>>);

impl Deref for Markdown {
    type Target = Arc<RwLock<MarkdownData>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

async fn parse_content(path: PathBuf) -> MyResult<PostData> {
    if let Ok(text) = tokio::fs::read_to_string(path).await {
        let matter = Matter::<YAML>::new();
        let metadata = matter
            .parse_with_struct::<MetaData>(&text)
            .ok_or(Error::MetaError)?;
        let parser = Parser::new_ext(&metadata.content, Options::ENABLE_HEADING_ATTRIBUTES);

        // to do get Table of Content
        let mut html = "".to_string();
        pulldown_cmark::html::push_html(&mut html, parser);
        return Ok(PostData {
            metadata: metadata.data,
            content: html,
        });
    }
    Err(Error::FileError)
}

async fn parse_from_path(dir: PathBuf) -> MyResult<IndexMap<String, PostData>> {
    let mut list = tokio::fs::read_dir(dir)
        .await
        .map_err(|_| Error::FileError)?;
    let mut out = IndexMap::new();
    while let Ok(Some(entry)) = list.next_entry().await {
        let path = entry.path();
        let extension = path.extension().unwrap_or_default();
        if extension == OsStr::new("md") {
            if let Some(name) = path.file_stem() {
                out.insert(
                    name.to_str().unwrap().to_owned(),
                    parse_content(path).await?,
                );
            }
        }
    }
    // sort by date desc (must be format Y-M-D)
    out.sort_by(|_, a, _, b| b.metadata.date.cmp(&a.metadata.date));
    Ok(out)
}

impl Markdown {
    async fn init() -> MyResult<MarkdownData> {
        let path = Path::new(".").join("pages").join("markdown");
        let en = path.join("en");
        let id = path.join("id");
        Ok(HashMap::from_iter([
            (
                Language::Eng,
                HashMap::from_iter([
                    (PostType::Blog, parse_from_path(en.join("blog")).await?),
                    (
                        PostType::Project,
                        parse_from_path(en.join("project")).await?,
                    ),
                ]),
            ),
            (
                Language::Idn,
                HashMap::from_iter([
                    (PostType::Blog, parse_from_path(id.join("blog")).await?),
                    (
                        PostType::Project,
                        parse_from_path(id.join("project")).await?,
                    ),
                ]),
            ),
        ]))
    }
    /// get instance of struct on default path ./public/markdown
    pub async fn new() -> MyResult<Self> {
        Ok(Markdown(Arc::new(RwLock::new(Self::init().await?))))
    }
    /// reload data to match latest
    pub async fn reload(&self) -> MyResult<()> {
        *self.write().await = Self::init().await?;
        Ok(())
    }
    /// listing all the post 6 per page
    async fn data_list(
        &self,
        language: Language,
        post: PostType,
        page: usize,
        func: impl Fn((&String, &PostData)) -> Option<(String, MetaData)>,
    ) -> PostList {
        let range;
        let binding_crap = self.read().await;
        let data = binding_crap.get(&language).unwrap().get(&post).unwrap();
        if data.len() < page * 6 {
            range = (page - 1) * 6..data.len();
        } else {
            range = (page - 1) * 6..page * 6;
        }
        let mut endd = data.len() / 6;
        (data.len() % 6 != 0).then(|| endd.add_assign(1));
        let end = (1..endd + 1).into_iter().collect();
        let pagination = Pagination { current: page, end };
        PostList {
            data: data
                .get_range(range)
                .unwrap()
                .into_iter()
                .filter_map(func)
                .collect(),
            pagination,
        }
    }
    pub async fn list(&self, language: Language, post: PostType, page: usize) -> PostList {
        self.data_list(language, post, page, |(k, v)| {
            Some((k.to_owned(), v.metadata.to_owned()))
        })
        .await
    }
    /// listing all the post that have specific tag
    pub async fn list_from_tag(
        &self,
        language: Language,
        post: PostType,
        tag: impl ToString,
        page: usize,
    ) -> PostList {
        self.data_list(language, post, page, |(k, v)| {
            v.metadata
                .tags
                .contains(&tag.to_string())
                .then_some((k.to_owned(), v.metadata.to_owned()))
        })
        .await
    }
    /// get the rendered html and metadata
    pub async fn get_post(
        &self,
        language: Language,
        post: PostType,
        slug: impl ToString,
    ) -> MyResult<PostData> {
        self.read()
            .await
            .get(&language)
            .unwrap()
            .get(&post)
            .unwrap()
            .get(&slug.to_string())
            .cloned()
            .ok_or(Error::FileError)
    }
}
