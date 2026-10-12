-- Last captured author-description links, including an explicitly empty set.
-- Link targets need not be catalogued or owned by the account.
CREATE TABLE product_description_links (
  product_id TEXT PRIMARY KEY REFERENCES products(product_id),
  target_ids TEXT NOT NULL CHECK(json_valid(target_ids) AND json_type(target_ids)='array'),
  content_hash TEXT NOT NULL,
  observed_at TEXT NOT NULL
) STRICT;
UPDATE bdl_meta SET value='0.10' WHERE key='format_version' AND value='0.9';
