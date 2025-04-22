use crate::requests::{DeckType};
use rand::rng;
use rand::seq::SliceRandom;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};
use strum::Display;
use uuid::Uuid;

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Display)]
pub enum Suit {
    Spades = 0,
    Hearts = 1,
    Diamonds = 2,
    Clubs = 3,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Display)]
pub enum Rank {
    Ace = 1,
    Two = 2,
    Three = 3,
    Four = 4,
    Five = 5,
    Six = 6,
    Seven = 7,
    Eight = 8,
    Nine = 9,
    Ten = 10,
    Jack = 11,
    Queen = 12,
    King = 13,
}

#[repr(u8)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Display)]
pub enum SpecialCard {
    JokerBlack = 0,
    JokerRed = 1,
}

impl Rank {
    fn from_u8(val: u8) -> Self {
        debug_assert!(val >> 2 <= 12, "Invalid rank: {}", val);
        unsafe { std::mem::transmute(val >> 2) }
    }
}

impl Suit {
    fn from_u8(val: u8) -> Self {
        debug_assert!(val & 0b11 <= 3, "Invalid suit: {}", val);
        unsafe { std::mem::transmute(val & 0b11) }
    }
}

impl SpecialCard {
    const MAX: u8 = SpecialCard::JokerRed as u8;

    fn from_u8(val: u8) -> Self {
        debug_assert!(val & 0b0111_1111 <= Self::MAX, "Invalid suit: {}", val);
        unsafe { std::mem::transmute(val & 0b0111_1111) }
    }
}

/// If bit 7 is set, represents a special card
/// If bit 6 is set, represents a face down card
/// If special card, bits 0-5 represent the special card type
/// If ordinary card, bits 2-5 represent the rank, and 0-1 represent the suit
/// (0 - Space, 1 - Heart, 2 - Diamond, 3 - Club)
#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Card(u8);

impl Card {
    /// Hidden card sent to clients
    /// Cards that are turned over in the deck will have bot 6 set, so the value is still accessible
    pub const HIDDEN_CARD: Card = Self(0b0100_0000);

    pub fn numerical(rank: Rank, suit: Suit) -> Self {
        Self(((rank as u8) << 2) | (suit as u8))
    }

    pub fn special(kind: SpecialCard) -> Self {
        Self(0b1000_0000 | (kind as u8))
    }

    fn from_u8(val: u8) -> Self {
        debug_assert!(
            val & 0b1011_000 <= 52
                || (val & 0b1000_0000 != 0 && val & 0b0011_1111 <= SpecialCard::MAX),
            "Invalid card byte: {}",
            val
        );
        Card(val)
    }

    pub fn is_face_down(&self) -> bool {
        self.0 & 0b0100_0000 != 0
    }

    pub fn is_special(&self) -> bool {
        self.0 & 0b1000_0000 != 0
    }

    pub fn is_numerical(&self) -> bool {
        self.0 & 0b1000_0000 == 0
    }

    pub fn kind(&self) -> Option<SpecialCard> {
        self.is_special().then(|| SpecialCard::from_u8(self.0))
    }

    pub fn rank(&self) -> Option<Rank> {
        self.is_numerical().then(|| Rank::from_u8(self.0))
    }

    pub fn suit(&self) -> Option<Suit> {
        self.is_numerical().then(|| Suit::from_u8(self.0))
    }
    
    pub fn flip(&mut self) {
        self.0 ^= 0b0100_0000
    }
}

impl Display for Card {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(kind) = self.kind() {
            write!(f, "{}", kind)
        } else if let (Some(rank), Some(suit)) = (self.rank(), self.suit()) {
            write!(f, "{} of {}", rank, suit)
        } else {
            write!(f, "Unknown card")
        }
    }
}

pub type StackId = String;

#[derive(Debug, Serialize, Deserialize)]
pub struct Stack {
    // todo uuid for now but can probably be int u8
    pub id: StackId,
    pub cards: Vec<Card>,
    pub position: (i8, i8),
}

impl Stack {
    pub(super) fn from(deck_type: DeckType) -> Vec<Self> {
        match deck_type {
            DeckType::Standard => {
                let mut cards: Vec<_> = (1..=52).map(Card::from_u8).collect();
                // Turn every card face down
                for card in &mut cards {
                    card.0 |= 0b0100_0000;
                }
                cards.shuffle(&mut rng());

                vec![Self {
                    id: Uuid::new_v4().to_string(),
                    cards,
                    position: (0, 0),
                }]
            }
            DeckType::Custom(custom_deck) => {
                custom_deck.into_iter().map(|cards| {
                    Self {
                        id: Uuid::new_v4().to_string(),
                        cards,
                        position: (0, 0),
                    }
                }).collect()
            }
        }
    }

    pub(super) fn state(&self) -> StackState {
        let top_card = self.cards.last().cloned().unwrap();
        let top_card = if top_card.is_face_down() { Card::HIDDEN_CARD } else { top_card };

        StackState {
            //todo do i need to clone here
            stack_id: self.id.clone(),
            position: self.position.clone(),
            visible_card: top_card,
            remaining_cards: self.cards.len(),
        }
    }
}

#[derive(Debug, Serialize, PartialEq, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StackState {
    pub stack_id: StackId,
    pub position: (i8, i8),
    pub visible_card: Card,
    pub remaining_cards: usize,
}
