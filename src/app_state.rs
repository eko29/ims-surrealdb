
use surrealdb::Surreal;
use surrealdb::engine::remote::ws::Client;
use std::sync::Arc;

type Db = Surreal<Client>;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Db>,
}
