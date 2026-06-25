use rustapi_rs::prelude::*;

use crate::auth::bearer::BearerAuth;
use crate::deploy::DeployResponse;
use crate::state::AppState;

pub async fn deploy_create(
    headers: Headers,
    State(state): State<AppState>,
    multipart: Multipart,
) -> Result<Json<DeployResponse>, ApiError> {
    let auth = BearerAuth::from_headers(&headers.0, &state.config.jwt_secret)?;
    let response = state
        .deploy_service
        .create_from_multipart(auth.user_id(), multipart)
        .await?;
    Ok(Json(response))
}

pub async fn deploy_status(
    headers: Headers,
    State(state): State<AppState>,
    Path(deploy_id): Path<String>,
) -> Result<Json<DeployResponse>, ApiError> {
    let auth = BearerAuth::from_headers(&headers.0, &state.config.jwt_secret)?;
    let response = state
        .deploy_service
        .get_status(auth.user_id(), &deploy_id)
        .await?;
    Ok(Json(response))
}