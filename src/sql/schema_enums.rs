#[rustfmt::skip]
pub const LOGTYPE_SCHEMA: &str = "
DO $$ BEGIN
    CREATE TYPE public.\"log_type\" AS ENUM
        ('Login', 'Logout', 'Item', 'Warning', 'Error');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;
";

#[rustfmt::skip]
pub const LOGTYPE_SCHEMA_ALTER: &str = "
ALTER TYPE public.\"log_type\"
    OWNER TO postgres;
";

#[rustfmt::skip]
pub const USERACCESS_SCHEMA: &str = "
DO $$ BEGIN
    CREATE TYPE public.\"user_access\" AS ENUM
        ('None', 'Monitor', 'Admin');
EXCEPTION
        WHEN duplicate_object THEN null;
END $$;
";

#[rustfmt::skip]
pub const USERACCESS_SCHEMA_ALTER: &str = "
ALTER TYPE public.\"user_access\"
    OWNER TO postgres;
";
