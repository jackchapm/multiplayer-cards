use std::fs::File;
use std::io::{Error, Write};
use schemars::schema_for;
use multiplayer_cards::message::WebsocketMessage;

fn main() -> Result<(), Error> {
    let schema = schema_for!(WebsocketMessage);
    let mut file = File::create("../websocket-message.json")?;
    file.write_all(serde_json::to_string_pretty(&schema)?.as_ref())
}