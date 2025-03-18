#[rustfmt::skip]
pub const PG_UUID: &str = "
CREATE OR REPLACE FUNCTION
  uuid_generate_v7()
RETURNS
  uuid
LANGUAGE
  plpgsql
PARALLEL SAFE
AS $$
  DECLARE
    -- The current UNIX timestamp in milliseconds
    unix_time_ms CONSTANT bytea NOT NULL DEFAULT substring(int8send((extract(epoch FROM clock_timestamp()) * 1000)::bigint) from 3);

    -- The buffer used to create the UUID, starting with the UNIX timestamp and followed by random bytes
    buffer                bytea NOT NULL DEFAULT unix_time_ms || gen_random_bytes(10);
  BEGIN
    -- Set most significant 4 bits of 7th byte to 7 (for UUID v7), keeping the last 4 bits unchanged
    buffer = set_byte(buffer, 6, (b'0111' || get_byte(buffer, 6)::bit(4))::bit(8)::int);

    -- Set most significant 2 bits of 9th byte to 2 (the UUID variant specified in RFC 4122), keeping the last 6 bits unchanged
    buffer = set_byte(buffer, 8, (b'10'   || get_byte(buffer, 8)::bit(6))::bit(8)::int);

    RETURN encode(buffer, 'hex');
  END
$$
;
";

pub const PG_CRYPTO_EXTENSION: &str = "
create extension if not exists pgcrypto;";

#[rustfmt::skip]
pub const LOGS_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS public.logs
(
    serverid smallint NOT NULL,
    userid uuid NOT NULL,
    logtype \"log_type\" NOT NULL,
    message text COLLATE pg_catalog.\"default\" NOT NULL,
    ipaddress text COLLATE pg_catalog.\"default\" NOT NULL
)

WITH (
    FILLFACTOR = 70
)
TABLESPACE pg_default;
";

#[rustfmt::skip]
pub const LOGS_SCHEMA_ALTER: &str = "
ALTER TABLE IF EXISTS public.logs
    OWNER to server
";

#[rustfmt::skip]
pub const ACCOUNT_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS public.account
(
    uid uuid NOT NULL DEFAULT uuid_generate_v7(),
    username text COLLATE pg_catalog.\"default\" NOT NULL,
    address text COLLATE pg_catalog.\"default\" NOT NULL,
    password text COLLATE pg_catalog.\"default\" NOT NULL,
    email text COLLATE pg_catalog.\"default\" NOT NULL,
    passresetcode text COLLATE pg_catalog.\"default\",
    useraccess \"user_access\" NOT NULL,
    created_on timestamp with time zone NOT NULL DEFAULT now(),
    CONSTRAINT player_pkey PRIMARY KEY (uid),
    CONSTRAINT email UNIQUE (email),
    CONSTRAINT username UNIQUE (username)
)

WITH (
    FILLFACTOR = 70
)
TABLESPACE pg_default;
";

#[rustfmt::skip]
pub const ACCOUNT_SCHEMA_ALTER: &str = "
ALTER TABLE IF EXISTS public.account
    OWNER to server;
";

#[rustfmt::skip]
pub const GENERAL_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS public.general
(
    uid uuid NOT NULL,
    sprite smallint NOT NULL,
    money bigint NOT NULL,
    resetcount smallint NOT NULL,
    itemtimer bigint NOT NULL,
    deathtimer bigint NOT NULL
)

WITH (
    FILLFACTOR = 70
)
TABLESPACE pg_default;
";

#[rustfmt::skip]
pub const GENERAL_SCHEMA_ALTER: &str = "
ALTER TABLE IF EXISTS public.general
    OWNER to server;
";

#[rustfmt::skip]
pub const LOCATION_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS public.locations
(
    uid uuid NOT NULL,
    spawn \"location\" NOT NULL,
    pos \"location\" NOT NULL,
    dir smallint NOT NULL
)

WITH (
    FILLFACTOR = 70
)
TABLESPACE pg_default;
";

#[rustfmt::skip]
pub const LOCATION_SCHEMA_ALTER: &str = "
ALTER TABLE IF EXISTS public.locations
    OWNER to server;
";

#[rustfmt::skip]
pub const COMBAT_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS public.combat
(
    uid uuid NOT NULL,
    indeath boolean NOT NULL,
    level integer NOT NULL,
    levelexp bigint NOT NULL,
    pk boolean NOT NULL,
    vital integer[] NOT NULL,
    vital_max integer[] NOT NULL
)

WITH (
    FILLFACTOR = 70
)
TABLESPACE pg_default;
";

#[rustfmt::skip]
pub const COMBAT_SCHEMA_ALTER: &str = "
ALTER TABLE IF EXISTS public.combat
    OWNER to server;
";

#[rustfmt::skip]
pub const EQUIPMENT_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS public.equipment
(
    uid uuid NOT NULL,
    id smallint NOT NULL,
    num integer NOT NULL,
    val smallint NOT NULL,
    level smallint NOT NULL,
    data smallint[] NOT NULL
)

WITH (
    FILLFACTOR = 70
)
TABLESPACE pg_default;
";

#[rustfmt::skip]
pub const EQUIPMENT_SCHEMA_ALTER: &str = "
ALTER TABLE IF EXISTS public.equipment
    OWNER to postgres;
";

#[rustfmt::skip]
pub const INVENTORY_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS public.inventory
(
    uid uuid NOT NULL,
    id smallint NOT NULL,
    num integer NOT NULL,
    val smallint NOT NULL,
    level smallint NOT NULL,
    data smallint[] NOT NULL
)

WITH (
    FILLFACTOR = 70
)
TABLESPACE pg_default;
";

#[rustfmt::skip]
pub const INVENTORY_SCHEMA_ALTER: &str = "
ALTER TABLE IF EXISTS public.inventory
    OWNER to postgres;
";

#[rustfmt::skip]
pub const STORAGE_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS public.storage
(
    uid uuid NOT NULL,
    id smallint NOT NULL,
    num integer NOT NULL,
    val smallint NOT NULL,
    level smallint NOT NULL,
    data smallint[] NOT NULL
)

WITH (
    FILLFACTOR = 70
)
TABLESPACE pg_default;
";

#[rustfmt::skip]
pub const STORAGE_SCHEMA_ALTER: &str = "
ALTER TABLE IF EXISTS public.storage
    OWNER to postgres;
";
