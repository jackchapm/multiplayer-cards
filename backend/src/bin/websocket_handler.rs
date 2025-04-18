use std::env;
use anyhow::anyhow;
use aws_sdk_apigatewaymanagement as apigw_management;
use aws_sdk_dynamodb as dynamodb;
use aws_sdk_apigatewaymanagement::primitives::Blob;
use aws_sdk_dynamodb::types::AttributeValue;
use lambda_http::{request::RequestContext, run, service_fn, tracing, Body, Error, IntoResponse, Request, RequestExt, RequestPayloadExt, Response};
use lambda_http::ext::request::JsonPayloadError;
use lambda_http::tracing::info;
use serde::{Deserialize, Serialize};
use multiplayer_cards::db_utils::{Connection, DynamoDBClient, Key};

#[derive(Debug, Serialize, Deserialize)]
struct WebsocketMessage {
    action: String,
    message: String,
}

async fn websocket_handler(event: Request, dd_client: &dynamodb::Client, apigw_client: &apigw_management::Client) -> Result<Response<Body>, Error> {
    let context = match event.request_context() {
        RequestContext::WebSocket(context) => context,
        _ => return Err(anyhow!("function only handles websockets").into())
    };

    let table_name = env::var("TABLE_NAME").expect("missing table name");

    // Can safely call unwrap as this header has been authorized by lambda authorizer
    let uuid = context.authorizer.fields.get("uuid").unwrap().to_string();

    match context.route_key.expect("no route key").as_str() {
        "$connect" => {
            let _ = dd_client.put_entry::<Connection>(&table_name, context.connection_id.unwrap().as_str(), uuid).await;
            Ok("".into_response().await)
        },
        "$disconnect" => {
            let _ = dd_client.delete_entry::<Connection>(&table_name, context.connection_id.unwrap().as_str()).await;
            Ok("".into_response().await)
        },
        "$default" => {
            let msg = match event.json::<WebsocketMessage>() {
                Ok(Some(msg)) => msg,
                Ok(None) => return Err(anyhow!("missing payload").into()),
                Err(JsonPayloadError::Parsing(err)) => return Err(anyhow!(
                    if err.is_data() { "malformed payload schema"}
                    else if err.is_syntax() { "malformed json"}
                    else { "failed to parse"}
                ).into())
            }.message;

            let connections = dd_client.scan().table_name(table_name).filter_expression("begins_with(pk, :prefix)").expression_attribute_values(":prefix", AttributeValue::S(Connection::prefix().to_string())).send().await?;
            info!("{connections:?}");
            for conn_id in connections.items.unwrap().iter().map(|v| v.get("value").unwrap().as_s().unwrap()) {
                let _ = apigw_client.post_to_connection().connection_id(conn_id).data(Blob::new(format!("{uuid} said: {msg}"))).send().await;
            }
            Ok("".into_response().await)
        },
        _ => Err(anyhow!("unhandled message").into())
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();
    let endpoint_url = env::var("WEBSOCKET_ENDPOINT").expect("websocket endpoint not set");
    let shared_conf = &aws_config::load_from_env().await;

    let apigw_config = apigw_management::config::Builder::from(shared_conf).endpoint_url(endpoint_url).build();
    let apigw_client = apigw_management::Client::from_conf(apigw_config);

    let dd_client = dynamodb::Client::new(shared_conf);

    run(service_fn(async |request| {
        websocket_handler(request, &dd_client, &apigw_client).await
    })).await
}
