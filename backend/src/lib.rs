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
    AlreadyInGame
}