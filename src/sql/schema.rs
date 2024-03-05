#[rustfmt::skip]
pub const PLAYER_SEQ_SCHEMA: &str = "
CREATE SEQUENCE IF NOT EXISTS public.player_uid_seq
    INCREMENT 1
    START 1
    MINVALUE 1
    MAXVALUE 9223372036854775807
    CACHE 1;
";

#[rustfmt::skip]
pub const PLAYER_SEQ_SCHEMA_ALTER: &str = "
ALTER SEQUENCE public.player_uid_seq
    OWNER TO postgres;
";

#[rustfmt::skip]
pub const PLAYER_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS public.player
(
    uid bigint NOT NULL DEFAULT nextval('player_uid_seq'::regclass),
    username text COLLATE pg_catalog.\"default\" NOT NULL,
    address text COLLATE pg_catalog.\"default\" NOT NULL,
    password text COLLATE pg_catalog.\"default\" NOT NULL,
    itemtimer bigint NOT NULL,
    deathtimer bigint NOT NULL,
    vals bigint NOT NULL,
    spawn \"Position\" NOT NULL,
    pos \"Position\" NOT NULL,
    email text COLLATE pg_catalog.\"default\" NOT NULL,
    sprite smallint NOT NULL,
    indeath boolean NOT NULL,
    level integer NOT NULL,
    levelexp bigint NOT NULL,
    resetcount smallint NOT NULL,
    pk boolean NOT NULL,
    data bigint[] NOT NULL,
    vital integer[] NOT NULL,
    passresetcode text COLLATE pg_catalog.\"default\",
    access \"UserAccess\" NOT NULL,
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
pub const PLAYER_SCHEMA_ALTER: &str = "
ALTER TABLE IF EXISTS public.player
    OWNER to postgres;
";

#[rustfmt::skip]
pub const EQUIPMENT_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS public.equipment
(
    uid bigint NOT NULL,
    id smallint NOT NULL,
    num integer NOT NULL,
    val smallint NOT NULL,
    itemlevel smallint NOT NULL,
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
    uid bigint NOT NULL,
    id smallint NOT NULL,
    num integer NOT NULL,
    val smallint NOT NULL,
    itemlevel smallint NOT NULL,
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
pub const LOGS_SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS public.logs
(
    serverid smallint NOT NULL,
    userid bigint NOT NULL,
    logtype \"LogType\" NOT NULL,
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
    OWNER to postgres
";
