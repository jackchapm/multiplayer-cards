# Multiplayer Cards
<img style="float: right" src=".github/readme_image.png" width=33%>
A mobile app that allows players to interact with a shared deck of cards.

## Roadmap
This app is currently in development and this list also acts as a todo list for features I
think of to implement. Not every feature is guaranteed to be implemented.

**Planned Features:**
- [ ] MVP (deck of cards that can be interacted with, with player hands)
- [ ] (frontend): Highlight face up cards in players hand
- [ ] (frontend): Tutorial on how to manipulate cards
- [ ] (backend): Move game store to in memory database (upstash) - can pop from list directly
- [ ] (backend): Replace json encoding with protobuf for efficiency
- [ ] (backend): Unit / integration testing
- [ ] Reclaim disconnected player's cards
- [ ] Enforced player turns
- [ ] Multiple shared deck support (e.g multiple stacks)
- [ ] Local only games, or locally discoverable tables
- [ ] Preconfigured card games (+ community support?)
- [ ] Betting support for preconfigured games
- [ ] Option to have central device as table
- [ ] Monetisation route (e.g deck skins, player limits)

## Accessing the WebSocket
Currently, no credentials are required for authentication. Simply `POST` to the `/auth/guest` API Gateway endpoint to retrieve an access token. To get another token after this has expired, `POST` to `/auth/refresh`, with the header `Authorization: Bearer {refresh_token}` 

## Deploying to AWS
AWS CDK is used to deploy the lambda functions and configure the AWS environment. To deploy:

1) Follow [AWS User Guide](https://docs.aws.amazon.com/signin/latest/userguide/command-line-sign-in.html) for how to sign in using the AWS command line.
2) Compiling the rust functions depends on the cargo-lambda tool to compile for the AWS environment. Follow their [Getting Started guided here](https://www.cargo-lambda.info/guide/getting-started.html).
3) Fill in values in `deployment/config.json` 
4) Deploy lambda functions by running the following command in the `deployment` directory (`package.json` is configured to use `bun` as the package manager and runtime although this can be changed).

```shell
bun install
cdk deploy
# Optional: generate JSON schema to validate kotlin data classes
cd ../backend
cargo run --bin schema
```