-- The final database-owned teaching graph.  The coordinator first invokes
-- `prepublication_context.sql`, then the ordinary Pilot publisher, and only
-- then invokes this manifest with the publisher's JSON result.
\ir live_demo.sql
\ir live_demo_oracle.sql
