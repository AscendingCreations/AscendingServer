#[rustfmt::skip]
pub const MAP_POSITION_SCHEMA: &str = "
CREATE TYPE public.\"MapPosition\" AS
(
	x integer,
	y integer,
	\"group\" integer
);

ALTER TYPE public.\"MapPosition\"
    OWNER TO postgres;
";

#[rustfmt::skip]
pub const POSITION_SCHEMA: &str = "
CREATE TYPE public.\"Position\" AS
(
	x integer,
	y integer,
	map \"MapPosition\"
);

ALTER TYPE public.\"Position\"
    OWNER TO postgres;
";
