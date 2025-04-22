use std::collections::HashMap;
use crate::db_utils::{Key};
use crate::requests::{DeckType, WebsocketResponse};
use crate::requests::WebsocketResponse::GameState;
use crate::Services;
use anyhow::{anyhow, Error};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

mod deck;
mod player;

pub use deck::*;
pub use player::*;

pub type GameId = String;

#[derive(Debug, Serialize, Deserialize)]
pub struct Game {
    pub id: GameId,
    pub created_at: u64, //todo move to separate game data object
    pub owner: PlayerId, // todo move to separate game data object
    pub authorized_players: Vec<PlayerId>, // todo move to separate game data object
    
    // todo This is rarely queried, convert into a vec that stores both (tuple?)
    pub connected_players: HashMap<PlayerId, String>,
    pub stacks: Vec<Stack>,
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
        services: &Services,
        player_id: PlayerId,
        deck_type: DeckType,
    ) -> Result<Self, Error> {
        let game_id = Uuid::new_v4().to_string();

        let stacks = Stack::from(deck_type);
        
        let new_game = Self {
            id: game_id,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            owner: player_id.clone(),
            authorized_players: vec![player_id],
            connected_players: HashMap::new(),
            stacks,
            _private: PhantomData,
        };

        services.put::<Game>(&new_game.id, &new_game).await?;
        Ok(new_game)
    }

    pub async fn add_player(
        &mut self,
        services: &Services,
        player_id: PlayerId,
        conn_id: &str,
    ) -> Result<Player, Error> {
        let player = match services.get::<Player>(&player_id).await {
            Some(player) => player,
            None => Player::new(services, player_id.clone(), self.id.clone()).await?,
        };
        self.connected_players.insert(player_id.clone(), conn_id.to_string());
        services.put::<Game>(&self.id, &self).await?;
        self.send_state_all(services, &self.state()).await?;
        player.send_state(services, Some(conn_id)).await?;
        Ok(player)
    }

    pub async fn add_authorized_player(
        &mut self,
        services: &Services,
        player_id: PlayerId
    ) -> Result<(), Error> {
        self.authorized_players.push(player_id);
        services.put::<Game>(&self.id, &self).await?;
        Ok(())
    }

    pub async fn remove_player(
        mut self,
        services: &Services,
        player_id: PlayerId,
    ) -> Result<(), Error> {
        if let None = self.connected_players.remove(&player_id) {
            return Err(anyhow!("player not in this game"))
        }
        // Keep player state in database incase they join back
        
        if self.connected_players.is_empty() {
            self.destroy(services).await?;
            return Ok(())
        }

        if self.owner == player_id {
            // Game owner has left, assign new owner
            // can safely call unwrap as we know the list is not empty
            self.owner = self.connected_players.values().next().unwrap().clone();
        }
        // todo only send update, not full state
        services.put::<Game>(&self.id, &self).await?;
        self.send_state_all(services, &self.state()).await?;
        Ok(())
    }

    pub async fn destroy(mut self, services: &Services) -> Result<(), Error> {
        services.delete::<Game>(&self.id, None).await?;
        for player in &self.authorized_players {
            services.delete::<Player>(&player, None).await?;
        }
        self.send_state_all(services, &WebsocketResponse::CloseGame).await?;
        for (_, conn_id) in self.connected_players.into_iter() {
            let _ = services.delete_connection(&conn_id).await;
        }
        Ok(())
    }

    /// Generates the game state to be sent to players
    fn state(&self) -> WebsocketResponse {
        GameState {
            owner: self.owner.clone(),
            game_id: self.id.clone(),
            // todo check performance on this call
            connected_players: self.connected_players.keys().cloned().collect(),
            stacks: self.stacks.iter().map(Stack::state).collect()
        }
    }

    pub async fn send_state(&self, services: &Services, conn_id: &str) -> Result<(), Error> {
        services.send(conn_id, &self.state()).await
    }

    /// Send a websocket response to all players connected to the game
    async fn send_state_all(&mut self, services: &Services, data: &WebsocketResponse) -> Result<(), Error> {
        services.send_batch(self.connected_players.values(), data).await
    }
}