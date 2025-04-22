use std::collections::HashMap;
use crate::db_utils::{Key};
use crate::requests::{DeckType, WebsocketResponse};
use crate::requests::WebsocketResponse::GameState;
use crate::{Services, WebsocketError};
use anyhow::{anyhow, Error};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;
use std::time::{SystemTime, UNIX_EPOCH};
use rand::rng;
use rand::seq::SliceRandom;
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
    pub deck_type: DeckType,
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

        let stacks = Stack::from(deck_type.clone());
        
        let new_game = Self {
            id: game_id,
            created_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            owner: player_id.clone(),
            authorized_players: vec![player_id],
            connected_players: HashMap::new(),
            deck_type,
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
        player.send_state(services, conn_id).await?;
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
            self.owner = self.connected_players.keys().next().unwrap().clone();
        }
        // todo only send update, not full state
        services.put::<Game>(&self.id, &self).await?;
        self.send_state_all(services, &self.state()).await?;
        Ok(())
    }

    pub async fn destroy(self, services: &Services) -> Result<(), Error> {
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

    fn get_stack(&mut self, stack_id: StackId) -> Result<&mut Stack, WebsocketError> {
        self.stacks
            .iter_mut()
            // todo check cost of deref
            .find(|s| s.id == *stack_id)
            .ok_or_else(|| WebsocketError::StackNotFound)
    }

    fn pop_from_stack(&mut self, stack_id: StackId) -> Result<Card, WebsocketError> {
        let stack_index = self.stacks.iter().position(|s| s.id == stack_id)
            .ok_or(WebsocketError::StackNotFound)?;

        let stack = self.stacks.get_mut(stack_index).unwrap();
        let card = stack.cards.pop().ok_or(WebsocketError::EmptyStack)?;

        if stack.cards.is_empty() {
            self.stacks.swap_remove(stack_index);
        }
        Ok(card)
    }

    async fn get_player(&self, services: &Services, player_id: &PlayerId) -> Result<Player, WebsocketError> {
        services.get::<Player>(player_id)
            .await
            .ok_or_else(|| WebsocketError::PlayerNotFound)
    }

    async fn save_and_send(&self, services: &Services) -> Result<(), Error> {
        self.send_state_all(services, &self.state()).await?;
        services.put::<Game>(&self.id, self).await.map(|_| ())
    }

    fn stack_at_position(&mut self, position: (i8, i8), create_if_none: bool) -> Option<&mut Stack> {
        if let Some(index) = self.stacks.iter().position(|s| s.position == position) {
            return Some(&mut self.stacks[index]);
        }

        if !create_if_none {
            return None;
        }

        self.stacks.push(Stack {
            id: Uuid::new_v4().to_string(),
            cards: Vec::new(),
            position,
        });

        self.stacks.last_mut()
    }

    pub async fn flip_card(&mut self, services: &Services, stack_id: StackId) -> Result<(), WebsocketError> {
        let stack = self.get_stack(stack_id)?;
        if stack.cards.is_empty() {
            // todo handle deleting this stack
            return Err(WebsocketError::EmptyStack)
        }
        stack.cards.last_mut().unwrap().flip();
        self.save_and_send(services).await?;
        Ok(())
    }

    pub async fn flip_stack(&mut self, services: &Services, stack_id: StackId) -> Result<(), WebsocketError> {
        let stack = self.get_stack(stack_id)?;
        stack.cards.reverse();
        for card in &mut stack.cards {
            card.flip();
        }
        self.save_and_send(services).await?;
        Ok(())
    }

    pub async fn shuffle_stack(&mut self, services: &Services, stack_id: StackId) -> Result<(), WebsocketError> {
        let stack = self.get_stack(stack_id)?;
        stack.cards.shuffle(&mut rng());
        self.save_and_send(services).await?;
        Ok(())
    }

    pub async fn move_stack(&mut self, services: &Services, stack_id: StackId, position: (i8, i8)) -> Result<(), WebsocketError> {
        let stack_index = self.stacks.iter().position(|s| s.id == stack_id)
            .ok_or(WebsocketError::StackNotFound)?;

        let mut stack = self.stacks.swap_remove(stack_index);
        if let Some(target_stack) = self.stack_at_position(position, false) {
            target_stack.cards.extend(stack.cards.drain(..));
        } else {
            stack.position = position;
            self.stacks.push(stack);
        }

        self.save_and_send(services).await?;
        Ok(())
    }

    pub async fn move_card(&mut self, services: &Services, stack_id: StackId, position: (i8, i8)) -> Result<(), WebsocketError> {
        let card = self.pop_from_stack(stack_id)?;
        let target_stack = self.stack_at_position(position, true).unwrap();
        target_stack.cards.push(card);

        self.save_and_send(services).await?;
        Ok(())
    }

    pub async fn take_card(&mut self, services: &Services, stack_id: StackId, player_id: &PlayerId, conn_id: &str) -> Result<(), WebsocketError> {
        let mut player = self.get_player(services, player_id).await?;
        let card = self.pop_from_stack(stack_id)?;

        player.hand.push(card);
        self.save_and_send(services).await?;
        player.send_state(services, conn_id).await?;
        services.put::<Player>(&player.player_id, &player).await?;
        Ok(())
    }

    // todo determine whether face up or down
    pub async fn put_card(
        &mut self,
        services: &Services,
        player_id: &PlayerId,
        hand_index: usize,
        position: (i8, i8),
        face_down: bool,
        conn_id: &str
    ) -> Result<(), WebsocketError> {
        let mut player = self.get_player(services, player_id).await?;
        if hand_index >= player.hand.len() {
            return Err(WebsocketError::CardNotFound)
        }

        let target_stack = self.stack_at_position(position, true).unwrap();
        let mut card = player.hand.swap_remove(hand_index);
        if face_down != card.is_face_down() {
            card.flip()
        } 
        target_stack.cards.push(card);
        player.send_state(services, conn_id).await?;
        services.put::<Player>(&player.player_id, &player).await?;
        self.save_and_send(services).await?;
        Ok(())
    }

    pub async fn reset(&mut self, services: &Services) -> Result<(), WebsocketError> {
        for player_id in &self.authorized_players {
            if let Some(conn_id) = self.connected_players.get(player_id) {
                let mut player = self.get_player(services, &player_id).await?;
                player.hand = Vec::new();
                player.send_state(services, conn_id).await?;
                services.put::<Player>(&player_id, &player).await?;
            } else {
                services.delete::<Player>(&player_id, None).await?; 
            }
        }
        self.stacks = Stack::from(self.deck_type.clone());
        self.save_and_send(services).await?;

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
    async fn send_state_all(&self, services: &Services, data: &WebsocketResponse) -> Result<(), Error> {
        services.send_batch(self.connected_players.values(), data).await
    }
}