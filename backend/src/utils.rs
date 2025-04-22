use aws_lambda_events::apigw::ApiGatewayRequestAuthorizer;

pub trait AuthorizerUtils {
    fn unwrap_field(&self, field: &str) -> String;
}

impl AuthorizerUtils for ApiGatewayRequestAuthorizer {
    fn unwrap_field(&self, field: &str) -> String {
        self.fields
            .get(field)
            .expect("request authorizer not found")
            .as_str()
            .expect("invalid format")
            .to_string()
    }
}