use lambda_http::{
    request::RequestContext, run, service_fn, tracing, Body, Error, IntoResponse, Request,
    RequestExt, Response,
};
use multiplayer_cards::db_utils::Connection;
use multiplayer_cards::game::{Game, Player, PlayerId};
use multiplayer_cards::message::WebsocketResponse::Success;
use multiplayer_cards::message::{WebsocketRequest, WebsocketResponse};
use multiplayer_cards::{Services, WebsocketError};

async fn websocket_handler(event: Request, services: &Services) -> Result<Response<Body>, Error> {
    let RequestContext::WebSocket(context) = event.request_context() else {
        return Err("expected websocket".into());
    };

    // Can safely call unwrap as this header has been authorized by lambda authorizer
    let uuid = context
        .authorizer
        .fields
        .get("uuid")
        .expect("request authorizer not found")
        .as_str()
        .unwrap()
        .to_string();

    let conn_id = &context.connection_id.expect("no connection id received");

    match context.route_key.expect("no route key").as_str() {
        "$connect" => {
            if let Some(old_connection) = services.get::<Connection>(&uuid).await {
                // Try disconnect any current open connection
                // It's okay if this is unsuccessful, the other connection will hang as there is no reference to its connection id
                services
                    .expect_apigw()
                    .delete_connection()
                    .connection_id(old_connection)
                    .send()
                    .await?;
            }

            let _ = services.put::<Connection>(&uuid, conn_id).await;
        }
        "$disconnect" => {
            let _ = services.delete::<Connection>(&uuid, Some(conn_id)).await;
        }
        "$default" => {
            let response = match WebsocketRequest::try_from(event) {
                Ok(message) => handle_message(services, message, uuid, conn_id)
                    .await
                    .unwrap_or_else(|e| WebsocketError::ServiceError(e.to_string()).into()),
                Err(error) => error.into(),
            };

            services.send(conn_id, &response).await?;
        }
        _ => return Err("unhandled message".into()),
    }

    Ok("handled request".into_response().await)
}

async fn handle_message(
    services: &Services,
    message: WebsocketRequest,
    uuid: PlayerId,
    conn_id: &str,
) -> Result<WebsocketResponse, Error> {
    // todo fix possible race conditions when pulling from db
    Ok(match message {
        WebsocketRequest::CreateGame { name, deck_options } => {
            if let Some(_) = services.get::<Player>(&uuid).await {
                WebsocketError::AlreadyInGame.into()
            } else {
                Game::new(services, uuid, conn_id, &deck_options).await?;
                Success
            }
        }
        WebsocketRequest::DrawCardToHand { deck } => todo!(),
        WebsocketRequest::JoinGame { game_id } => {
            let game = services.get::<Game>(&game_id).await;
            if let Some(mut game) = game {
                game.add_player(services, uuid, conn_id).await?;
                Success
            } else {
                WebsocketError::NonExistentGame(game_id).into()
            }
        }
        WebsocketRequest::LeaveGame => {
            if let Some(player) = services.get::<Player>(&uuid).await {
                let mut game = player.get_game(services).await;
                game.remove_player(services, uuid, false).await?;
                Success
            } else {
                WebsocketError::NotInGame.into()
            }
        }
        WebsocketRequest::Ping => {
            if let Some(player) = services.get::<Player>(&uuid).await {
                player.send_state(services, Some(conn_id)).await?;
                let game = player.get_game(services).await;
                game.send_state(services, conn_id).await?;
                for deck in &game.decks {
                    game.send_deck_state(services, deck, conn_id).await?;
                }
            }

            WebsocketResponse::Pong
        }
    })
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    let services = Services::create().await;

    run(service_fn(async |request| {
        websocket_handler(request, &services).await
    }))
    .await
}
