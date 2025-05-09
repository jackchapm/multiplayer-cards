use std::collections::HashMap;
use std::iter;
use crate::db_utils::{Key};
use crate::requests::{DeckType, GameStateData, WebsocketResponse};
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
use crate::requests::WebsocketRequestDiscriminants::{FlipCard, FlipStack, JoinGame, LeaveGame, MoveStack, Ping, PutCard, Reset, Shuffle, TakeCard};

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
            None => Player::new(services, player_id, self.id.clone()).await?,
        };
        self.connected_players.insert(player.player_id.clone(), conn_id.to_string());
        services.put::<Game>(&self.id, &self).await?;
        services.send(conn_id, &GameStateData {
            cause_action: Some(Ping),
            // could send current player but won't provide any extra detail and involves another clone
            cause_player: None,
            owner: Some(self.owner.clone()),
            players: Some(self.connected_players.keys().cloned().collect()),
            stacks: Some(self.stacks.iter().map(Stack::state).collect())
        }.with(&self.id)).await?;
        self.send_state_all(services, &GameStateData {
            cause_action: Some(JoinGame),
            cause_player: Some(player.player_id.clone()),
            ..Default::default()
        }.with(&self.id)).await?;
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
        self.send_state_all(services, &GameStateData {
            cause_action: Some(LeaveGame),
            cause_player: Some(player_id),
            owner: Some(self.owner.clone()),
            ..Default::default()
        }.with(&self.id)).await?;
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

    /// Returns the popped card, as well as the remaining stack state
    fn pop_from_stack(&mut self, stack_id: StackId) -> Result<(Card, StackState), WebsocketError> {
        let stack_index = self.stacks.iter().position(|s| s.id == stack_id)
            .ok_or(WebsocketError::StackNotFound)?;

        let stack = self.stacks.get_mut(stack_index).unwrap();
        let card = stack.cards.pop().ok_or(WebsocketError::EmptyStack)?;
        let state = stack.state();

        if stack.cards.is_empty() {
            self.stacks.swap_remove(stack_index);
        }
        Ok((card, state))
    }

    async fn get_player(&self, services: &Services, player_id: &PlayerId) -> Result<Player, WebsocketError> {
        services.get::<Player>(player_id)
            .await
            .ok_or_else(|| WebsocketError::PlayerNotFound)
    }

    async fn save(&self, services: &Services) -> Result<(), Error> {
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
        let state = {
            let stack = self.get_stack(stack_id)?;
        if stack.cards.is_empty() {
            // todo handle deleting this stack
            return Err(WebsocketError::EmptyStack)
        }
        stack.cards.last_mut().unwrap().flip();
            stack.state()
            };
        self.save(services).await?;
        self.send_state_all(services, &GameStateData {
            cause_action: Some(FlipCard),
            stacks: Some(vec![state]),
            .. Default::default()
        }.with(&self.id)).await?;
        Ok(())
    }

    pub async fn flip_stack(&mut self, services: &Services, stack_id: StackId) -> Result<(), WebsocketError> {
        let state = {
            let stack = self.get_stack(stack_id)?;
            stack.cards.reverse();
            for card in &mut stack.cards {
                card.flip();
            }
            stack.state()
        };
        self.save(services).await?;
        self.send_state_all(services, &GameStateData {
            cause_action: Some(FlipStack),
            stacks: Some(vec![state]),
            ..Default::default()
        }.with(&self.id)).await?;
        Ok(())
    }

    pub async fn shuffle_stack(&mut self, services: &Services, stack_id: StackId) -> Result<(), WebsocketError> {
        let state = {
            let stack = self.get_stack(stack_id)?;
            stack.cards.shuffle(&mut rng());
            stack.state()
        };

        self.save(services).await?;
        self.send_state_all(services, &GameStateData {
            cause_action: Some(Shuffle),
            stacks: Some(vec![state]),
            ..Default::default()
        }.with(&self.id)).await?;
        Ok(())
    }

    pub async fn move_stack(&mut self, services: &Services, stack_id: StackId, position: (i8, i8)) -> Result<(), WebsocketError> {
        let stack_index = self.stacks.iter().position(|s| s.id == stack_id)
            .ok_or(WebsocketError::StackNotFound)?;

        let (old_stack_state, new_stack_state) = {
            let mut mut_stack = self.stacks.swap_remove(stack_index);
            if let Some(target_stack) = self.stack_at_position(position, false) {
                target_stack.cards.extend(mut_stack.cards.drain(..));
                (Some(mut_stack.state()), target_stack.state())
            } else {
                mut_stack.position = position;
                let state = mut_stack.state();
                self.stacks.push(mut_stack);
                (None, state)
            }
        };

        self.save(services).await?;
        self.send_state_all(services, &GameStateData {
            cause_action: Some(MoveStack),
            stacks: Some(iter::once(new_stack_state).chain(old_stack_state.into_iter()).collect()),
            ..Default::default()
        }.with(&self.id)).await?;
        Ok(())
    }

    pub async fn move_card(&mut self, services: &Services, stack_id: StackId, position: (i8, i8)) -> Result<(), WebsocketError> {
        let (old_stack_state, new_stack_state) = {
            let (card, old_stack_state) = self.pop_from_stack(stack_id)?;
            let target_stack = self.stack_at_position(position, true).unwrap();
            target_stack.cards.push(card);
            (target_stack.state(), old_stack_state)
        };

        self.save(services).await?;
        self.send_state_all(services, &GameStateData {
            cause_action: Some(MoveStack),
            stacks: Some(vec![old_stack_state, new_stack_state]),
            ..Default::default()
        }.with(&self.id)).await?;
        Ok(())
    }

    pub async fn take_card(&mut self, services: &Services, stack_id: StackId, player_id: &PlayerId, conn_id: &str) -> Result<(), WebsocketError> {
        let mut player = self.get_player(services, player_id).await?;
        let (card, stack_state) = self.pop_from_stack(stack_id)?;
        player.hand.push(card);
        self.save(services).await?;
        self.send_state_all(services, &GameStateData {
            cause_action: Some(TakeCard),
            cause_player: Some(player_id.clone()),
            stacks: Some(vec![stack_state]),
            ..Default::default()
        }.with(&self.id)).await?;
        player.send_state(services, conn_id).await?;
        services.put::<Player>(&player.player_id, &player).await?;
        Ok(())
    }

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

        let state = {
            let target_stack = self.stack_at_position(position, true).unwrap();
            let mut card = player.hand.swap_remove(hand_index);
            if face_down != card.is_face_down() {
                card.flip()
            }
            target_stack.cards.push(card);
            target_stack.state()
        };

        player.send_state(services, conn_id).await?;
        services.put::<Player>(&player.player_id, &player).await?;
        self.save(services).await?;
        self.send_state_all(services, &GameStateData{
            cause_action: Some(PutCard),
            cause_player: Some(player_id.clone()),
            stacks: Some(vec![state]),
            ..Default::default()
        }.with(&self.id)).await?;
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
        self.save(services).await?;
        self.send_state_all(services, &GameStateData {
            cause_action: Some(Reset),
            stacks: Some(self.stacks.iter().map(Stack::state).collect()),
            ..Default::default()
        }.with(&self.id)).await?;
        Ok(())
    }

    /// Send a websocket response to all players connected to the game
    async fn send_state_all(&self, services: &Services, data: &WebsocketResponse) -> Result<(), Error> {
        services.send_batch(self.connected_players.values(), data).await
    }
}
