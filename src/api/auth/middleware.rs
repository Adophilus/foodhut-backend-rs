use axum::extract::{FromRequestParts, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::{async_trait, Json};
use axum::{
    extract::{Extension, Request},
    http,
    http::request::Parts,
    middleware::Next,
    response::Response,
};
use serde::Serialize;
use serde_json::json;

use crate::repository::user::User;
use crate::types::Context;
use crate::utils::{self, database::DatabaseConnection};
use crate::{repository, types};
use std::sync::Arc;

enum Error {
    InvalidSession,
}

fn get_session_id_from_header(header: String) -> Result<String, Error> {
    header
        .split(" ")
        .skip(1)
        .next()
        .map(|h| h.to_string())
        .ok_or(Error::InvalidSession)
}

async fn get_user_from_header(
    ctx: Arc<types::Context>,
    header: String,
) -> Result<repository::user::User, Error> {
    let session_id = get_session_id_from_header(header)?;
    let session = utils::auth::verify_access_token(ctx.clone(), session_id)
        .await
        .map_err(|_| Error::InvalidSession)?;
    repository::user::find_by_id(ctx.db_conn.clone(), session.user_id)
        .await
        .map_err(|_| Error::InvalidSession)?
        .ok_or(Error::InvalidSession)
}

// pub async fn auth(
//     req: Request,
//     State(ctx): State<Context>,
//     next: Next,
// ) -> Result<Response, ApiResponse<&'static str, &'static str>> {
//     match req
//         .headers()
//         .get(http::header::AUTHORIZATION)
//         .and_then(|header| header.to_str().ok())
//     {
//         Some(auth_header) => {
//             match get_user_from_header(ctx.db_conn.clone(), auth_header.to_string()).await {
//                 Ok(user) => Ok(next.run(req).await),
//                 Err(_) => Err(ApiResponse::err("Invalid session token")),
//             }
//         }
//         None => Err(ApiResponse::err("Invalid session token")),
//     }
// }

#[derive(Serialize, Clone)]
pub struct Auth {
    pub user: User,
}

async fn get_user_from_request<State: Send + Sync>(
    parts: &mut Parts,
    state: &State,
) -> Result<User, Response> {
    use axum::RequestPartsExt;
    let Extension(ctx) = parts.extract::<Extension<Arc<Context>>>().await.unwrap();
    let headers = parts.extract::<HeaderMap>().await.unwrap();

    let err = (
        StatusCode::UNAUTHORIZED,
        Json(json!({"error": "Invalid session token"})),
    );

    let auth_header = headers
        .get(http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .ok_or(err.clone().into_response())?;

    get_user_from_header(ctx.clone(), auth_header.to_string())
        .await
        .map_err(|_| err.clone().into_response())
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for Auth {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        get_user_from_request(parts, state)
            .await
            .map(|user| Self { user })
    }
}

#[derive(Serialize, Clone)]
pub struct AdminAuth {
    pub user: User,
}

#[async_trait]
impl<S: Send + Sync> FromRequestParts<S> for AdminAuth {
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        get_user_from_request(parts, state)
            .await
            .map(|user| Self { user })
    }
}
