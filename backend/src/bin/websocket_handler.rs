use aws_sdk_apigatewaymanagement as apigw_management;
use aws_sdk_dynamodb as dynamodb;
use lambda_http::{
    request::RequestContext, run, service_fn, tracing, Body, Error, IntoResponse, Request,
    RequestExt, Response,
};
use multiplayer_cards::db_utils::{Connection, DynamoDBClient};
use multiplayer_cards::message::WebsocketMessage;
use std::env;
use multiplayer_cards::game::Game;

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
        .to_string();

    match context.route_key.expect("no route key").as_str() {
        "$connect" => {
            if let Some(current_connection) =
                dd_client.get_entry::<Connection>(&uuid).await
            {
                // Try disconnect any current open connection
                // It's okay if this is unsuccessful, the other connection will hang as there is no reference to its connection id
                let _ = apigw_client
                    .delete_connection()
                    .connection_id(current_connection)
                    .send()
                    .await;
            }

            let _ = dd_client
                .put_entry::<Connection>(&uuid, &context.connection_id.unwrap())
                .await;
        }
        "$disconnect" => {
            let _ = dd_client
                .delete_entry::<Connection>(&uuid, Some(&context.connection_id.unwrap()))
                .await;
        }
        "$default" => {
            let message = WebsocketMessage::from_request(event)?;
            // todo fix possible race conditions when pulling from db
            match message {
                WebsocketMessage::CreateGame(data) => {
                    let (game, player) = Game::new(&dd_client, uuid).await?;
                    // todo handle these errors
                    let _ = game.send_state(apigw_client, dd_client).await?;
                    let _ = player.send_state(apigw_client, dd_client).await?;
                }
                
                WebsocketMessage::DrawCardToHand => todo!(),
                WebsocketMessage::JoinGame => todo!(),
                WebsocketMessage::LeaveGame => todo!(),
                WebsocketMessage::Ping => (),
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
