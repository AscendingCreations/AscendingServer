#[rustfmt::skip]
pub const MAP_POSITION_SCHEMA: &str = "
DO $$ BEGIN
	CREATE TYPE public.\"MapPosition\" AS
	(
		x integer,
		y integer,
		\"group\" integer
	);
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;
";

#[rustfmt::skip]
pub const MAP_POSITION_SCHEMA_ALTER: &str = "
ALTER TYPE public.\"MapPosition\"
    OWNER TO postgres;
";

#[rustfmt::skip]
pub const POSITION_SCHEMA: &str = "
DO $$ BEGIN
	CREATE TYPE public.\"Position\" AS
	(
		x integer,
		y integer,
		map \"MapPosition\"
	);
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;
";

#[rustfmt::skip]
pub const POSITION_SCHEMA_ALTER: &str = "
ALTER TYPE public.\"Position\"
    OWNER TO postgres;
";
