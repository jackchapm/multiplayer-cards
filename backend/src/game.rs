use crate::db_utils::{Connection, DynamoDBClient, Key};
use anyhow::{Error};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::time::{SystemTime, UNIX_EPOCH};
use aws_sdk_apigatewaymanagement::Client;
use aws_sdk_apigatewaymanagement::primitives::Blob;
use uuid::Uuid;

mod deck;
use crate::message::{GameState, PlayerState};
pub use deck::*;

pub type GameId = String;
pub type PlayerId = String;

#[derive(Debug, Serialize, Deserialize)]
pub struct Game {
    pub id: GameId,
    pub created_at: u64,
    pub owner: PlayerId,
    pub decks: Vec<Deck>,
    pub players: Vec<PlayerId>,
    _private: PhantomData<()>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Player {
    player_id: PlayerId,
    game_id: GameId,
    hand: Vec<Card>,
}

impl Key for Game {
    type Key = GameId;
    type Value = Self;

    fn prefix() -> &'static str {
        "game:game"
    }
}

impl Key for Player {
    type Key = PlayerId;
    type Value = Self;

    fn prefix() -> &'static str {
        "game:player"
    }
}

impl Game {
    pub async fn new(ddb: &DynamoDBClient, player_id: PlayerId) -> Result<(Self, Player), Error> {
        let new_game = Self {
            id: Uuid::new_v4().to_string(),
            created_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            owner: player_id.clone(),
            decks: vec![Deck::shuffled52(Default::default())],
            players: vec![player_id.clone()],
            _private: PhantomData,
        };

        let _ = ddb.put_entry::<Game>(&new_game.id, &new_game).await?;
        let player = Player::new(ddb, player_id, new_game.id.clone()).await?;
        Ok((new_game, player))
    }

    pub async fn add_player(
        &mut self,
        ddb: &DynamoDBClient,
        player_id: PlayerId,
    ) -> Result<Player, Error> {
        let player = Player::new(ddb, player_id.clone(), self.id.clone()).await?;
        self.players.push(player_id.clone());
        let _ = ddb.put_entry::<Game>(&self.id, &self).await?;
        Ok(player)
    }

    pub async fn destroy(&mut self, ddb: &DynamoDBClient) -> Result<(), Error> {
        let _ = ddb.delete_entry::<Game>(&self.id, None).await?;
        for player in &self.players {
            // todo add sanity check that players current game is the one we are removing from
            // todo need to write condition expression
            let _ = ddb.delete_entry::<Player>(&player, None).await?;
        }
        Ok(())
    }

    // Generates the game state to be sent to players
    pub async fn state(&self) -> GameState {
        GameState {
            visible_decks: self
                .decks
                .iter()
                .map(|d| {
                    d.cards
                        .iter()
                        .peekable()
                        .peek()
                        .map(|&&c| if d.face_up { Some(c) } else { None })
                })
                .collect(),
        }
    }

    // Sends the current game state to all connected players
    pub async fn send_state(&self, apigw_client: &Client, ddb: &DynamoDBClient) -> Result<(), Error> {
        let state = self.state().await;
        let state_blob = Blob::new(serde_json::to_string(&state)?);
        let connections = self.players
            .iter()
            .map(async |uuid| ddb.get_entry::<Connection>(uuid).await);

        for conn_id in connections {
            // todo standardise the way state is sent (all websocket messages need to look the same)
            let _ = apigw_client.post_to_connection().connection_id(conn_id.await.unwrap()).data(state_blob.clone()).send().await;
        }
        Ok(())
    }
}

impl Player {
    pub async fn new(
        ddb: &DynamoDBClient,
        player_id: PlayerId,
        game_id: GameId,
    ) -> Result<Self, Error> {
        let player = Player {
            player_id,
            game_id,
            hand: vec![],
        };

        let _ = ddb.put_entry::<Player>(&player.player_id, &player).await?;
        Ok(player)
    }

    pub async fn state(&self) -> PlayerState {
        // todo better solution than clone
        PlayerState {
            hand: self.hand.clone(),
        }
    }

    // Sends the current player state to the player
    pub async fn send_state(&self, apigw_client: &Client, ddb: &DynamoDBClient) -> Result<(), Error> {
        let state = self.state().await;
        let state_blob = Blob::new(serde_json::to_string(&state)?);
        // todo check player game is this current one
        // todo handle this error gracefully
        let conn_id = ddb.get_entry::<Connection>(&self.player_id).await.expect("connection not found");
        let _ = apigw_client.post_to_connection().connection_id(conn_id).data(state_blob.clone()).send().await;
        Ok(())
    }
}
