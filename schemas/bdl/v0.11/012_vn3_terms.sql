-- IN-2: source tracing and explicit empty/missing-description distinction in terms.getProduct.
CREATE TABLE product_terms_snapshots (
  product_id TEXT PRIMARY KEY REFERENCES products(product_id),
  observed_at TEXT NOT NULL,
  problem TEXT CHECK(problem IS NULL OR problem='source_limit')
) STRICT;

-- Source identity, latest refresh state and cache pointer serve both terms read methods.
CREATE TABLE terms_sources (
  source_id TEXT PRIMARY KEY,
  source_kind TEXT NOT NULL CHECK(source_kind IN ('drive','docs','vn3','link')),
  source_url TEXT NOT NULL,
  google_file_id TEXT CHECK(google_file_id IS NULL OR (length(google_file_id) BETWEEN 1 AND 200 AND google_file_id NOT GLOB '*[^a-zA-Z0-9_-]*')),
  status TEXT NOT NULL CHECK(status IN ('pending','link_only','parsed','unrecognized','unsupported','not_public','failed')),
  fetched_at TEXT,
  sha256 TEXT CHECK(sha256 IS NULL OR (length(sha256)=64 AND sha256 NOT GLOB '*[^0-9a-f]*')),
  parser_version TEXT,
  problem TEXT,
  CHECK((source_kind IN ('drive','docs'))=(google_file_id IS NOT NULL)),
  CHECK(google_file_id IS NOT NULL OR status='link_only'),
  CHECK(google_file_id IS NULL OR source_id=source_kind||':'||google_file_id)
) STRICT;
CREATE TABLE product_terms_sources (
  product_id TEXT NOT NULL REFERENCES products(product_id),
  source_id TEXT NOT NULL REFERENCES terms_sources(source_id),
  PRIMARY KEY(product_id, source_id)
) STRICT;

-- Content identity + parser version answer cache reuse; remaining fields are document display evidence.
CREATE TABLE vn3_documents (
  source_id TEXT NOT NULL REFERENCES terms_sources(source_id),
  sha256 TEXT NOT NULL CHECK(length(sha256)=64 AND sha256 NOT GLOB '*[^0-9a-f]*'),
  parser_version TEXT NOT NULL,
  status TEXT NOT NULL CHECK(status IN ('parsed','unrecognized','unsupported')),
  generator_version TEXT,
  vn3_version TEXT,
  language TEXT CHECK(language IN ('ja','en')),
  content_title TEXT,
  licensor TEXT,
  credit_notation TEXT,
  hashtags TEXT NOT NULL CHECK(json_valid(hashtags) AND json_type(hashtags)='array'),
  agreement_version TEXT,
  special_note TEXT,
  warnings TEXT NOT NULL CHECK(json_valid(warnings) AND json_type(warnings)='array'),
  CHECK(status!='parsed' OR (generator_version IS NOT NULL AND generator_version IN ('1.10','1.11') AND vn3_version IS NOT NULL AND vn3_version='1.10' AND language IS NOT NULL)),
  PRIMARY KEY(source_id,sha256,parser_version)
) STRICT;

-- Every row supplies a decision input and its original detailed/summary evidence.
CREATE TABLE vn3_document_items (
  source_id TEXT NOT NULL,
  sha256 TEXT NOT NULL,
  parser_version TEXT NOT NULL,
  item TEXT NOT NULL CHECK(length(item)=1 AND item BETWEEN 'A' AND 'X'),
  term_key TEXT NOT NULL CHECK(substr(term_key,1,6)='vn3.'||item||'.' AND length(term_key)>6 AND substr(term_key,7) NOT GLOB '*[^a-z_]*'),
  value TEXT NOT NULL CHECK(value IN ('permitted','prohibited','contact_licensor','unknown')),
  condition TEXT CHECK(condition IN ('content_rating_required','between_users_only','commercial_and_non_profit','credit_preferred','modified_distinguishable','non_profit_only','private_use_only','private_use_exception','limited_publication_only')),
  raw_text TEXT NOT NULL,
  summary_raw_text TEXT,
  CHECK(value!='unknown' OR condition IS NULL),
  PRIMARY KEY(source_id,sha256,parser_version,item),
  FOREIGN KEY(source_id,sha256,parser_version) REFERENCES vn3_documents(source_id,sha256,parser_version)
) STRICT;
CREATE TRIGGER vn3_items_require_recognized_document BEFORE INSERT ON vn3_document_items
WHEN (SELECT status FROM vn3_documents WHERE source_id=NEW.source_id AND sha256=NEW.sha256 AND parser_version=NEW.parser_version)!='parsed'
BEGIN SELECT RAISE(ABORT,'items require a recognized VN3 document'); END;
UPDATE bdl_meta SET value='0.11' WHERE key='format_version' AND value='0.10';
