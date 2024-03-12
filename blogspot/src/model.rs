use crate::setup::ThisResult;
use markdown::Language;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Content {
    pub indonesia: Pages,
    pub english: Pages,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Pages {
    pub intro: Intro,
    pub portfolio: Portfolio,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Intro {
    pub greet: String,
    pub description: String,
    pub about: Vec<About>,
}
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Portfolio {
    pub occupation: String,
    pub address: String,
    pub skills: Vec<Skills>,
    pub experiences: Vec<Experiences>,
    pub educations: Vec<Experiences>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Experiences {
    pub name: String,
    pub date: String,
    pub description: String,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Skills {
    pub name: String,
    pub prof: u8,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct About {
    pub name: String,
    pub description: String,
}

impl Content {
    pub async fn new() -> ThisResult<Self> {
        Ok(toml::from_str(
            &tokio::fs::read_to_string("./Content.toml").await?,
        )?)
    }
    pub fn to_page(self, language: Language) -> Pages {
        match language {
            Language::Eng => self.english,
            Language::Idn => self.indonesia,
        }
    }
}
