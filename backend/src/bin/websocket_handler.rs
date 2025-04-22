use lambda_http::{
    request::RequestContext, run, service_fn, tracing, Body, Error, IntoResponse, Request,
    RequestExt, Response,
};
use lambda_http::http::StatusCode;
use multiplayer_cards::db_utils::Connection;
use multiplayer_cards::game::{Game, GameId, PlayerId};
use multiplayer_cards::requests::WebsocketResponse::{CloseGame, NoResponse, Success};
use multiplayer_cards::requests::{WebsocketRequest, WebsocketResponse};
use multiplayer_cards::{Services, WebsocketError};
use multiplayer_cards::utils::AuthorizerUtils;

async fn websocket_handler(event: Request, services: &Services) -> Result<Response<Body>, Error> {
    let RequestContext::WebSocket(context) = event.request_context() else {
        return Err("expected websocket".into());
    };

    // Can safely call unwrap as this header has been authorized by lambda authorizer
    let uuid = context.authorizer.unwrap_field("uuid");
    let game_id = context.authorizer.unwrap_field("gameId");
    let conn_id = &context.connection_id.expect("no connection id received");

    match context.route_key.expect("no route key").as_str() {
        "$connect" => {
            if let Some(old_connection) = services.get::<Connection>(&uuid).await {
                // Try disconnect any current open connection
                // It's okay if this is unsuccessful, the other connection will hang as there is no reference to its connection id
                let _ = services.delete_connection(&old_connection).await;
            }
            
            if services.get::<Game>(&game_id).await.is_none() {
                return Ok((StatusCode::GONE, "game closed").into_response().await)
            };

            services.put::<Connection>(&uuid, conn_id).await?;
        }
        "$disconnect" => {
            let _ = services.delete::<Connection>(&uuid, Some(conn_id)).await;

            if let Some(game) = services.get::<Game>(&game_id).await {
                game.remove_player(services, uuid).await?;
            };
        }
        "$default" => {
            let response = match WebsocketRequest::try_from(event) {
                Ok(message) => handle_message(services, message, uuid, game_id, conn_id)
                    .await
                    .unwrap_or_else(|e| WebsocketError::ServiceError(e.to_string()).into()),
                Err(error) => error.into(),
            };
            if response != NoResponse {
                services.send(conn_id, &response).await?;
            }
        }
        _ => return Err("unhandled message".into()),
    }

    Ok("handled request".into_response().await)
}

async fn handle_message(
    services: &Services,
    message: WebsocketRequest,
    uuid: PlayerId,
    game_id: GameId,
    conn_id: &str,
) -> Result<WebsocketResponse, Error> {
    // todo fix possible race conditions when pulling from db
    let Some(mut game) = services.get::<Game>(&game_id).await else {
        services.send(conn_id, &CloseGame).await?;
        services.delete_connection(conn_id).await?;
        return Ok(NoResponse)
    };
    
    // join game -> only player showing in game -> join again, item not in db to delete?

    Ok(match message {
        WebsocketRequest::JoinGame => {
            if game.connected_players.contains_key(&uuid) {
                return Ok(WebsocketError::AlreadyInGame.into())
            }
            
            game.add_player(services, uuid, conn_id).await?;
            Success
        }
        WebsocketRequest::DrawCardToHand { stack } => todo!(),
        WebsocketRequest::LeaveGame => {
            if !game.connected_players.contains_key(&uuid) {
                return Ok(WebsocketError::NotInGame.into())
            }
            game.remove_player(services, uuid).await?;
            services.send(conn_id, &Success).await?;
            services.delete_connection(conn_id).await?;
            NoResponse
        }
        WebsocketRequest::FlipCard { stack } => todo!(),
        WebsocketRequest::MoveStack { stack, position } => todo!(),
        WebsocketRequest::Ping => WebsocketResponse::Pong,
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
