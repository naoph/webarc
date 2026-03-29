use super::schema::*;
use diesel::{Insertable, Queryable};

#[derive(Queryable)]
pub struct DbUser {
    pub id: i32,
    pub username: String,
    pub passhash: String,
}

#[derive(Insertable)]
#[diesel(table_name=users)]
pub struct InsUser {
    pub username: String,
    pub passhash: String,
}

impl InsUser {
    pub fn new(username: String, passhash: String) -> Self {
        Self { username, passhash }
    }
}

#[derive(Queryable)]
pub struct DbCapture {
    pub id: i32,
    pub uuid: uuid::Uuid,
    pub url: url::Url,
    pub time_initiated: chrono::DateTime<chrono::Utc>,
    pub owner: i32,
    pub public: bool,
}

#[derive(Insertable)]
#[diesel(table_name=captures)]
pub struct InsCapture {
    pub uuid: uuid::Uuid,
    #[diesel(serialize_as=String)]
    pub url: url::Url,
    pub time_initiated: chrono::DateTime<chrono::Utc>,
    pub owner: i32,
    pub public: bool,
}

#[derive(Queryable)]
pub struct DbExtract {
    pub id: i32,
    pub uuid: uuid::Uuid,
    pub capture: i32,
    pub extractor: String,
    pub success: bool,
}

#[derive(Insertable)]
#[diesel(table_name=extracts)]
pub struct InsExtract {
    pub uuid: uuid::Uuid,
    pub capture: i32,
    pub extractor: String,
    pub success: bool,
}
