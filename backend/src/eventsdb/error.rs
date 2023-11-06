use std::num::ParseIntError;

use aws_sdk_dynamodb::{
    error::SdkError,
    operation::{
        create_table::CreateTableError, get_item::GetItemError, list_tables::ListTablesError,
        put_item::PutItemError,
    },
};
use aws_smithy_http::body::SdkBody;
use axum::http::Response;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("General Error: {0}")]
    General(String),

    #[error("Malformed Object at field: {0}")]
    MalformedObject(String),

    #[error("Concurrency Error")]
    Concurrency,

    #[error("Item Not Found")]
    ItemNotFound,

    #[error("Serde Error: {0}")]
    Serde(#[from] serde_json::Error),

    #[error("ParseInt Error: {0}")]
    ParseInt(#[from] ParseIntError),

    #[error("Dynamo PutItemError: {0}")]
    DynamoPut(#[from] SdkError<PutItemError, Response<SdkBody>>),

    #[error("Dynamo ListTablesError: {0}")]
    DynamoListTables(#[from] SdkError<ListTablesError, Response<SdkBody>>),

    #[error("Dynamo CreateTableError: {0}")]
    DynamoCreateTable(#[from] SdkError<CreateTableError, Response<SdkBody>>),

    #[error("Dynamo GetItemError: {0}")]
    DynamoGetItem(#[from] SdkError<GetItemError, Response<SdkBody>>),
}

pub type Result<T> = std::result::Result<T, Error>;
