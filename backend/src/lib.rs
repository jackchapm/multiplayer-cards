use serde::{Deserialize, Serialize};

pub mod db_utils;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}