use std::num::ParseIntError;

use aws_sdk_dynamodb::{
    error::SdkError,
    operation::{
        create_table::CreateTableError, get_item::GetItemError, list_tables::ListTablesError,
        put_item::PutItemError,
    },
};
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
    DynamoPut(Box<SdkError<PutItemError>>),

    #[error("Dynamo ListTablesError: {0}")]
    DynamoListTables(Box<SdkError<ListTablesError>>),

    #[error("Dynamo CreateTableError: {0}")]
    DynamoCreateTable(Box<SdkError<CreateTableError>>),

    #[error("Dynamo GetItemError: {0}")]
    DynamoGetItem(Box<SdkError<GetItemError>>),

    #[error("Dynamo BuildError: {0}")]
    DynamoBuild(#[from] aws_sdk_dynamodb::error::BuildError),

    #[error("serde_dynamo BuildError: {0}")]
    SerdeDynamo(#[from] serde_dynamo::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<SdkError<PutItemError>> for Error {
    fn from(e: SdkError<PutItemError>) -> Self {
        Self::DynamoPut(Box::new(e))
    }
}

impl From<SdkError<ListTablesError>> for Error {
    fn from(e: SdkError<ListTablesError>) -> Self {
        Self::DynamoListTables(Box::new(e))
    }
}

impl From<SdkError<CreateTableError>> for Error {
    fn from(e: SdkError<CreateTableError>) -> Self {
        Self::DynamoCreateTable(Box::new(e))
    }
}

impl From<SdkError<GetItemError>> for Error {
    fn from(e: SdkError<GetItemError>) -> Self {
        Self::DynamoGetItem(Box::new(e))
    }
}
