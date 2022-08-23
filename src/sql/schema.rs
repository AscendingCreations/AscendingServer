diesel::table! {
    use diesel::sql_types::*;
    achievements (uid) {
        uid -> BigInt,
        daykills -> Integer,
        nightkills -> Integer,
        survivekill -> Integer,
        revivals -> Integer,
        deaths -> Integer,
        npckilled -> Array<Integer>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    equipment (uid) {
        uid -> BigInt,
        id -> SmallInt,
        num -> Integer,
        val -> SmallInt,
        itemlevel -> SmallInt,
        data -> Array<SmallInt>,
    }
}

diesel::table! {
    use diesel::sql_types::*;
    invitems (uid) {
        uid -> BigInt,
        id -> SmallInt,
        num -> Integer,
        val -> SmallInt,
        itemlevel -> SmallInt,
        data -> Array<SmallInt>,
    }
}

diesel::table! {
    use crate::{sql, gametypes};
    use diesel::sql_types::*;
    players (uid) {
        uid -> BigInt,
        name -> Text,
        address -> Text,
        created_on -> Timestamp,
        sprite -> SmallInt,
        spawn -> gametypes::PosType,
        itemtimer -> BigInt,
        vals -> BigInt,
        data -> Array<BigInt>,
        access -> sql::UserAccessMapping,
        passresetcode -> Nullable<Text>,
        pos -> gametypes::PosType,
        vital -> Array<Integer>,
        deathtimer -> BigInt,
        indeath -> Bool,
        isonline -> Bool,
        email -> Text,
        password -> Text,
        username -> Text,
        level -> Integer,
        levelexp -> BigInt,
        resetcount -> SmallInt,
        pk -> Bool,
    }
}

diesel::table! {
    use crate::{sql, gametypes};
    use diesel::sql_types::*;
    #[sql_name = "players"]
    player_ret (uid) {
        uid -> BigInt,
        name -> Text,
        address -> Text,
        sprite -> SmallInt,
        spawn -> gametypes::PosType,
        itemtimer -> BigInt,
        vals -> BigInt,
        data -> Array<BigInt>,
        access -> sql::UserAccessMapping,
        pos -> gametypes::PosType,
        vital -> Array<Integer>,
        deathtimer -> BigInt,
        indeath -> Bool,
        level -> Integer,
        levelexp -> BigInt,
        resetcount -> SmallInt,
        pk -> Bool,
    }
}

diesel::table! {
    use crate::sql;
    use diesel::sql_types::*;
    logs (serverid) {
        serverid -> SmallInt,
        userid -> BigInt,
        logtype -> sql::LogTypeMapping,
        message -> Text,
        ipaddress -> Text,
    }
}
