use crate::db_utils::{Connection, DynamoDBClient, Key};
use crate::message::{DeckOptions, WebsocketResponse};
use crate::message::WebsocketResponse::GameState;
use anyhow::{anyhow, Error};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::time::{SystemTime, UNIX_EPOCH};
use aws_sdk_apigatewaymanagement::Client;
use futures::future::join_all;
use uuid::Uuid;

mod deck;
mod player;

pub use deck::*;
pub use player::*;

pub type GameId = String;

#[derive(Debug, Serialize, Deserialize)]
pub struct Game {
    pub id: GameId,
    pub created_at: u64,
    pub owner: PlayerId,
    // todo with current structure, could possibly be just a single deck id?
    pub decks: Vec<DeckId>,
    // This would ideally be a set, but as this struct is not kept in memory
    // it is more expensive to deserialise a set every time
    pub players: Vec<PlayerId>,
    #[serde(skip)]
    _private: PhantomData<()>,
}

impl Key for Game {
    type Key = GameId;
    type Value = Self;

    fn prefix() -> &'static str {
        "game:game"
    }
}

// todo periodically scan for stale games and purge from db (TTL?)
impl Game {
    pub async fn new(
        apigw_client: &Client,
        ddb: &DynamoDBClient,
        player_id: PlayerId,
        conn_id: &str,
        deck_options: &DeckOptions
    ) -> Result<Self, Error> {
        let game_id = Uuid::new_v4().to_string();

        let deck = Deck::from_options(deck_options, &game_id);
        ddb.put_entry::<Deck>(&deck.id, &deck).await?;

        let mut new_game = Self {
            id: game_id,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            owner: player_id.clone(),
            decks: vec![deck.id.clone()],
            players: vec![],
            _private: PhantomData,
        };

        // No need to add the game to the database yet, as it will be done inside the add_player call
        new_game.add_player(apigw_client, ddb, player_id, conn_id).await?;
        Ok(new_game)
    }

    pub async fn add_player(
        &mut self,
        apigw_client: &Client,
        ddb: &DynamoDBClient,
        player_id: PlayerId,
        conn_id: &str,
    ) -> Result<Player, Error> {
        let player = Player::new(ddb, player_id.clone(), self.id.clone()).await?;
        self.players.push(player_id.clone());
        ddb.put_entry::<Game>(&self.id, &self).await?;
        self.send_state_all(apigw_client, ddb, &self.state()).await?;
        for deck_id in &self.decks {
            self.send_deck_state(apigw_client, ddb, &deck_id, conn_id).await?
        }
        player.send_state(apigw_client, ddb, Some(conn_id)).await?;
        Ok(player)
    }

    pub async fn remove_player(
        &mut self,
        apigw_client: &Client,
        ddb: &DynamoDBClient,
        player_id: PlayerId,
        save_data: bool,
    ) -> Result<(), Error> {
        if let Some(index) = self.players.iter().position(|p| p == &player_id) {
            self.players.swap_remove(index);
        } else {
            return Err(anyhow!("player not in this game"))
        }

        if !save_data {
            // todo add sanity check that players current game is the one we are removing from
            // need to write condition expression
            let _ = ddb.delete_entry::<Player>(&player_id, None).await?;
        }

        if self.players.is_empty() {
            self.destroy(apigw_client, ddb).await?;
            return Ok(())
        }

        if self.owner == player_id {
            // Game owner has left, assign new owner
            // can safely call unwrap as we know the list is not empty
            self.owner = self.players.first().unwrap().clone();
        }
        // todo only send update, not full state
        self.send_state_all(apigw_client, ddb, &self.state()).await?;
        Ok(())
    }

    pub async fn destroy(&mut self, apigw_client: &Client, ddb: &DynamoDBClient) -> Result<(), Error> {
        ddb.delete_entry::<Game>(&self.id, None).await?;
        for player in &self.players {
            ddb.delete_entry::<Player>(&player, None).await?;
        }
        for deck in &self.decks {
            let _ = ddb.delete_entry::<Deck>(&deck, None).await?;
        }
        self.send_state_all(apigw_client, ddb, &WebsocketResponse::CloseGame).await?;
        Ok(())
    }

    /// Generates the game state to be sent to players
    fn state(&self) -> WebsocketResponse {
        GameState {
            owner: self.owner.clone(),
            game_id: self.id.clone(),
            connected_players: self.players.iter().cloned().collect(),
            visible_decks: self.decks.clone(),
        }
    }

    pub async fn send_state(&self, apigw_client: &Client, conn_id: &str) -> Result<(), Error> {
        self.state().send(apigw_client, conn_id).await
    }

    pub async fn send_deck_state(&self, apigw_client: &Client, ddb: &DynamoDBClient, deck: &DeckId, conn_id: &str) -> Result<(), Error> {
        let deck_state = ddb.get_entry::<Deck>(&deck).await.expect("deck refereneced in game not in database").state();
        deck_state.send(apigw_client, conn_id).await?;
        Ok(())
    }

    /// Send a websocket response to all players connected to the game
    async fn send_state_all(&mut self, apigw_client: &Client, ddb: &DynamoDBClient, data: &WebsocketResponse) -> Result<(), Error> {
        let players = self.players.clone();
        let connections = join_all(players
            .iter()
            .map(async |uuid| {
                (uuid, ddb.get_entry::<Connection>(uuid).await)
            }));

        let connections: Vec<_> = connections.await.into_iter().filter_map(|(_uuid, c)| if c.is_none() {
            // todo check how long connection has been stale and possibly clean up?
            None
        } else {
            Some(c.unwrap())
        }).collect();

        data.send_batch(&apigw_client, connections).await
    }
}