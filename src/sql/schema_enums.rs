#[rustfmt::skip]
pub const LOGTYPE_SCHEMA: &str = "
DO $$ BEGIN
    CREATE TYPE public.\"LogType\" AS ENUM
        ('Login', 'Logout', 'Item', 'Warning', 'Error');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;
";

#[rustfmt::skip]
pub const LOGTYPE_SCHEMA_ALTER: &str = "
ALTER TYPE public.\"LogType\"
    OWNER TO postgres;
";

#[rustfmt::skip]
pub const USERACCESS_SCHEMA: &str = "
DO $$ BEGIN
    CREATE TYPE public.\"UserAccess\" AS ENUM
        ('None', 'Monitor', 'Admin');
EXCEPTION
        WHEN duplicate_object THEN null;
END $$;
";

#[rustfmt::skip]
pub const USERACCESS_SCHEMA_ALTER: &str = "
ALTER TYPE public.\"UserAccess\"
    OWNER TO postgres;
";
