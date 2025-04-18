# Multiplayer Cards
<img style="float: right" src=".github/readme_image.png" width=33%>
A mobile app that allows players to interact with a shared deck of cards.

## Roadmap
Currently, this app is in development. Planned features are below:
- [ ] Base game (deck of cards that can be interacted with, with player hands)
- [ ] Unit / integration testing
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
```