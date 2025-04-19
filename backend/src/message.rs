use lambda_http::ext::request::JsonPayloadError;
use lambda_http::{Request, RequestPayloadExt, Error};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use crate::game::{Card};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub enum WebsocketMessage {
    CreateGame(CreateGameData),
    DrawCardToHand,
    JoinGame,
    LeaveGame,
    Ping,
}

impl WebsocketMessage {
    pub fn from_request(request: Request) -> Result<Self, Error> {
        match request.json::<WebsocketMessage>().map_err(|JsonPayloadError::Parsing(err)| err.to_string())? {
            Some(msg) => Ok(msg),
            None => Err("missing payload".into()),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct CreateGameData;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct GameState {
    // Vector of Option<Card> or None if empty deck
    // Option<Card> is none if card is present but face down
    pub visible_decks: Vec<Option<Option<Card>>>,
}

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct PlayerState {
    pub hand: Vec<Card>,
}