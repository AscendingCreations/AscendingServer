#[rustfmt::skip]
pub const MAP_POSITION_SCHEMA: &str = "
DO $$ BEGIN
	CREATE TYPE public.\"map_position\" AS
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
ALTER TYPE public.\"map_position\"
    OWNER TO postgres;
";

#[rustfmt::skip]
pub const POSITION_SCHEMA: &str = "
DO $$ BEGIN
	CREATE TYPE public.\"location\" AS
	(
		x integer,
		y integer,
		map map_position
	);
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;
";

#[rustfmt::skip]
pub const POSITION_SCHEMA_ALTER: &str = "
ALTER TYPE public.\"location\"
    OWNER TO postgres;
";
