use serde::{Deserialize, Serialize};
use tokio::process::Command;

use crate::setup::{AppState, Myerror};

#[derive(Debug, Deserialize)]
struct Access {
    access_token: String,
}
#[derive(Debug, Deserialize)]
struct Secret {
    secret: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct User {
    pub login: String,
    pub id: usize,
    pub avatar_url: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Oauth {
    pub user: User,
    pub allowed: bool,
}

pub async fn redirect(code: String, app: &AppState) -> Result<Oauth, Myerror> {
    let setting = app.setting.read().await;
    let req = reqwest::Client::new();
    let secret = toml::from_str::<Secret>(&tokio::fs::read_to_string("./Secret.toml").await?)?;
    let body = [
        ("client_id", &setting.client_id),
        ("code", &code),
        ("client_secret", &secret.secret),
        ("redirect_uri", &setting.callback_url),
    ];
    let res = req
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&body)
        .send()
        .await?
        .json::<Access>()
        .await?;

    let res2 = req
        .get("https://api.github.com/user")
        .header("Authorization", &format!("Bearer {}", res.access_token))
        .header("User-Agent", "HadziqApp")
        .send()
        .await?
        .json::<User>()
        .await?;

    let allowed = &res2.id == &setting.user_id;
    if allowed {
        Command::new("sh")
            .args(&["-c", "npm run flow"])
            .spawn()?
            .wait()
            .await?;
        app.template.reload().await?;
        app.markdown.reload().await?;
    }
    Ok(Oauth {
        allowed,
        user: res2,
    })
}
