use aws_lambda_events::apigw::{
    ApiGatewayCustomAuthorizerPolicy, ApiGatewayCustomAuthorizerRequestTypeRequest,
    ApiGatewayCustomAuthorizerResponse,
};
use aws_lambda_events::iam::{IamPolicyEffect, IamPolicyStatement};
use jsonwebtoken::{decode, DecodingKey, Validation};
use lambda_runtime::{run, service_fn, tracing, Error, LambdaEvent};
use multiplayer_cards::auth::{AuthorizationContext, Claims, HTTP_AUDIENCE, WEBSOCKET_AUDIENCE};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    let secret = std::env::var("JWT_SECRET").expect("jwt secret not set");
    let websocket_arn = std::env::var("WEBSOCKET_ARN").expect("websocket arn not set");
    let decoding_key = DecodingKey::from_secret(secret.as_bytes());

    run(service_fn(async |e| {
        function_handler(e, &decoding_key, &websocket_arn).await
    }))
    .await
}

pub async fn function_handler(
    event: LambdaEvent<ApiGatewayCustomAuthorizerRequestTypeRequest>,
    decoding_key: &DecodingKey,
    websocket_arn: &str,
) -> Result<ApiGatewayCustomAuthorizerResponse, Error> {
    let auth_header = event
        .payload
        .headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    let method_arn = event.payload.method_arn.expect("no arn given for auth");

    let Some(token) = (match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            Some(header.trim_start_matches("Bearer ").trim())
        }
        _ => None,
    }) else {
        return Ok(generate_response(IamPolicyEffect::Deny, method_arn, None));
    };


    let mut validation = Validation::default();
    if method_arn == websocket_arn {
        validation.set_audience(&[WEBSOCKET_AUDIENCE])
    } else {
        validation.set_audience(&[HTTP_AUDIENCE])
    }

    decode::<Claims>(token, &decoding_key, &validation)
        .map(|t| t.claims)
        .map_or_else(
            |_| {
                Ok(generate_response(
                    IamPolicyEffect::Deny,
                    method_arn.clone(),
                    None,
                ))
            },
            |claims| {
                Ok(generate_response(
                    IamPolicyEffect::Allow,
                    method_arn.clone(),
                    Some(claims),
                ))
            },
        )
}

pub fn generate_response(
    effect: IamPolicyEffect,
    arn: String,
    claims: Option<Claims>,
) -> ApiGatewayCustomAuthorizerResponse {
    ApiGatewayCustomAuthorizerResponse {
        principal_id: claims.as_ref().map(|c| c.sub.clone()),
        policy_document: ApiGatewayCustomAuthorizerPolicy {
            version: Some(String::from("2012-10-17")),
            statement: vec![IamPolicyStatement {
                action: vec![String::from("execute-api:Invoke")],
                effect,
                resource: vec![arn],
                condition: None,
            }],
        },
        context: claims
            .map(|claims| {
                json!(AuthorizationContext {
                    uuid: claims.sub,
                    expires: claims.exp,
                    game_id: claims.game_id
                })
            })
            .unwrap_or_else(|| json!({})),
        usage_identifier_key: None,
    }
}
