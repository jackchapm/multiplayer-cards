use anyhow::{anyhow, Error};
use aws_sdk_dynamodb::operation::put_item::PutItemOutput;
use aws_sdk_dynamodb::types::{AttributeValue, ReturnValue};
use serde::de::DeserializeOwned;
use serde::Serialize;

pub trait Key {
    type Value: Serialize + DeserializeOwned;
    fn prefix() -> &'static str;

    fn key(key: &str) -> String {
        format!("{}:{key}", Self::prefix())
    }
}

macro_rules! db_entry {
    ($struct_name:ident, $value_type:ty, $prefix:literal) => {
        pub struct $struct_name;
        impl Key for $struct_name {
            type Value = $value_type;
            fn prefix() -> &'static str {
                $prefix
            }
        }
    };
}

db_entry!(RefreshToken, String, "refresh_token");
db_entry!(Connection, String, "connection");

#[allow(async_fn_in_trait)]
pub trait DynamoDBClient {
    async fn put_entry<T: Key>(
        &self,
        table_name: &str,
        key: &str,
        value: T::Value,
    ) -> Result<PutItemOutput, Error>;
    async fn get_entry<T: Key>(&self, table_name: &str, key: &str) -> Option<T::Value>;
    async fn delete_entry<T: Key>(
        &self,
        table_name: &str,
        key: &str,
        value: Option<T::Value>,
    ) -> Result<T::Value, Error>;
}

impl DynamoDBClient for &aws_sdk_dynamodb::Client {
    async fn put_entry<T: Key>(
        &self,
        table_name: &str,
        key: &str,
        value: T::Value,
    ) -> Result<PutItemOutput, Error> {
        let key_av = AttributeValue::S(T::key(key));
        let value = AttributeValue::S(serde_json::to_string(&value)?);
        self.put_item()
            .table_name(table_name)
            .item("pk", key_av)
            .item("content", value)
            .send()
            .await
            .map_err(|err| err.into())
    }

    async fn get_entry<T: Key>(&self, table_name: &str, key: &str) -> Option<T::Value> {
        let key_av = AttributeValue::S(T::key(key));
        let response = self
            .get_item()
            .table_name(table_name)
            .key("pk", key_av)
            .send()
            .await
            .unwrap();
        let value = response.item()?.get("content").unwrap().as_s().unwrap();
        serde_json::from_str::<T::Value>(value).ok()
    }

    async fn delete_entry<T: Key>(
        &self,
        table_name: &str,
        key: &str,
        value: Option<T::Value>,
    ) -> Result<T::Value, Error> {
        let key_av = AttributeValue::S(T::key(key));
        let partial = self
            .delete_item()
            .table_name(table_name)
            .key("pk", key_av)
            .return_values(ReturnValue::AllOld);

        let partial = match value {
            Some(value) => partial
                .condition_expression("content = :value")
                .expression_attribute_values(
                    ":value",
                    AttributeValue::S(serde_json::to_string(&value)?),
                ),
            None => partial,
        };

        let response = partial.send().await?;

        let value = response
            .attributes()
            .ok_or(anyhow!("item does not exist"))?
            .get("content")
            .unwrap()
            .as_s()
            .unwrap();

        Ok(serde_json::from_str::<T::Value>(value)?)
    }
}
