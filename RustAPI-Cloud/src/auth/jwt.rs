use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,   // user id
    pub login: String, // GitHub login
    pub avatar_url: Option<String>,
    pub tier: String,
    pub exp: usize, // expiration
    pub iat: usize, // issued at
}

pub fn create_token(
    user_id: &str,
    login: &str,
    avatar_url: Option<&str>,
    tier: &str,
    secret: &str,
    ttl_hours: i64,
) -> Result<(String, String), jsonwebtoken::errors::Error> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;

    let access_claims = Claims {
        sub: user_id.to_string(),
        login: login.to_string(),
        avatar_url: avatar_url.map(|s| s.to_string()),
        tier: tier.to_string(),
        exp: now + 3600, // 1 hour
        iat: now,
    };

    let refresh_claims = Claims {
        sub: user_id.to_string(),
        login: login.to_string(),
        avatar_url: None,
        tier: tier.to_string(),
        exp: now + (ttl_hours * 3600) as usize,
        iat: now,
    };

    let access_token = encode(
        &Header::default(),
        &access_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    let refresh_token = encode(
        &Header::default(),
        &refresh_claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok((access_token, refresh_token))
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}
