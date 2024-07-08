use chrono::NaiveDateTime;
use num_bigint::{BigInt, Sign};
use serde::{Deserialize, Serialize};
use sqlx::types::BigDecimal;
use std::convert::Into;
use ulid::Ulid;

use std::str::FromStr;

use crate::utils::{
    database::DatabaseConnection,
    pagination::{Paginated, Pagination},
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Kitchen {
    pub id: String,
    pub name: String,
    pub address: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub phone_number: String,
    pub opening_time: String,
    pub closing_time: String,
    pub preparation_time: String,
    pub delivery_time: String,
    pub cover_image_url: Option<String>,
    pub rating: BigDecimal,
    pub owner_id: String,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
}

pub struct CreateKitchenPayload {
    pub name: String,
    pub address: String,
    pub phone_number: String,
    pub type_: String,
    pub opening_time: String,
    pub closing_time: String,
    pub preparation_time: String,
    pub delivery_time: String,
    pub owner_id: String,
}

pub enum Error {
    UnexpectedError,
}

pub async fn create(db: DatabaseConnection, payload: CreateKitchenPayload) -> Result<(), Error> {
    match sqlx::query!(
        "
        INSERT INTO kitchens (
            id,
            name,
            address,
            type,
            phone_number,
            opening_time,
            closing_time,
            preparation_time,
            delivery_time,
            rating,
            owner_id
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
    ",
        Ulid::new().to_string(),
        payload.name,
        payload.address,
        payload.type_,
        payload.phone_number,
        payload.opening_time,
        payload.closing_time,
        payload.preparation_time,
        payload.delivery_time,
        BigDecimal::new(BigInt::new(Sign::Plus, vec![0]), 2),
        payload.owner_id
    )
    .execute(&db.pool)
    .await
    {
        Ok(_) => Ok(()),
        Err(err) => {
            tracing::info!("Error occurred while trying to create a kitchen: {}", err);
            Err(Error::UnexpectedError)
        }
    }
}

#[derive(Deserialize)]
struct DatabaseCountedResult {
    data: Vec<Kitchen>,
    total: u32,
}

impl Into<DatabaseCountedResult> for Option<serde_json::Value> {
    fn into(self) -> DatabaseCountedResult {
        match self {
            Some(json) => {
                serde_json::de::from_str::<DatabaseCountedResult>(json.to_string().as_ref())
                    .unwrap()
            }
            None => DatabaseCountedResult {
                data: vec![],
                total: 0,
            },
        }
    }
}

#[derive(Deserialize)]
struct DatabaseCounted {
    result: DatabaseCountedResult,
}

pub async fn find_many(
    db: DatabaseConnection,
    pagination: Pagination,
) -> Result<Paginated<Kitchen>, Error> {
    match sqlx::query_as!(
        DatabaseCounted,
        "
            WITH filtered_data AS (
                SELECT *
                FROM kitchens 
                LIMIT $1
                OFFSET $2
            ), 
            total_count AS (
                SELECT COUNT(id) AS total_rows
                FROM kitchens
            )
            SELECT JSONB_BUILD_OBJECT(
                'data', JSONB_AGG(ROW_TO_JSON(filtered_data)),
                'total', (SELECT total_rows FROM total_count)
            ) AS result
            FROM filtered_data;
        ",
        pagination.per_page as i64,
        ((pagination.page - 1) * pagination.per_page) as i64,
    )
    .fetch_one(&db.pool)
    .await
    {
        Ok(counted) => Ok(Paginated::new(
            counted.result.data,
            counted.result.total,
            pagination.page,
            pagination.per_page,
        )),
        Err(err) => {
            tracing::info!(
                "Error occurred while trying to fetch many kitchens: {}",
                err
            );
            Err(Error::UnexpectedError)
        }
    }
}
