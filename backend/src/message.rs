use anyhow::{anyhow, Error};
use aws_sdk_apigatewaymanagement::Client;
use aws_sdk_apigatewaymanagement::primitives::Blob;
use crate::game::{Card, PlayerId, DeckId, GameId};
use lambda_http::ext::request::JsonPayloadError;
use lambda_http::{Request, RequestPayloadExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::WebsocketError;

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(tag = "action")]
#[serde(rename_all = "kebab-case")]
#[serde(rename_all_fields = "camelCase")]
pub enum WebsocketRequest {
    CreateGame {
        name: String,
        #[serde(flatten)]
        deck_options: DeckOptions,
    },
    DrawCardToHand {
        deck: DeckId,
    },
    JoinGame {
        game_id: GameId,
    },
    LeaveGame,
    Ping,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeckOptions {
    pub face_down: bool,
    pub deck_type: DeckType,
    pub capacity: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(rename_all_fields = "camelCase")]
pub enum DeckType {
    Standard,
    Custom(Vec<Card>),
}

impl WebsocketRequest {
    pub fn from_request(request: Request) -> Result<Self, Error> {
        match request.json().map_err(|JsonPayloadError::Parsing(_)| anyhow!("error parsing json"))? {
            Some(msg) => Ok(msg),
            None => Err(anyhow!("missing payload")),
        }
    }
}

// todo only send update not whole state
#[derive(Debug, Serialize, JsonSchema)]
#[serde(tag = "type")]
#[serde(rename_all = "kebab-case")]
#[serde(rename_all_fields = "camelCase")]
pub enum WebsocketResponse {
    GameState {
        game_id: GameId,
        owner: PlayerId,
        connected_players: Vec<PlayerId>,
        // Vector of Option<Card> or None if empty deck
        visible_decks: Vec<DeckId>,
    },
    DeckState {
        deck_id: DeckId,
        visible_cards: Vec<(usize, Card)>,
    },
    PlayerState {
        game_id: GameId,
        hand: Vec<Card>,
    },
    Error {
        error: &'static str,
        message: String,
    },
    CloseGame,
    Success,
    Pong,
}

impl From<WebsocketError> for WebsocketResponse {
    fn from(value: WebsocketError) -> Self {
        WebsocketResponse::Error {
            message: value.to_string(),
            error: value.into(),
        }
    }
}

impl WebsocketResponse {
    pub async fn send(&self, apigw_client: &Client, conn_id: &str) -> Result<(), Error> {
        let state_blob =  Blob::new(serde_json::to_string(self)?);
        let _ = apigw_client.post_to_connection().connection_id(conn_id).data(state_blob.clone()).send().await?;
        Ok(())
    }

    // todo change to unowned string
    pub async fn send_batch<I: IntoIterator<Item = String>>(&self, apigw_client: &Client, connections: I) -> Result<(), Error> {
        let state_blob =  Blob::new(serde_json::to_string(self)?);
        for conn_id in connections {
            let _ = apigw_client.post_to_connection().connection_id(conn_id).data(state_blob.clone()).send().await?;
        }
        Ok(())
    }
}