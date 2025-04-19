use serde::{Deserialize, Serialize};

pub mod db_utils;
pub mod message;
pub mod game;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}