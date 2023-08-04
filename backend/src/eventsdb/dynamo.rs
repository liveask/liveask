use async_trait::async_trait;
use aws_sdk_dynamodb::{
    error::SdkError,
    operation::put_item::PutItemError,
    types::{
        AttributeDefinition, AttributeValue, KeySchemaElement, KeyType, ProvisionedThroughput,
        ScalarAttributeType,
    },
};
use aws_smithy_http::body::SdkBody;
use axum::http::Response;
use tracing::instrument;

use crate::eventsdb::event_key;

use super::{
    error::{Error, Result},
    types::{ApiEventInfo, AttributeMap},
    EventEntry, EventsDB,
};

const DB_TABLE_NAME: &str = "liveask";

#[derive(Clone)]
pub struct DynamoEventsDB {
    db: aws_sdk_dynamodb::Client,
    table: String,
}

#[async_trait]
impl EventsDB for DynamoEventsDB {
    #[instrument(skip(self), err)]
    async fn get(&self, key: &str) -> Result<EventEntry> {
        let key = event_key(key);

        let res = self
            .db
            .get_item()
            .table_name(&self.table)
            .key("key", AttributeValue::S(key))
            .send()
            .await?;

        let item = res.item().ok_or(Error::ItemNotFound)?;

        let format_version = item
            .get("format")
            .and_then(|value| value.as_n().ok())
            .and_then(|format| format.parse::<usize>().ok())
            .unwrap_or_default();

        if format_version == 0 {
            let version = item["v"]
                .as_n()
                .map_err(|_| Error::General("malformed event: `v`".into()))?
                .parse::<usize>()?;

            let value = item["value"]
                .as_s()
                .map_err(|_| Error::General("malformed event: `value`".to_string()))?;

            let event: ApiEventInfo = serde_json::from_str(value)?;

            Ok(EventEntry {
                event,
                version,
                ttl: None,
            })
        } else {
            Ok(EventEntry::try_from(item)?)
        }
    }

    #[instrument(skip(self), err)]
    async fn put(&self, event: EventEntry) -> Result<()> {
        let event_version = event.version;

        let attributes: AttributeMap = event.into();

        let mut request = self
            .db
            .put_item()
            .table_name(&self.table)
            .set_item(Some(attributes));

        if event_version > 0 {
            let old_version_av = AttributeValue::N(event_version.saturating_sub(1).to_string());
            request = request
                .condition_expression("v = :ver")
                .expression_attribute_values(":ver", old_version_av);
        }

        //Note: filter out conditional error
        if let Err(e) = request.send().await {
            if matches!(&e,SdkError::<PutItemError, Response<SdkBody>>::ServiceError (err)
            if matches!(
                err.err(),PutItemError::ConditionalCheckFailedException(_)

            )) {
                return Err(Error::Concurrency);
            }

            return Err(Error::DynamoPut(e));
        }

        Ok(())
    }
}

impl DynamoEventsDB {
    pub async fn new(db: aws_sdk_dynamodb::Client, check_table_exists: bool) -> Result<Self> {
        if check_table_exists {
            let resp = db.list_tables().send().await?;
            let names = resp.table_names().unwrap_or_default();

            tracing::trace!("tables: {}", names.join(","));

            if !names.contains(&DB_TABLE_NAME.into()) {
                tracing::info!("table not found, creating now");

                create_table(&db, DB_TABLE_NAME.into(), "key".into()).await?;
            }
        }

        Ok(Self {
            db,
            table: DB_TABLE_NAME.into(),
        })
    }
}

async fn create_table(
    client: &aws_sdk_dynamodb::Client,
    table_name: String,
    key_name: String,
) -> Result<()> {
    let ad = AttributeDefinition::builder()
        .attribute_name(&key_name)
        .attribute_type(ScalarAttributeType::S)
        .build();

    let ks = KeySchemaElement::builder()
        .attribute_name(&key_name)
        .key_type(KeyType::Hash)
        .build();

    let pt = ProvisionedThroughput::builder()
        .read_capacity_units(5)
        .write_capacity_units(5)
        .build();

    client
        .create_table()
        .table_name(table_name)
        .attribute_definitions(ad)
        .key_schema(ks)
        .provisioned_throughput(pt)
        .send()
        .await?;

    Ok(())
}
