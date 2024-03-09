use std::{sync::Arc, collections::HashMap, path::{PathBuf, Path}, ops::Deref};
use gray_matter::{Matter, engine::YAML};
use pulldown_cmark::{Parser, Options};
use serde::{Serialize,Deserialize};
use thiserror::Error;
use tokio::sync::RwLock;
use indexmap::IndexMap;



#[derive(Serialize,Deserialize,Clone)]
pub struct MetaData {
    pub title: String,
    pub date: String,
    pub description: String,
    pub image: String,
    pub tags: Vec<String>,
}

#[derive(Serialize,Deserialize,Clone)]
pub struct PostData {
    pub metadata: MetaData,
    pub content: String
}

#[derive(Hash,PartialEq, Eq)]
pub enum PostType {
    Project,
    Blog
}

#[derive(Error,Debug)]
pub enum Error {
    #[error("cant get access to filesystem or not found")]
    FileError,
    #[error("invalid Yaml Meta format")]
    MetaError
}

pub type MyResult<T> = Result<T,Error>;
type MarkdownData = HashMap<PostType,IndexMap<String,PostData>>;

#[derive(Clone)]
pub struct Markdown(Arc<RwLock<MarkdownData>>);

impl Deref for Markdown {
    type Target = Arc<RwLock<MarkdownData>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}


async fn parse_content(path:PathBuf) -> MyResult<PostData> {
    if let Ok(text) = tokio::fs::read_to_string(path).await {
        let matter = Matter::<YAML>::new();
        let metadata = matter.parse_with_struct::<MetaData>(&text).ok_or(Error::MetaError)?;
        let parser = Parser::new_ext(&metadata.content,Options::ENABLE_HEADING_ATTRIBUTES);

        // to do get Table of Content
        let mut html = "".to_string();
        pulldown_cmark::html::push_html(&mut html, parser);
        return Ok(PostData { metadata: metadata.data , content: html });
    }
    Err(Error::FileError)
}

async fn parse_from_path(dir:PathBuf) -> MyResult<IndexMap<String,PostData>> {
    let mut list = tokio::fs::read_dir(dir).await.map_err(|_|Error::FileError)?;
    let mut out = IndexMap::new();
    while let Ok(Some(entry)) = list.next_entry().await {
        let path = entry.path();
        if path.ends_with(".md") {
            if let Some(name) = path.file_stem() {
                out.insert(name.to_str().unwrap().to_owned(),parse_content(path).await? );
            }
        }
    }
    // sort by date desc (must be format Y-M-D)
    out.sort_by(|_,a,_,b|{
        b.metadata.date.cmp(&a.metadata.date)
    });
    Ok(out)
}


impl Markdown {
    async fn init() -> MyResult<MarkdownData> {
        let path = Path::new(".").join("public").join("markdown");
        Ok(HashMap::from_iter([
            (PostType::Blog,parse_from_path(path.join("blog")).await?),
            (PostType::Project,parse_from_path(path.join("project")).await?)
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
    /// listing all the post
    pub async fn list(&self,post:PostType) -> IndexMap<String,MetaData> {
        self.read().await.get(&post).unwrap().into_iter().map(|(k,v)|(k.to_owned(),v.metadata.to_owned())).collect()
    }
    /// listing all the post that have specific tag
    pub async fn list_from_tag(&self,post:PostType, tag: impl ToString) -> IndexMap<String,MetaData> {
        self.read().await.get(&post).unwrap().into_iter().filter_map(|(k,v)| {
            v.metadata.tags.contains(&tag.to_string())
                .then_some((k.to_owned(),v.metadata.to_owned()))
        }).collect()
    }
    /// get the rendered html and metadata
    pub async fn get_page(&self,post:PostType,slug: impl ToString) -> Option<PostData> {
        self.read().await.get(&post).unwrap().get(&slug.to_string()).cloned()
    }
}
