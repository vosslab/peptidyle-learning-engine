-- The final database-owned teaching graph. The coordinator first invokes
-- `prepublication_context.sql`, then the ordinary Pilot publisher, and only
-- then invokes this manifest with the publisher's JSON result. Exact fixture
-- acceptance lives in `live_demo_oracle.sql`; ordinary installation must allow
-- subsequent product lifecycle changes to the Live Demo records.
\ir live_demo.sql
