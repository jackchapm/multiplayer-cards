use crate::game::{Card, DeckId, GameId, PlayerId};
use crate::{Services, WebsocketError};
use anyhow::{Error};
use aws_sdk_apigatewaymanagement::primitives::Blob;
use lambda_http::{Request, RequestPayloadExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

impl TryFrom<Request> for WebsocketRequest {
    type Error = WebsocketError;

    fn try_from(value: Request) -> Result<Self, Self::Error> {
        let Ok(message) = value.json() else {
            return Err(WebsocketError::InvalidRequest("error parsing json"))
        };

        match message {
            Some(msg) => Ok(msg),
            None => Err(WebsocketError::InvalidRequest("no payload sent")),
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
        /// (index, value)
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

impl Services {
    pub async fn send<T: Serialize>(&self, conn_id: &str, data: &T) -> Result<(), Error> {
        let state_blob = Blob::new(serde_json::to_string(data)?);
        let _ = self
            .expect_apigw()
            .post_to_connection()
            .connection_id(conn_id)
            .data(state_blob.clone())
            .send()
            .await?;
        Ok(())
    }

    // todo change to unowned string
    pub async fn send_batch<T: Serialize, I: IntoIterator<Item = String>>(
        &self,
        connections: I,
        data: &T,
    ) -> Result<(), Error> {
        let state_blob = Blob::new(serde_json::to_string(data)?);
        for conn_id in connections {
            let _ = self
                .expect_apigw()
                .post_to_connection()
                .connection_id(conn_id)
                .data(state_blob.clone())
                .send()
                .await?;
        }
        Ok(())
    }
}
