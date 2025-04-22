use crate::game::{Card, GameId, PlayerId, StackId, StackState};
use crate::{Services, WebsocketError};
use anyhow::Error;
use aws_sdk_apigatewaymanagement::primitives::Blob;
use lambda_http::{Request, RequestPayloadExt};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(tag = "action")]
#[serde(rename_all = "kebab-case")]
#[serde(rename_all_fields = "camelCase")]
pub enum WebsocketRequest {
    JoinGame,
    TakeCard { stack: StackId },
    PutCard { hand_index: usize, position: (i8, i8), face_down: bool },
    FlipCard { stack: StackId },
    FlipStack { stack: StackId },
    MoveCard { stack: StackId, position: (i8, i8) },
    MoveStack { stack: StackId, position: (i8, i8) },
    Shuffle { stack: StackId },
    Deal { stack: StackId },
    GivePlayer { hand_index: usize, trade_to: PlayerId},
    Reset,
    LeaveGame,
    Ping,
}

// todo move into an enum
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateGameRequest {
    pub name: String,
    pub deck_type: DeckType,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct JoinGameRequest {
    pub game_id: GameId,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct JoinGameResponse {
    pub game_id: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "kebab-case")]
#[serde(rename_all_fields = "camelCase")]
pub enum DeckType {
    Standard,
    Custom(Vec<Vec<Card>>),
}

impl TryFrom<Request> for WebsocketRequest {
    type Error = WebsocketError;

    fn try_from(value: Request) -> Result<Self, Self::Error> {
        let Ok(message) = value.json() else {
            return Err(WebsocketError::InvalidRequest("error parsing json"));
        };

        match message {
            Some(msg) => Ok(msg),
            None => Err(WebsocketError::InvalidRequest("no payload sent")),
        }
    }
}

// todo only send update not whole state
#[derive(Debug, Serialize, PartialEq, JsonSchema)]
#[serde(tag = "type")]
#[serde(rename_all = "kebab-case")]
#[serde(rename_all_fields = "camelCase")]
pub enum WebsocketResponse {
    GameState {
        game_id: GameId,
        owner: PlayerId,
        connected_players: Vec<PlayerId>,
        stacks: Vec<StackState>,
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
    NoResponse,
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
    pub async fn send_batch<T, I>(&self, connections: I, data: &T) -> Result<(), Error>
    where
        T: Serialize,
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        let state_blob = Blob::new(serde_json::to_string(data)?);
        for conn_id in connections {
            let _ = self
                .expect_apigw()
                .post_to_connection()
                .connection_id(conn_id.as_ref())
                .data(state_blob.clone())
                .send()
                .await;
        }
        Ok(())
    }
}
