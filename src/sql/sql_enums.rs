#[derive(diesel::sql_types::SqlType)]
#[diesel(postgres_type(name = "user_access"))]
pub struct UserAccessMapping;

#[derive(diesel::sql_types::SqlType)]
#[diesel(postgres_type(name = "vital_types"))]
pub struct VitalTypesMapping;

#[derive(diesel::sql_types::SqlType)]
#[diesel(postgres_type(name = "log_type"))]
pub struct LogTypeMapping;
