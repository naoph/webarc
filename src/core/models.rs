use super::schema::*;
use diesel::{Insertable, Queryable};

/// Wrapper around `String` for loading `url::Url`s from databases
pub struct IntermediaryUrl(String);

impl TryInto<url::Url> for IntermediaryUrl {
    type Error = url::ParseError;
    fn try_into(self) -> Result<url::Url, url::ParseError> {
        url::Url::parse(&self.0)
    }
}

impl<DB> Queryable<diesel::sql_types::Text, DB> for IntermediaryUrl
where
    DB: diesel::backend::Backend,
    String: diesel::deserialize::FromSql<diesel::sql_types::Text, DB>,
{
    type Row = String;

    fn build(s: String) -> diesel::deserialize::Result<Self> {
        Ok(Self(s))
    }
}

#[derive(Debug, Queryable)]
pub struct DbUser {
    pub id: i32,
    pub username: String,
    pub passhash: String,
}

#[derive(Debug, Insertable)]
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

#[derive(Debug, Queryable)]
pub struct DbCapture {
    pub id: i32,
    pub uuid: uuid::Uuid,
    #[diesel(deserialize_as=IntermediaryUrl)]
    pub url: url::Url,
    pub time_initiated: chrono::DateTime<chrono::Utc>,
    pub owner: i32,
    pub public: bool,
}

#[derive(Debug, Insertable)]
#[diesel(table_name=captures)]
pub struct InsCapture {
    pub uuid: uuid::Uuid,
    #[diesel(serialize_as=String)]
    pub url: url::Url,
    pub time_initiated: chrono::DateTime<chrono::Utc>,
    pub owner: i32,
    pub public: bool,
}

#[derive(Debug, Queryable)]
pub struct DbExtract {
    pub id: i32,
    pub uuid: uuid::Uuid,
    pub capture: i32,
    pub extractor: String,
    pub success: bool,
}

#[derive(Debug, Insertable)]
#[diesel(table_name=extracts)]
pub struct InsExtract {
    pub uuid: uuid::Uuid,
    pub capture: i32,
    pub extractor: String,
    pub success: bool,
}

impl InsExtract {
    pub fn new(uuid: uuid::Uuid, capture: i32, extractor: String, success: bool) -> Self {
        Self {
            uuid,
            capture,
            extractor,
            success,
        }
    }
}
