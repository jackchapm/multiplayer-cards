use anyhow::anyhow;
use jsonwebtoken::{encode, EncodingKey, Header};
use lambda_http::request::RequestContext;
use lambda_http::{run, service_fn, tracing, Body, Error, IntoResponse, Request, RequestExt, Response};
use aws_sdk_dynamodb as dynamodb;
use multiplayer_cards::Claims;
use std::env;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use lambda_http::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use multiplayer_cards::db_utils::{DynamoDBClient, RefreshToken};

const TOKEN_EXPIRY: u64 = 60*60;

#[derive(Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub refresh_token: String,
}

async fn guest_login_handler(_event: Request, client: &DynamoDBClient) -> Result<Response<Body>, Error> {
    let uuid = Uuid::new_v4().to_string();
    let access_token = generate_jwt(&uuid).await?;
    let refresh_token = generate_refresh_token();

    let _ = client.put_entry::<RefreshToken>(&refresh_token, &uuid).await?;

    Ok(json!(AuthResponse {
        access_token,
        expires_in: TOKEN_EXPIRY,
        refresh_token
    }).into_response().await)
}

async fn refresh_token_handler(event: Request, client: &DynamoDBClient) -> Result<Response<Body>, Error> {
    let auth_header = event.headers().get("Authorization").and_then(|h| h.to_str().ok());
    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => Some(header.trim_start_matches("Bearer ").trim()),
        _ => None
    };

    if let Some(token_str) = token {
        // todo these possibly aren't getting deleted
        let resp = client.delete_entry::<RefreshToken>(&token_str.to_string(), None).await;
        if let Ok(uuid) = resp {
            let access_token = generate_jwt(&uuid).await?;
            let new_refresh_token = generate_refresh_token();
            let _ = client.put_entry::<RefreshToken>(&new_refresh_token, &uuid).await?;

            Ok(json!(AuthResponse {
                access_token,
                expires_in: TOKEN_EXPIRY,
                refresh_token: new_refresh_token
            }).into_response().await)
        } else {
            Ok((StatusCode::UNAUTHORIZED, "Unauthorized").into_response().await)
        }
    } else {
        Ok((StatusCode::UNAUTHORIZED, "Missing authorization header").into_response().await)
    }
}

async fn generate_jwt(user_id: &str) -> Result<String, Error> {
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .checked_add(Duration::from_secs(TOKEN_EXPIRY))
        .unwrap()
        .as_secs() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration,
    };

    let secret = env::var("JWT_SECRET").expect("expected jwt secret");
    let encoding_key = EncodingKey::from_secret(secret.as_bytes());

    Ok(encode(&Header::default(), &claims, &encoding_key)?)
}
pub fn generate_refresh_token() -> String {
    format!("refresh_{}", Uuid::new_v4())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();
    let table_name = env::var("TABLE_NAME").expect("table name not provided");
    let client = DynamoDBClient::new(
        dynamodb::Client::new(&aws_config::load_from_env().await),
        table_name
    );

    run(service_fn(async |event: Request| {
        let route_key = if let RequestContext::ApiGatewayV2(context) = event.request_context() {
            context
                .route_key
                .and_then(|key| key.split_once(' ').map(|(_, route)| route.to_string()))
                .expect("route key expected")
        } else {
            return Err(anyhow!("function only handles http requests").into());
        };

        match route_key.as_str() {
            "/guest" => guest_login_handler(event, &client).await,
            "/refresh" => refresh_token_handler(event, &client).await,
            _ => Ok(Response::builder()
                .status(400)
                .body(format!("Unknown route {route_key}").into())?),
        }
    }))
    .await
}
