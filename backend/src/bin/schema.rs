use std::fs::File;
use std::io::{Error, Write};
use schemars::schema_for;
use multiplayer_cards::message::{WebsocketRequest, WebsocketResponse};

fn main() -> Result<(), Error> {
    let request_schema = schema_for!(WebsocketRequest);
    let response_schema = schema_for!(WebsocketResponse);

    let mut websocket_file = File::create("../websocket-message.json")?;
    let mut response_file = File::create("../websocket-response.json")?;

    websocket_file.write_all(serde_json::to_string_pretty(&request_schema)?.as_ref())?;
    response_file.write_all(serde_json::to_string_pretty(&response_schema)?.as_ref())
}