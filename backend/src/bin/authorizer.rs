use aws_lambda_events::apigw::{
    ApiGatewayCustomAuthorizerPolicy, ApiGatewayCustomAuthorizerRequestTypeRequest,
    ApiGatewayCustomAuthorizerResponse,
};
use aws_lambda_events::iam::{IamPolicyEffect, IamPolicyStatement};
use jsonwebtoken::{decode, DecodingKey, Validation};
use lambda_runtime::{run, service_fn, tracing, Error, LambdaEvent};
use multiplayer_cards::Claims;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize)]
pub struct AuthorizationContext {
    pub uuid: String,
    pub expires: usize,
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    run(service_fn(function_handler)).await
}

pub async fn function_handler(
    event: LambdaEvent<ApiGatewayCustomAuthorizerRequestTypeRequest>,
) -> Result<ApiGatewayCustomAuthorizerResponse, Error> {
    let auth_header = event
        .payload
        .headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(header) if header.starts_with("Bearer ") => {
            Some(header.trim_start_matches("Bearer ").trim())
        }
        _ => None,
    };

    // identity source set for authorizer, so header SHOULD be present
    // we'll check anyway
    if let Some(token) = token {
        let secret = std::env::var("JWT_SECRET")?;
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());
        let validation = Validation::default();
        let arn = event.payload.method_arn.expect("no arn given for auth");

        decode::<Claims>(token, &decoding_key, &validation)
            .map(|t| t.claims)
            .map_or_else(
                |_| Ok(generate_response(IamPolicyEffect::Deny, arn.clone(), None)),
                |claims| {
                    Ok(generate_response(
                        IamPolicyEffect::Allow,
                        arn.clone(),
                        Some(claims),
                    ))
                },
            )
    } else {
        Ok(generate_response(
            IamPolicyEffect::Deny,
            event.payload.method_arn.expect("no arn given for auth"),
            None,
        ))
    }
}

fn generate_response(
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
                })
            })
            .unwrap_or(Value::Null),
        usage_identifier_key: None,
    }
}
