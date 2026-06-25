//! Mint a JWT for verification scripts (stdout: access token only).
use std::env;

fn main() {
    let user_id = env::args().nth(1).expect("usage: mint-verify-jwt <user_id> [login] [tier]");
    let login = env::args().nth(2).unwrap_or_else(|| "verify-cli".into());
    let tier = env::args().nth(3).unwrap_or_else(|| "hobby".into());
    let secret = env::var("JWT_SECRET").unwrap_or_else(|_| "dev-secret-change-in-production".into());

    let (token, _) = rustapi_cloud::auth::jwt::create_token(
        &user_id,
        &login,
        None,
        &tier,
        &secret,
        24,
    )
    .expect("mint jwt");

    println!("{token}");
}