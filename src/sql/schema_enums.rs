#[rustfmt::skip]
pub const LOGTYPE_SCHEMA: &str = "
CREATE TYPE public.\"LogType\" AS ENUM
    ('Login', 'Logout', 'Item', 'Warning', 'Error');

ALTER TYPE public.\"LogType\"
    OWNER TO postgres;
";

#[rustfmt::skip]
pub const USERACCESS_SCHEMA: &str = "
CREATE TYPE public.\"UserAccess\" AS ENUM
    ('None', 'Monitor', 'Admin');

ALTER TYPE public.\"UserAccess\"
    OWNER TO postgres;
";
