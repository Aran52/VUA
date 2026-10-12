-- Cards are presentation records. Original files, identities and references survive.
CREATE TABLE removed_library_cards (
  kind TEXT NOT NULL CHECK(kind IN ('entry','product')),
  target_id TEXT NOT NULL,
  removed_at TEXT NOT NULL,
  PRIMARY KEY(kind,target_id)
) STRICT;
CREATE TABLE removed_library_card_commands (
  command_id TEXT PRIMARY KEY,
  fingerprint TEXT NOT NULL,
  result_json TEXT NOT NULL
) STRICT;
UPDATE bdl_meta SET value='0.9' WHERE key='format_version' AND value='0.8';
