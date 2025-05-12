use std::env;
use anyhow::Error;
use serde::{Serialize};
use strum::IntoStaticStr;
use thiserror::Error;
use crate::game::{GameId};

pub mod db_utils;
pub mod requests;
pub mod game;
pub mod auth;
pub mod utils;

#[derive(Debug, Serialize, Error, IntoStaticStr)]
#[serde(rename_all="kebab-case")]
#[strum(serialize_all="kebab-case")]
pub enum WebsocketError {
    #[error("the game `{0}` does not exist")]
    NonExistentGame(GameId),

    #[error("you are not currently in a game")]
    NotInGame,

    #[error("you cannot do this whilst already in game")]
    AlreadyInGame,

    #[error("only the game owner can perform this action")]
    NoPermission,

    #[error("the stack does not exist")]
    StackNotFound,

    #[error("attempted operation on empty stack")]
    EmptyStack,

    #[error("attempted operation on card in player's hand which doesn't exist")]
    CardNotFound,

    #[error("the player does not exist")]
    PlayerNotFound,

    #[error("{0}")]
    InvalidRequest(&'static str),

    #[error("Internal server error: {0}")]
    ServiceError(String)
}

impl From<Error> for WebsocketError {
    fn from(value: Error) -> Self {
        WebsocketError::ServiceError(value.to_string())
    }
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

    pub async fn delete_connection(&self, conn_id: &str) -> Result<(), Error> {
        self.expect_apigw()
            .delete_connection()
            .connection_id(conn_id)
            .send()
            .await?;
        Ok(())
    }
}