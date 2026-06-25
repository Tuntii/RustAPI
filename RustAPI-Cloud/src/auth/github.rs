use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GithubTokenResponse {
    pub access_token: String,
    #[allow(dead_code)]
    pub token_type: String,
    #[allow(dead_code)]
    pub scope: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct GithubUser {
    pub id: i64,
    pub login: String,
    pub avatar_url: Option<String>,
    pub email: Option<String>,
}

pub async fn exchange_code(
    code: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
) -> Result<GithubTokenResponse, reqwest::Error> {
    let client = reqwest::Client::new();
    let response = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("code", code),
            ("redirect_uri", redirect_uri),
        ])
        .send()
        .await?
        .json::<GithubTokenResponse>()
        .await?;

    Ok(response)
}

pub async fn get_user(access_token: &str) -> Result<GithubUser, reqwest::Error> {
    let client = reqwest::Client::new();
    let user = client
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", access_token))
        .header("User-Agent", "RustAPI-Cloud")
        .send()
        .await?
        .json::<GithubUser>()
        .await?;

    Ok(user)
}
