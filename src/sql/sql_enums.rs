#[derive(diesel::sql_types::SqlType)]
#[diesel(postgres_type(name = "combat_stat"))]
pub struct CombatStatMapping;

#[derive(diesel::sql_types::SqlType)]
#[diesel(postgres_type(name = "user_access"))]
pub struct UserAccessMapping;

#[derive(diesel::sql_types::SqlType)]
#[diesel(postgres_type(name = "vital_types"))]
pub struct VitalTypesMapping;

#[derive(diesel::sql_types::SqlType)]
#[diesel(postgres_type(name = "stat_type"))]
pub struct StatTypeMapping;

#[derive(diesel::sql_types::SqlType)]
#[diesel(postgres_type(name = "log_type"))]
pub struct LogTypeMapping;
