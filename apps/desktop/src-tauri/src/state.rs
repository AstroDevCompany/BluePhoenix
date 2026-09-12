use crate::db::Db;

pub struct AppState {
    pub db: Db,
    pub http: reqwest::Client,
}
