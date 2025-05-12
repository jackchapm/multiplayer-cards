use std::env;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use anyhow::Error;
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use crate::game::GameId;

pub const TOKEN_EXPIRY: u64 = 60 * 60;
// expiry doesn't really matter as players can just POST /game/join again to get a new token
// only useful for reconnecting if connection drops
pub const WEBSOCKET_TOKEN_EXPIRY: u64 = 60 * 60 * 24;
pub const HTTP_AUDIENCE: &'static str = "cards";
pub const WEBSOCKET_AUDIENCE: &'static str = "websocket";

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub aud: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub game_id: Option<GameId>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all="camelCase")]
pub struct AuthorizationContext {
    pub uuid: String,
    pub expires: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub game_id: Option<GameId>
}

pub async fn generate_jwt(user_id: &str, expiry: u64, game_id: Option<&GameId>) -> Result<String, Error> {
    let expiration = SystemTime::now()
        .duration_since(UNIX_EPOCH)?
        .checked_add(Duration::from_secs(expiry))
        .unwrap()
        .as_secs() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration,
        aud: game_id.as_ref().map_or("cards", |_| "websocket").to_string(),
        game_id: game_id.cloned()
    };

    let secret = env::var("JWT_SECRET").expect("expected jwt secret");
    let encoding_key = EncodingKey::from_secret(secret.as_bytes());

    Ok(encode(&Header::default(), &claims, &encoding_key)?)
}
