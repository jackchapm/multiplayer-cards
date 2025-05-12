use anyhow::anyhow;
use lambda_http::http::StatusCode;
use lambda_http::request::RequestContext;
use lambda_http::{
    run, service_fn, tracing, Body, Error, IntoResponse, Request, RequestExt, RequestPayloadExt,
    Response,
};
use multiplayer_cards::auth::{generate_jwt, TOKEN_EXPIRY, WEBSOCKET_TOKEN_EXPIRY};
use multiplayer_cards::db_utils::RefreshToken;
use multiplayer_cards::game::Game;
use multiplayer_cards::requests::{CreateGameRequest, JoinGameRequest, JoinGameResponse};
use multiplayer_cards::utils::AuthorizerUtils;
use multiplayer_cards::{Services, WebsocketError};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub expires_in: u64,
    pub refresh_token: String,
}

async fn guest_login_handler(
    _event: Request,
    services: &Services,
) -> Result<Response<Body>, Error> {
    let uuid = Uuid::new_v4().to_string();
    let access_token = generate_jwt(&uuid, TOKEN_EXPIRY, None).await?;
    let refresh_token = generate_refresh_token();

    let _ = services.put::<RefreshToken>(&refresh_token, &uuid).await?;

    Ok(json!(AuthResponse {
        access_token,
        expires_in: TOKEN_EXPIRY,
        refresh_token
    })
    .into_response()
    .await)
}

async fn refresh_token_handler(
    event: Request,
    services: &Services,
) -> Result<Response<Body>, Error> {
    let auth_header = event
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    let Some(token_str) = (match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            Some(header.trim_start_matches("Bearer ").trim())
        }
        _ => None,
    }) else {
        return Ok((StatusCode::UNAUTHORIZED, "Missing authorization header")
            .into_response()
            .await);
    };

    let Ok(uuid) = services
        .delete::<RefreshToken>(&token_str.to_string(), None)
        .await
    else {
        return Ok((StatusCode::UNAUTHORIZED, "Unauthorized")
            .into_response()
            .await);
    };
    let access_token = generate_jwt(&uuid, TOKEN_EXPIRY, None).await?;
    let new_refresh_token = generate_refresh_token();
    let _ = services
        .put::<RefreshToken>(&new_refresh_token, &uuid)
        .await?;

    Ok(json!(AuthResponse {
        access_token,
        expires_in: TOKEN_EXPIRY,
        refresh_token: new_refresh_token
    })
    .into_response()
    .await)
}

async fn create_game_handler(event: Request, services: &Services) -> Result<Response<Body>, Error> {
    let uuid = event
        .request_context()
        .authorizer()
        .unwrap()
        .unwrap_field("uuid");
    let Ok(Some(request)) = event.payload::<CreateGameRequest>() else {
        return Ok((
            StatusCode::BAD_REQUEST,
            json!(WebsocketError::InvalidRequest("error parsing json")),
        )
            .into_response()
            .await);
    };

    // If player is currently in a game they will be removed when reconnecting to the websocket
    // if let Some(_) = services.get::<Player>(&uuid).await {
    //     Ok((StatusCode::BAD_REQUEST, json!(WebsocketError::AlreadyInGame)).into_response().await)
    // } else {
    let game = Game::new(services, uuid.clone(), request.deck_type).await?;
    let token = generate_jwt(uuid.as_str(), WEBSOCKET_TOKEN_EXPIRY, Some(&game.id)).await?;
    Ok(json!(JoinGameResponse {
        game_id: game.id,
        token
    })
    .into_response()
    .await)
    // }
}

async fn join_game_handler(event: Request, services: &Services) -> Result<Response<Body>, Error> {
    let uuid = event
        .request_context()
        .authorizer()
        .unwrap()
        .unwrap_field("uuid");
    let Ok(Some(request)) = event.payload::<JoinGameRequest>() else {
        return Ok((
            StatusCode::BAD_REQUEST,
            json!(WebsocketError::InvalidRequest("error parsing json")),
        )
            .into_response()
            .await);
    };
    let game = services.get::<Game>(&request.game_id).await;
    if let Some(mut game) = game {
        let token = generate_jwt(uuid.as_str(), WEBSOCKET_TOKEN_EXPIRY, Some(&game.id)).await?;
        game.add_authorized_player(services, uuid).await?;
        Ok(json!(JoinGameResponse {
            game_id: game.id,
            token
        })
        .into_response()
        .await)
    } else {
        Ok((
            StatusCode::BAD_REQUEST,
            json!(WebsocketError::NonExistentGame(request.game_id.clone())),
        )
            .into_response()
            .await)
    }
}

pub fn generate_refresh_token() -> String {
    format!("refresh_{}", Uuid::new_v4())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();
    let services = Services::create().await;
    run(service_fn(async |event: Request| {
        let RequestContext::ApiGatewayV2(context) = event.request_context() else {
            return Err(anyhow!("function only handles http requests").into());
        };

        let route_key = context
            .route_key
            .and_then(|key| key.split_once(' ').map(|(_, route)| route.to_string()))
            .expect("route key expected");

        match route_key.as_str() {
            "/auth/guest" => guest_login_handler(event, &services).await,
            "/auth/refresh" => refresh_token_handler(event, &services).await,
            // Authorized routes
            "/game/create" => create_game_handler(event, &services).await,
            "/game/join" => join_game_handler(event, &services).await,
            _ => Ok(Response::builder()
                .status(400)
                .body(format!("Unknown route {route_key}").into())?),
        }
    }))
    .await
}
