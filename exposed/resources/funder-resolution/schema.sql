CREATE TABLE profile_rows (
    "key" VARCHAR PRIMARY KEY,
    name VARCHAR NOT NULL,
    address VARCHAR,
    needs_resolution BOOLEAN NOT NULL
);
CREATE TABLE profile_blocking_keys (
    profile_key VARCHAR NOT NULL,
    ordinal UINTEGER NOT NULL,
    blocking_key VARCHAR NOT NULL
);
CREATE VIEW profiles AS
SELECT p."key", p.name, p.address,
       coalesce(
           (SELECT list(k.blocking_key ORDER BY k.ordinal)
            FROM profile_blocking_keys k WHERE k.profile_key = p."key"),
           []::VARCHAR[]
       ) AS blocking_keys,
       p.needs_resolution
FROM profile_rows p;
