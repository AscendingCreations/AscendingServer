#[derive(Debug, Clone, Default)]
pub struct PlayerEntity {
    pub account: Account,
}

#[derive(Clone, Debug, Default)]
pub struct Account {
    pub username: String,
    pub passresetcode: Option<String>,
    pub id: i64,
}
