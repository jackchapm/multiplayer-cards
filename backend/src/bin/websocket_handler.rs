use aws_sdk_apigatewaymanagement as apigw_management;
use aws_sdk_dynamodb as dynamodb;
use lambda_http::{
    request::RequestContext, run, service_fn, tracing, Body, Error, IntoResponse, Request,
    RequestExt, Response,
};
use multiplayer_cards::db_utils::{Connection, DynamoDBClient};
use multiplayer_cards::message::{WebsocketRequest, WebsocketResponse};
use std::env;
use multiplayer_cards::game::{Game, Player};
use multiplayer_cards::WebsocketError;

async fn websocket_handler(
    event: Request,
    dd_client: &DynamoDBClient,
    apigw_client: &apigw_management::Client,
) -> Result<Response<Body>, Error> {
    let RequestContext::WebSocket(context) = event.request_context() else {
        return Err("expected websocket".into());
    };

    // Can safely call unwrap as this header has been authorized by lambda authorizer
    let uuid = context
        .authorizer
        .fields
        .get("uuid")
        .unwrap()
        .as_str()
        .unwrap()
        .to_string();

    let conn_id = &context.connection_id.unwrap();

    match context.route_key.expect("no route key").as_str() {
        "$connect" => {
            if let Some(old_connection) =
                dd_client.get_entry::<Connection>(&uuid).await
            {
                // Try disconnect any current open connection
                // It's okay if this is unsuccessful, the other connection will hang as there is no reference to its connection id
                let _ = apigw_client
                    .delete_connection()
                    .connection_id(old_connection)
                    .send()
                    .await;
            }

            let _ = dd_client
                .put_entry::<Connection>(&uuid, conn_id)
                .await;
        }
        "$disconnect" => {
            let _ = dd_client
                .delete_entry::<Connection>(&uuid, Some(conn_id))
                .await;
        }
        "$default" => {
            let message = WebsocketRequest::from_request(event)?;
            // todo fix possible race conditions when pulling from db
            match message {
                WebsocketRequest::CreateGame { name, deck_options}=> {
                    if let Some(_) = dd_client.get_entry::<Player>(&uuid).await {
                        let response: WebsocketResponse = WebsocketError::AlreadyInGame.into();
                        response.send(apigw_client, conn_id).await?
                    } else {
                        let _ = Game::new(apigw_client, dd_client, uuid, conn_id, &deck_options).await?;
                    }
                }
                WebsocketRequest::DrawCardToHand { deck } => todo!(),
                WebsocketRequest::JoinGame { game_id } => {
                    let game = dd_client.get_entry::<Game>(&game_id).await;
                    if let Some(mut game) = game {
                        game.add_player(apigw_client, dd_client, uuid, conn_id).await?;
                    } else {
                        let response: WebsocketResponse = WebsocketError::NonExistentGame(game_id).into();
                        response.send(apigw_client, conn_id).await?
                    }
                },
                WebsocketRequest::LeaveGame => {
                    if let Some(player) = dd_client.get_entry::<Player>(&uuid).await {
                        let mut game = player.get_game(dd_client).await;
                        game.remove_player(apigw_client, dd_client, uuid, false).await?;
                    } else {
                        let response: WebsocketResponse = WebsocketError::NotInGame.into();
                        response.send(apigw_client, conn_id).await?
                    }
                },
                WebsocketRequest::Ping => {
                    if let Some(player) = dd_client.get_entry::<Player>(&uuid).await {
                        player.send_state(apigw_client, dd_client, Some(conn_id)).await?;
                        let game = player.get_game(dd_client).await;
                        game.send_state(apigw_client, conn_id).await?;
                        for deck in &game.decks {
                            game.send_deck_state(apigw_client, dd_client, deck, conn_id).await?;
                        }
                    } else {
                        WebsocketResponse::Pong.send(apigw_client, conn_id).await?
                    }
                },
            }
        }
        _ => return Err("unhandled message".into()),
    }

    Ok("handled request".into_response().await)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();
    let endpoint_url = env::var("WEBSOCKET_ENDPOINT").expect("websocket endpoint not set");
    let table_name = env::var("TABLE_NAME").expect("table name not set");
    let shared_conf = &aws_config::load_from_env().await;

    // todo create struct to pass around config and apigw client together
    let apigw_config = apigw_management::config::Builder::from(shared_conf)
        .endpoint_url(endpoint_url)
        .build();
    let apigw_client = apigw_management::Client::from_conf(apigw_config);

    let dd_client = DynamoDBClient::new(dynamodb::Client::new(shared_conf), table_name);

    run(service_fn(async |request| {
        websocket_handler(request, &dd_client, &apigw_client).await
    }))
    .await
}
