use std::sync::Arc;
use thanksgivings_db::Db;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
}
