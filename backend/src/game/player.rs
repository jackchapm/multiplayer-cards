use std::marker::PhantomData;
use anyhow::Error;
use serde::{Deserialize, Serialize};
use crate::db_utils::{Connection, Key};
use crate::game::{Card, Game, GameId};
use crate::message::WebsocketResponse;
use crate::Services;

pub type PlayerId = String;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Player {
    pub player_id: PlayerId,
    pub game_id: GameId,
    pub hand: Vec<Card>,
    #[serde(skip)]
    _private: PhantomData<()>,
}

impl Key for Player {
    type Key = PlayerId;
    type Value = Self;

    fn prefix() -> &'static str {
        "game:player"
    }
}

impl Player {
    pub async fn new(
        services: &Services,
        player_id: PlayerId,
        game_id: GameId,
    ) -> Result<Self, Error> {
        let player = Player {
            player_id,
            game_id,
            hand: vec![],
            _private: PhantomData
        };

        let _ = services.put::<Player>(&player.player_id, &player).await?;
        Ok(player)
    }

    fn state(&self) -> WebsocketResponse {
        // todo better solution than clone
        WebsocketResponse::PlayerState {
            game_id: self.game_id.clone(),
            hand: self.hand.clone(),
        }
    }

    pub async fn get_game(&self, services: &Services) -> Game {
        services.get::<Game>(&self.game_id).await.expect("player object in database after game destroyed")
    }

    /// Sends the current player state to the player
    pub async fn send_state(&self, services: &Services, conn_id: Option<&str>) -> Result<(), Error> {
        // todo check player game is this current one
        // todo handle this error gracefully
        let conn_id= if let Some(conn_id) = conn_id {
            conn_id
        } else {
            &*services.get::<Connection>(&self.player_id).await.expect("connection not found")
        };

        services.send(conn_id, &self.state()).await
    }
}
