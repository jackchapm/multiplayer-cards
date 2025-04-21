use std::env;
use serde::{Deserialize, Serialize};
use strum::IntoStaticStr;
use thiserror::Error;
use crate::game::GameId;

pub mod db_utils;
pub mod message;
pub mod game;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

#[derive(Debug, Serialize, Error, IntoStaticStr)]
#[strum(serialize_all="kebab-case")]
pub enum WebsocketError {
    #[error("the game `{0}` does not exist")]
    NonExistentGame(GameId),

    #[error("you are not currently in a game")]
    NotInGame,

    #[error("you cannot do this whilst already in game")]
    AlreadyInGame,

    #[error("{0}")]
    InvalidRequest(&'static str),

    #[error("Internal server error: {0}")]
    // todo stack trace should not be sent to client
    ServiceError(String)
}

pub struct Services {
    pub db: aws_sdk_dynamodb::Client,
    pub apigw: Option<aws_sdk_apigatewaymanagement::Client>,
    pub table_name: String,
}

impl Services {
    pub async fn create() -> Self {
        let endpoint = env::var("WEBSOCKET_ENDPOINT").ok();
        let table_name = env::var("TABLE_NAME").expect("table name not set");
        let shared_conf = &aws_config::load_from_env().await;

        let apigw = endpoint.map(|endpoint| {
            let config = aws_sdk_apigatewaymanagement::config::Builder::from(shared_conf)
                .endpoint_url(endpoint)
                .build();

            aws_sdk_apigatewaymanagement::Client::from_conf(config)
        });

        Self {
            db: aws_sdk_dynamodb::Client::new(shared_conf),
            apigw,
            table_name
        }
    }

    pub fn expect_apigw(&self) -> &aws_sdk_apigatewaymanagement::Client {
        self.apigw.as_ref().expect("no API Gateway client initialised")
    }
}