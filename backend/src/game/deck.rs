use rand::rng;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};
use std::ops::{Deref, DerefMut};
use schemars::JsonSchema;
use strum::Display;

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
    Ace = 0,
    Two = 1,
    Three = 2,
    Four = 3,
    Five = 4,
    Six = 5,
    Seven = 6,
    Eight = 7,
    Nine = 8,
    Ten = 9,
    Jack = 10,
    Queen = 11,
    King = 12,
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

#[repr(transparent)]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Card(u8);

impl Card {
    pub fn numerical(rank: Rank, suit: Suit) -> Self {
        Self(((rank as u8) << 2) | (suit as u8))
    }

    pub fn special(kind: SpecialCard) -> Self {
        Self(0b1000_0000 | (kind as u8))
    }

    fn from_u8(val: u8) -> Self {
        debug_assert!(
            val < 52 || (val & 0b1000_0000 != 0 && val & 0b0111_1111 <= SpecialCard::MAX),
            "Invalid card byte: {}",
            val
        );
        Card(val)
    }

    pub fn is_special(self) -> bool {
        self.0 & 0b1000_0000 != 0
    }

    pub fn is_numerical(self) -> bool {
        self.0 < 52
    }

    pub fn kind(self) -> Option<SpecialCard> {
        self.is_special().then(|| SpecialCard::from_u8(self.0))
    }

    pub fn rank(self) -> Option<Rank> {
        self.is_numerical().then(|| Rank::from_u8(self.0))
    }

    pub fn suit(self) -> Option<Suit> {
        self.is_numerical().then(|| Suit::from_u8(self.0))
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

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct Deck {
    pub(super) cards: Vec<Card>,
    pub(super) face_up: bool,
    pub(super) capacity: Option<usize>,
}

#[derive(Debug, Default)]
pub struct DeckOptions {
    pub face_up: bool,
    pub capacity: Option<usize>,
}

impl Deck {
    pub fn shuffled52(deck_options: DeckOptions) -> Self {
        let mut cards = Vec::with_capacity(deck_options.capacity.unwrap_or(52));
        cards.extend((0..52).map(|x| Card::from_u8(x)));

        let mut deck = Self::from_options(&deck_options, cards);
        deck.shuffle();
        deck
    }

    pub fn empty(deck_options: DeckOptions) -> Self {
        Self::from_options(
            &deck_options,
            deck_options
                .capacity
                .map_or_else(|| Vec::new(), |capacity| Vec::with_capacity(capacity)),
        )
    }

    fn from_options(deck_options: &DeckOptions, cards: Vec<Card>) -> Self {
        Self {
            cards,
            face_up: deck_options.face_up,
            capacity: deck_options.capacity,
        }
    }

    pub fn shuffle(&mut self) {
        self.cards.shuffle(&mut rng());
    }
}

// todo manually implement needed methods
impl Deref for Deck {
    type Target = Vec<Card>;
    fn deref(&self) -> &Self::Target {
        &self.cards
    }
}

impl DerefMut for Deck {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.cards
    }
}
