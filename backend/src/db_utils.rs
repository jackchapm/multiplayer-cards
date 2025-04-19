use std::fmt::Display;
use anyhow::{anyhow, Error};
use aws_sdk_dynamodb::operation::put_item::PutItemOutput;
use aws_sdk_dynamodb::types::{AttributeValue, ReturnValue};
use serde::de::DeserializeOwned;
use serde::{Serialize};

#[macro_export] macro_rules! db_entry {
    ($struct_name:ident, $key_type:ty, $value_type:ty, $prefix:literal) => {
        pub struct $struct_name;
        impl Key for $struct_name {
            type Key = $key_type;
            type Value = $value_type;
            fn prefix() -> &'static str {
                $prefix
            }
        }
    };
}

pub trait Key {
    type Key: Display;
    type Value: Serialize + DeserializeOwned;
    fn prefix() -> &'static str;

    fn key(key: &Self::Key) -> String {
        format!("{}:{key}", Self::prefix())
    }
}

db_entry!(RefreshToken, String, String, "refresh_token");
db_entry!(Connection, String, String, "connection");

pub struct DynamoDBClient {
    client: aws_sdk_dynamodb::Client,
    table_name: String,
}

impl DynamoDBClient {
    pub fn new(client: aws_sdk_dynamodb::Client, table_name: String) -> Self {
        Self { client, table_name }
    }
    pub async fn put_entry<T: Key>(
        &self,
        key: &T::Key,
        value: &T::Value,
    ) -> Result<PutItemOutput, Error> {
        let key_av = AttributeValue::S(T::key(key));
        let value = AttributeValue::S(serde_json::to_string(&value)?);
        self.client.put_item()
            .table_name(&self.table_name)
            .item("pk", key_av)
            .item("content", value)
            .send()
            .await
            .map_err(|err| err.into())
    }

    pub async fn get_entry<T: Key>(&self, key: &T::Key) -> Option<T::Value> {
        let key_av = AttributeValue::S(T::key(key));
        let response = self.client
            .get_item()
            .table_name(&self.table_name)
            .key("pk", key_av)
            .send()
            .await
            .unwrap();
        let value = response.item()?.get("content").unwrap().as_s().unwrap();
        serde_json::from_str::<T::Value>(value).ok()
    }

    pub async fn delete_entry<T: Key>(
        &self,
        key: &T::Key,
        value: Option<&T::Value>,
    ) -> Result<T::Value, Error> {
        let key_av = AttributeValue::S(T::key(key));
        let partial = self.client
            .delete_item()
            .table_name(&self.table_name)
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
