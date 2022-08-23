use crate::{gametypes::*, sql::*};

#[derive(Debug, Queryable, Insertable)]
#[table_name = "logs"]
pub struct PGLog {
    serverid: i16,
    userid: i64,
    logtype: LogType,
    message: String,
    ipaddress: String,
}

impl PGLog {
    pub fn new(
        serverid: i16,
        userid: i64,
        logtype: LogType,
        message: String,
        ipaddress: String,
    ) -> PGLog {
        PGLog {
            serverid,
            userid,
            logtype,
            message,
            ipaddress,
        }
    }
}
