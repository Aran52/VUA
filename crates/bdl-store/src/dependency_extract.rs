//! Conservative layout-pattern extractor for product-page dependency
//! declarations (proposal 030 "extractor slice", landed as a CAPABILITY only
//! — see the flag-semantics note at the bottom of this module doc).
//!
//! Scope and honesty laws (collab/proposals/030 §1/§2, the 2026-09-23
//! production-seat 1.5.0 respecification note in the 030 inline thread, and
//! the operator batch-178 dispatch):
//!
//! - **Pure parser.** The input is caller-provided product-page CONTENT TEXT.
//!   Nothing here fetches, watches, scans the filesystem, or touches the
//!   network. The crawl surface stays design-blank per the 1.5.0
//!   respecification (a candidate source among others, awaiting its own
//!   accepted proposal).
//! - **Only high-confidence STRUCTURAL layout families are extracted**
//!   (the 030 §1 layout prototypes): author-made prerequisite-environment
//!   heading sections (`explicit_heading`), version-pinned bullet lines
//!   (`bullet`), and bare `com.*` reverse-domain package-name lines
//!   (`one_line`). Everything else — running prose, compressed title claims,
//!   keyed one-line declarations ("Shader: X"), description links — is
//!   honestly NOT extracted: prefer missing over guessing.
//! - **Every extraction lands as an unconfirmed clue.** `confirmed_by_human`
//!   is not even a field of the store write face (`DEFAULT 0`); the only
//!   writer of 1 is `confirm_dependency_resolution`. The parser never fills
//!   a resolution: `resolved_ref_product_id` stays `None` and
//!   `resolution_evidence` stays empty (the 030 §1 sample-3 mislink evidence
//!   keeps identity resolution out of the extraction path entirely).
//! - **`dep_kind` rides the frozen four-value closed set**
//!   (`'shader' | 'tool_package' | 'avatar_base' | 'other'`, BDL v0.2). A
//!   small, documented name lexicon classifies well-known dependencies;
//!   everything unrecognized lands `'other'` (the honest catch-all). Engine
//!   and SDK pins are forced to `'other'` by the frozen dep_kind ruling.
//!   `'avatar_base'` is never produced: recognizing an outfit-to-avatar-base
//!   dependency needs semantic knowledge a layout parser does not have, so
//!   it produces zero such leads rather than guessed ones.
//! - **Verbatim law.** `raw_quote` is the source line trimmed of outer
//!   whitespace — no rewriting; `dep_name` is AS WRITTEN; `version_hint` is
//!   the version substring AS WRITTEN (`2.3.2~`), no normalization.
//!
//! **Flag semantics (capability, not a trigger).** This module is library
//! capability only. NOTHING in the product invokes it automatically: no
//! caller exists outside tests, and wiring it to any real input source
//! (the user's actual BOOTH browsing / Unity usage observation channels),
//! together with the experiment flag (default OFF) and any flag UI, requires
//! a separately accepted proposal per product-boundary 1.5.0 ("automatic
//! compatibility-evidence collection is an experimental feature, off by
//! default"). Capability existing is NOT the feature being enabled.

use crate::bdl_store::{DependencyResolutionEvidence, NewDependencyObservation};

/// `extracted_by` identity stamped on every observation this extractor
/// produces (open vocabulary per the frozen two-confidence-dimensions law:
/// `extraction_method` is the page layout, `extracted_by` is who/what
/// extracted — never merged).
pub const EXTRACTOR_ID: &str = "conservative-layout-extractor-v1";

/// The only `source_span` member this extractor emits: it reads body text
/// the caller hands it and never lifts from titles, subproduct names,
/// images or links.
pub const SPAN_BODY: &str = "body";

/// Extraction-method members this extractor can emit (a subset of the frozen
/// six-value closed set; `prose`/`title`/`link` are deliberately never
/// emitted — those families are not extracted).
pub const METHOD_EXPLICIT_HEADING: &str = "explicit_heading";
pub const METHOD_BULLET: &str = "bullet";
pub const METHOD_ONE_LINE: &str = "one_line";

/// Frozen dep_kind closed-set members (BDL v0.2). `avatar_base` is declared
/// for completeness of the word face but never emitted by this extractor.
pub const DEP_KIND_SHADER: &str = "shader";
pub const DEP_KIND_TOOL_PACKAGE: &str = "tool_package";
pub const DEP_KIND_AVATAR_BASE: &str = "avatar_base";
pub const DEP_KIND_OTHER: &str = "other";

/// Author-made heading words that open a prerequisite-environment section
/// (030 §1 layout prototype: the author's own「〇前提環境」小节). A heading
/// line must be SHORT and must not read like a sentence — a long prose line
/// merely mentioning a heading word is not a heading.
const HEADING_MARKERS: &[&str] = &[
    "前提環境",
    "必要環境",
    "動作環境",
    "必須環境",
    "環境要件",
    "requirements",
];
const HEADING_MAX_CHARS: usize = 24;

/// Bullet markers that open a list line. The dash family needs a following
/// space (`- Unity …`) so leading hyphens inside words stay inert; the
/// dot family (ja/`●`/`•`) does not (`・liltoon 2.3.2~`).
const BULLET_MARKERS_NO_SPACE: &[char] = &['・', '●', '○', '◯', '•', '·'];
const BULLET_MARKERS_NEED_SPACE: &[char] = &['-', '－', '*'];

/// Allowed tails after a version token for a line to stay
/// declaration-shaped. Anything longer or unlisted reads like a sentence
/// (prose) and the line is skipped — prefer missing over guessing.
const ALLOWED_TAILS: &[&str] = &[
    "", "~", "～", "+", "or later", "or newer", "以上", "以降", "対応", "必須", "推奨",
];

/// Well-known dependency names, classified deliberately narrow: only names
/// this seat could defend as unambiguous. Matching is case-insensitive on a
/// normalized haystack (`-`/`_` folded to space). Everything unrecognized
/// lands `other` — the honest catch-all, never a guessed kind.
const SHADER_NAMES: &[&str] = &["liltoon", "poiyomi"];
const TOOL_PACKAGE_NAMES: &[&str] = &["modular avatar", "avatar optimizer"];
/// Engine/SDK names are forced to `other` by the frozen dep_kind ruling
/// (engine/SDK pins land `other` with `version_hint` carrying the pin).
const ENGINE_SDK_NAMES: &[&str] = &["unity", "vrchat", "sdk"];

/// One extracted dependency-declaration LEAD: layout evidence destined for
/// `dependency_observations` through the existing store write face
/// (`record_dependency_observation`). A lead is a clue, never a fact claim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyLead {
    /// Frozen closed-set member (`shader`/`tool_package`/`avatar_base`/
    /// `other`), verbatim; classification source documented in this module.
    pub dep_kind: String,
    /// The dependency's name AS WRITTEN (`liltoon`); no normalization, no
    /// equivalence guessing.
    pub dep_name: String,
    /// The version string AS WRITTEN (`2.3.2~`), or `None` when the line
    /// pins no version.
    pub version_hint: Option<String>,
    /// The source line, verbatim (outer whitespace trimmed only) — no
    /// semantic rewriting.
    pub raw_quote: String,
    /// Always [`SPAN_BODY`] in this extractor.
    pub source_span: String,
    /// `explicit_heading` (inside a prerequisite-environment heading
    /// section), `bullet` (a bullet line outside one), or `one_line` (a bare
    /// `com.*` package-name line) — the page-layout confidence dimension.
    pub extraction_method: String,
    /// Always EMPTY in this extractor: identity resolution is never
    /// auto-populated (030 §1 sample-3 mislink evidence). The field exists
    /// so the lead carries the full evidence shape end to end.
    pub resolution_evidence: Vec<DependencyResolutionEvidence>,
}

/// Extracts dependency-declaration leads from caller-provided product-page
/// content text. Deterministic, pure, allocation-honest: same input, same
/// leads, in encounter order; exact duplicate declarations (same quote,
/// kind, name and version) within one document are emitted once.
pub fn extract_dependency_leads(body_text: &str) -> Vec<DependencyLead> {
    let lines: Vec<&str> = body_text.lines().collect();
    let in_section = compute_section_membership(&lines);

    let mut leads: Vec<DependencyLead> = Vec::new();
    let mut seen: std::collections::HashSet<(String, String, String, Option<String>)> =
        std::collections::HashSet::new();

    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || is_heading_line(trimmed) {
            continue;
        }
        let stripped = strip_bullet(trimmed);
        let bulleted = stripped.is_some();
        let body = stripped.unwrap_or(trimmed);
        let section = in_section[index];

        // Candidate contexts only: inside a prerequisite-environment heading
        // section, a bullet-marked line, or a bare `com.*` package line.
        // Running prose outside these shapes is never extracted.
        let bare_com = bare_com_lead(body);
        if !section && !bulleted && bare_com.is_none() {
            continue;
        }

        let parsed = bare_com.or_else(|| pinned_declaration_lead(body));
        let Some((dep_name, version_hint)) = parsed else {
            continue;
        };

        let extraction_method = if section {
            METHOD_EXPLICIT_HEADING
        } else if bulleted {
            METHOD_BULLET
        } else {
            METHOD_ONE_LINE
        };

        let dep_kind = classify_dep_name(&dep_name).to_string();
        let raw_quote = trimmed.to_string();
        let fingerprint = (
            raw_quote.clone(),
            dep_kind.clone(),
            dep_name.clone(),
            version_hint.clone(),
        );
        if seen.insert(fingerprint) {
            leads.push(DependencyLead {
                dep_kind,
                dep_name,
                version_hint,
                raw_quote,
                source_span: SPAN_BODY.to_string(),
                extraction_method: extraction_method.to_string(),
                resolution_evidence: Vec::new(),
            });
        }
    }
    leads
}

/// Converts a lead into a store write-face row, stamping the caller-supplied
/// observation context. `extracted_by` is [`EXTRACTOR_ID`]; the resolution
/// fields stay empty/`None` (unconfirmed by construction — the row lands as
/// a clue, and only `confirm_dependency_resolution` ever confirms one).
pub fn lead_to_new_observation(
    lead: &DependencyLead,
    product_id: &str,
    observed_at: &str,
    processor_version: &str,
    content_hash: Option<&str>,
    run_id: Option<&str>,
) -> NewDependencyObservation {
    NewDependencyObservation {
        product_id: product_id.to_string(),
        dep_kind: lead.dep_kind.clone(),
        dep_name: lead.dep_name.clone(),
        raw_quote: lead.raw_quote.clone(),
        source_span: lead.source_span.clone(),
        version_hint: lead.version_hint.clone(),
        resolved_ref_product_id: None,
        resolution_evidence: Vec::new(),
        extraction_method: lead.extraction_method.clone(),
        extracted_by: EXTRACTOR_ID.to_string(),
        observed_at: observed_at.to_string(),
        processor_version: processor_version.to_string(),
        content_hash: content_hash.map(str::to_string),
        run_id: run_id.map(str::to_string),
    }
}

/// Classifies a dep name against the narrow, documented lexicon. Engine/SDK
/// names win first (the frozen ruling forces their pins to `other`), then
/// shader, then tool; anything else is `other`.
fn classify_dep_name(dep_name: &str) -> &'static str {
    let normalized = dep_name.to_lowercase().replace(['-', '_'], " ");
    for engine in ENGINE_SDK_NAMES {
        if normalized.contains(engine) {
            return DEP_KIND_OTHER;
        }
    }
    for shader in SHADER_NAMES {
        if normalized.contains(shader) {
            return DEP_KIND_SHADER;
        }
    }
    for tool in TOOL_PACKAGE_NAMES {
        if normalized.contains(tool) {
            return DEP_KIND_TOOL_PACKAGE;
        }
    }
    DEP_KIND_OTHER
}

/// Marks which lines sit inside a prerequisite-environment heading section:
/// from a heading line until the first non-blank line that is neither a
/// bullet nor a declaration-shaped line (running prose closes the section),
/// or the next heading (which opens its own).
fn compute_section_membership(lines: &[&str]) -> Vec<bool> {
    let mut in_section = vec![false; lines.len()];
    let mut open = false;
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if is_heading_line(trimmed) {
            open = true;
            continue;
        }
        if trimmed.is_empty() {
            continue; // blank lines do not close a section
        }
        if !open {
            continue;
        }
        let bullet_stripped = strip_bullet(trimmed);
        let body = bullet_stripped.unwrap_or(trimmed);
        // All THREE structural families count as declaration-shaped here —
        // including a bare `com.*` package line. The main loop extracts a
        // bare com line as a lead (it IS one of the 030 §1 families), so the
        // section scan must treat it the same way: letting it close the
        // section contradicted this function's own contract and demoted the
        // line's method (and silently dropped the bare declaration lines
        // after it) — batch-192 reverse review fix, same fix+pin discipline.
        if bullet_stripped.is_some()
            || pinned_declaration_lead(body).is_some()
            || bare_com_lead(body).is_some()
        {
            in_section[index] = true;
        } else {
            open = false;
        }
    }
    in_section
}

/// Heading detection: SHORT line containing a known prerequisite-heading
/// word, not ending in sentence punctuation (a long prose line mentioning a
/// heading word is not a heading).
fn is_heading_line(trimmed: &str) -> bool {
    if trimmed.is_empty() || trimmed.chars().count() > HEADING_MAX_CHARS {
        return false;
    }
    if ends_with_sentence_punctuation(trimmed) {
        return false;
    }
    let lowered = trimmed.to_lowercase();
    HEADING_MARKERS
        .iter()
        .any(|marker| lowered.contains(marker))
}

fn ends_with_sentence_punctuation(trimmed: &str) -> bool {
    trimmed.chars().last().is_some_and(|c| {
        matches!(
            c,
            '。' | '．' | '.' | '、' | ',' | '！' | '!' | '？' | '?' | '：' | ':'
        )
    })
}

/// Strips a leading bullet marker, returning the remaining text. The dash
/// family requires a following whitespace so `-` inside a leading word stays
/// inert; the dot family does not (`・liltoon 2.3.2~`).
fn strip_bullet(trimmed: &str) -> Option<&str> {
    let mut chars = trimmed.chars();
    let first = chars.next()?;
    if BULLET_MARKERS_NO_SPACE.contains(&first) {
        return Some(chars.as_str().trim_start());
    }
    if BULLET_MARKERS_NEED_SPACE.contains(&first) {
        let rest = chars.as_str();
        if rest.starts_with(' ') || rest.starts_with('\t') || rest.starts_with('\u{3000}') {
            return Some(rest.trim_start());
        }
    }
    None
}

/// A lead from a BARE `com.*` package line: the whole (bullet-stripped) line
/// is the package token, optionally followed by a version pin with an
/// allowed tail. A `com.*` token embedded anywhere else in a line is prose
/// territory and is not extracted.
fn bare_com_lead(body: &str) -> Option<(String, Option<String>)> {
    let (start, end) = find_com_package_token(body)?;
    if start != 0 {
        return None;
    }
    let package = body[..end].to_string();
    let rest = body[end..].trim();
    let version_hint = leading_version_hint(rest)?;
    Some((package, version_hint))
}

/// The version pin allowed after a bare `com.*` package token: a
/// version-shaped token (optionally `v`-prefixed) at the start of the
/// remainder with an allowed tail. Anything else (`… を導入`, `(VPM)`) is
/// not a bare package line and is skipped.
fn leading_version_hint(rest: &str) -> Option<Option<String>> {
    if rest.is_empty() {
        return Some(None);
    }
    let bytes = rest.as_bytes();
    let start = if matches!(bytes[0], b'v' | b'V') && bytes.get(1).is_some_and(u8::is_ascii_digit) {
        1
    } else {
        0
    };
    let (end, clean) = scan_version_token(rest, start)?;
    if !clean || !tail_allowed(&rest[end..]) {
        return None;
    }
    Some(Some(rest[start..end].to_string()))
}

/// A lead from a pinned declaration line: a valid dependency-name region
/// followed by a version-shaped token with an allowed tail
/// (`・liltoon 2.3.2~`, `- Unity 2022.3.22f1`). The name guards are the
/// precision core: ASCII-only, short, letter-bearing, no sentence or field
/// punctuation — prose and keyed one-line declarations are skipped.
fn pinned_declaration_lead(body: &str) -> Option<(String, Option<String>)> {
    let bytes = body.as_bytes();
    for start in 0..bytes.len() {
        if !is_version_candidate_start(bytes, start) {
            continue;
        }
        let Some((version_end, tail_ok)) = scan_version_token(body, start) else {
            continue;
        };
        if !tail_ok || !tail_allowed(&body[version_end..]) {
            continue;
        }
        let mut name_end = start;
        // A `v`/`V` immediately before the version is a version prefix, not
        // part of the name (`・dummyshader v1.2.3` → name `dummyshader`).
        if name_end > 0 && matches!(bytes[name_end - 1], b'v' | b'V') {
            let before_v = name_end
                .checked_sub(2)
                .map(|i| bytes[i].is_ascii_alphanumeric())
                .unwrap_or(false);
            if !before_v {
                name_end -= 1;
            }
        }
        let name = body[..name_end].trim();
        if !valid_dep_name(name) {
            continue;
        }
        return Some((name.to_string(), Some(body[start..version_end].to_string())));
    }
    None
}

/// A version-candidate start: an ASCII digit not inside an alphanumeric run,
/// allowing a single `v`/`V` prefix (`v2.3.2`).
fn is_version_candidate_start(bytes: &[u8], start: usize) -> bool {
    if !bytes[start].is_ascii_digit() {
        return false;
    }
    match start.checked_sub(1) {
        None => true,
        Some(prev) => {
            if !bytes[prev].is_ascii_alphanumeric() {
                return true;
            }
            matches!(bytes[prev], b'v' | b'V')
                && prev
                    .checked_sub(1)
                    .map(|p| !bytes[p].is_ascii_alphanumeric())
                    .unwrap_or(true)
        }
    }
}

/// Scans a version-shaped token from `start`: digits (`.` digits)+ then an
/// alphanumeric run (`f1`), then one optional trailing `~`/`～`. Returns the
/// exclusive end and whether the boundary after the token is clean (no
/// alphanumeric or `.`/`_` continues it).
fn scan_version_token(text: &str, start: usize) -> Option<(usize, bool)> {
    let bytes = text.as_bytes();
    let mut end = start;
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    let mut groups = 0;
    while end < bytes.len() && bytes[end] == b'.' {
        let mut digits = end + 1;
        let group_start = digits;
        while digits < bytes.len() && bytes[digits].is_ascii_digit() {
            digits += 1;
        }
        if digits == group_start {
            break;
        }
        end = digits;
        groups += 1;
    }
    if groups == 0 {
        return None;
    }
    while end < bytes.len() && bytes[end].is_ascii_alphanumeric() {
        end += 1;
    }
    if end < bytes.len() && matches!(bytes[end], b'~') {
        end += 1;
    } else if end + "～".len() <= bytes.len() && text[end..].starts_with('～') {
        end += "～".len();
    }
    let clean = match bytes.get(end) {
        None => true,
        Some(&b) => !(b.is_ascii_alphanumeric() || b == b'.' || b == b'_'),
    };
    Some((end, clean))
}

fn tail_allowed(rest: &str) -> bool {
    ALLOWED_TAILS.contains(&rest.trim())
}

/// The dep-name guards: non-empty, short, ASCII-only (a ja-script name
/// region is prose evidence — the surveyed dependency names are proper
/// nouns), letter-bearing, and free of sentence/field punctuation (`.` `:`
/// and friends). Punctuation-bearing lines read like sentences or keyed
/// declarations and are skipped — prefer missing over guessing.
fn valid_dep_name(name: &str) -> bool {
    if name.is_empty() || name.chars().count() > 48 {
        return false;
    }
    let mut has_letter = false;
    for c in name.chars() {
        if c.is_ascii_alphanumeric() || matches!(c, ' ' | '-' | '_' | '+' | '&' | '\'' | '/') {
            has_letter |= c.is_ascii_alphabetic();
        } else {
            return false;
        }
    }
    has_letter
}

/// Finds a `com.*` reverse-domain package token (`com.vendor.name`, at
/// least three segments total), bounded so it is not part of a longer word.
fn find_com_package_token(text: &str) -> Option<(usize, usize)> {
    let bytes = text.as_bytes();
    let mut search_from = 0;
    while let Some(found) = text[search_from..].find("com.") {
        let start = search_from + found;
        search_from = start + 1;
        let prev_clean = match start.checked_sub(1) {
            None => true,
            Some(prev) => {
                !(bytes[prev].is_ascii_alphanumeric()
                    || bytes[prev] == b'.'
                    || bytes[prev] == b'-'
                    || bytes[prev] == b'_')
            }
        };
        if !prev_clean {
            continue;
        }
        if let Some(end) = scan_com_segments(bytes, start + "com.".len()) {
            return Some((start, end));
        }
    }
    None
}

/// Parses ≥2 dot-separated segments of `[A-Za-z0-9][A-Za-z0-9_-]*` starting
/// right after `com.`; returns the exclusive end if at least two full
/// segments parse and the boundary is clean.
fn scan_com_segments(bytes: &[u8], mut cursor: usize) -> Option<usize> {
    let mut segments = 0;
    loop {
        if cursor >= bytes.len() || !bytes[cursor].is_ascii_alphanumeric() {
            return None;
        }
        while cursor < bytes.len()
            && (bytes[cursor].is_ascii_alphanumeric()
                || bytes[cursor] == b'-'
                || bytes[cursor] == b'_')
        {
            cursor += 1;
        }
        segments += 1;
        if cursor < bytes.len() && bytes[cursor] == b'.' {
            cursor += 1;
            continue;
        }
        break;
    }
    if segments < 2 {
        return None;
    }
    Some(cursor)
}
