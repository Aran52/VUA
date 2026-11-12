//! Conservative layout-pattern extractor tests (proposal 030 extractor
//! slice). ALL fixtures are synthetic: layout SHAPES follow the 030 §1
//! prototypes (author-made heading sections, version-pinned bullet lines,
//! `com.*` reverse-domain package lines) while every text is composed for
//! this suite — classifier lexicon names (liltoon, Modular Avatar) are the
//! code's own word list, all version strings are invented, and unknown
//! dependencies use dummy names. Zero real page content, zero network, zero
//! file access.
//!
//! What the suite pins:
//! 1. the three structural families extract, with frozen closed-set members,
//!    verbatim quotes and as-written version hints;
//! 2. prose, keyed one-line declarations and compat-style bullets are
//!    honestly NOT extracted (prefer missing over guessing), and
//!    `avatar_base` is never produced;
//! 3. leads land through the EXISTING store write face
//!    (`record_dependency_observation`) as unconfirmed clues
//!    (`confirmed_by_human` 0, no resolution), verbatim round-trip;
//! 4. the extractor is deterministic and de-duplicates one document's
//!    identical declarations.

use vua_bdl_store::{
    extract_dependency_leads, lead_to_new_observation, BdlStore, DependencyLead, EXTRACTOR_ID,
    METHOD_BULLET, METHOD_EXPLICIT_HEADING, METHOD_ONE_LINE, SPAN_BODY,
};

/// Prototype shape: an author-made prerequisite-environment heading section
/// with version-pinned bullet lines, surrounded by plain description prose
/// (030 §1 sample-1 layout; all word faces synthetic).
const HEADING_SECTION_FIXTURE: &str = "\
かわいいグローステッキのセットです。導入方法は同梱説明書を
ご覧ください。

〇前提環境
・liltoon 1.2.3~
・VRChatSDK - Avatars 3.7.10~

不明な点はお問い合わせください。";

#[test]
fn explicit_heading_section_extracts_pinned_bullet_lines() {
    let leads = extract_dependency_leads(HEADING_SECTION_FIXTURE);
    assert_eq!(
        leads.len(),
        2,
        "exactly the two pinned lines under the heading; all prose skipped: {leads:?}"
    );

    let shader = &leads[0];
    assert_eq!(shader.dep_kind, "shader");
    assert_eq!(shader.dep_name, "liltoon");
    assert_eq!(shader.version_hint.as_deref(), Some("1.2.3~"));
    assert_eq!(shader.raw_quote, "・liltoon 1.2.3~");
    assert_eq!(shader.source_span, SPAN_BODY);
    assert_eq!(shader.extraction_method, METHOD_EXPLICIT_HEADING);
    assert!(shader.resolution_evidence.is_empty());

    let engine = &leads[1];
    // Engine/SDK pins land `other` with the pin in version_hint (the frozen
    // dep_kind ruling).
    assert_eq!(engine.dep_kind, "other");
    assert_eq!(engine.dep_name, "VRChatSDK - Avatars");
    assert_eq!(engine.version_hint.as_deref(), Some("3.7.10~"));
    assert_eq!(engine.raw_quote, "・VRChatSDK - Avatars 3.7.10~");
    assert_eq!(engine.extraction_method, METHOD_EXPLICIT_HEADING);
}

#[test]
fn bullet_lines_outside_sections_use_the_bullet_method() {
    // Dash-marker bullets with a space (030 §1 sample-3 line shape) and
    // dot-marker bullets, all OUTSIDE any heading section.
    let fixture = "\
必要なもの一覧
- Unity 2022.3.22f1
・Modular Avatar 1.4.0
・CoolGlow 2.0対応";

    let leads = extract_dependency_leads(fixture);
    assert_eq!(leads.len(), 3, "{leads:?}");

    assert_eq!(leads[0].dep_kind, "other", "Unity is forced to other");
    assert_eq!(leads[0].dep_name, "Unity");
    assert_eq!(leads[0].version_hint.as_deref(), Some("2022.3.22f1"));
    assert_eq!(leads[0].extraction_method, METHOD_BULLET);

    assert_eq!(leads[1].dep_kind, "tool_package", "known tool lexicon hit");
    assert_eq!(leads[1].dep_name, "Modular Avatar");
    assert_eq!(leads[1].version_hint.as_deref(), Some("1.4.0"));
    assert_eq!(leads[1].extraction_method, METHOD_BULLET);

    // A name that merely SAYS it is a shader stays `other`: the narrow
    // lexicon never guesses kinds for unrecognized names.
    assert_eq!(leads[2].dep_kind, "other");
    assert_eq!(leads[2].dep_name, "CoolGlow");
    assert_eq!(leads[2].version_hint.as_deref(), Some("2.0"));
}

#[test]
fn bare_com_package_lines_extract_as_one_line_clues() {
    let fixture = "\
VPMで導入してください
com.example.coolglow 1.2.0
・com.example.coolglow.addon
com.example.toolkit v0.9以上";

    let leads = extract_dependency_leads(fixture);
    assert_eq!(leads.len(), 3, "{leads:?}");

    assert_eq!(leads[0].dep_name, "com.example.coolglow");
    assert_eq!(leads[0].version_hint.as_deref(), Some("1.2.0"));
    assert_eq!(leads[0].extraction_method, METHOD_ONE_LINE);
    assert_eq!(
        leads[0].dep_kind, "other",
        "unrecognized package: other, never guessed"
    );

    assert_eq!(leads[1].dep_name, "com.example.coolglow.addon");
    assert_eq!(leads[1].version_hint, None);
    assert_eq!(
        leads[1].extraction_method, METHOD_BULLET,
        "bulleted com line keeps the bullet method"
    );

    assert_eq!(leads[2].dep_name, "com.example.toolkit");
    assert_eq!(leads[2].version_hint.as_deref(), Some("0.9"));
    assert_eq!(leads[2].extraction_method, METHOD_ONE_LINE);
}

#[test]
fn prose_and_non_structural_lines_are_never_extracted() {
    // Every line here carries dependency-ish content in a NON-structural
    // shape (running prose, a bulleted sentence, a ja-script name region, a
    // compat-style bullet without any version pin, a keyed declaration) —
    // all of it must be skipped: 宁缺勿猜.
    let fixture = "\
本ギミックはdummyglowのカスタムパラメータとして作動します。
・この商品はdummyglow 1.0.0を前提としております
・この衣装はdummyglow 1.2.3
・『SampleAvatar』対応
使用シェーダー:dummyglow 2.1";
    assert!(
        extract_dependency_leads(fixture).is_empty(),
        "no lead may come out of prose or keyed/ja-script lines"
    );

    // Compat-style avatar references produce no leads at all — and the
    // extractor therefore never emits `avatar_base` (recognizing an
    // outfit-to-base dependency is semantic, not structural).
    let compat_fixture = "◎対応アバター\n・『SampleAvatar』対応\n・『AnotherAvatar』対応";
    let leads = extract_dependency_leads(compat_fixture);
    assert!(leads.is_empty(), "{leads:?}");
    assert!(leads.iter().all(|lead| lead.dep_kind != "avatar_base"));
}

#[test]
fn degenerate_inputs_stay_honestly_empty() {
    assert!(extract_dependency_leads("").is_empty());
    assert!(extract_dependency_leads("\n\n   \n\t\n").is_empty());
    assert!(extract_dependency_leads("ただの説明文です。依頼の必要はありません。").is_empty());
}

#[test]
fn extraction_is_deterministic_and_deduplicates_one_document() {
    let fixture = "〇前提環境\n・liltoon 1.2.3~\n・liltoon 1.2.3~\n・Modular Avatar 1.4.0";
    let first = extract_dependency_leads(fixture);
    let second = extract_dependency_leads(fixture);
    assert_eq!(first, second, "same input, same leads");
    assert_eq!(
        first.iter().filter(|l| l.dep_name == "liltoon").count(),
        1,
        "an identical declaration inside one document emits one lead: {first:?}"
    );
    assert_eq!(first.len(), 2);
}

#[test]
fn a_bare_com_line_inside_a_heading_section_keeps_the_section_and_its_method() {
    // Batch-192 reverse review pin: a bare `com.*` line IS one of the three
    // structural families, so inside a heading section it must keep the
    // section open — its own method stays explicit_heading, and the bare
    // declaration lines after it are not silently dropped (before the fix
    // the section scan treated the com line as running prose, closed the
    // section, demoted the line to one_line and dropped the rest).
    let fixture = "\
〇前提環境
com.example.coolglow 1.2.0
liltoon 1.2.3~";

    let leads = extract_dependency_leads(fixture);
    assert_eq!(leads.len(), 2, "{leads:?}");

    assert_eq!(leads[0].dep_name, "com.example.coolglow");
    assert_eq!(leads[0].version_hint.as_deref(), Some("1.2.0"));
    assert_eq!(
        leads[0].extraction_method, METHOD_EXPLICIT_HEADING,
        "a com line inside the section registers the section method, not one_line"
    );

    assert_eq!(leads[1].dep_name, "liltoon");
    assert_eq!(
        leads[1].extraction_method, METHOD_EXPLICIT_HEADING,
        "the com line must not close the section: the bare declaration after it still extracts"
    );
}

#[test]
fn crlf_input_and_lone_cr_text_stay_honest() {
    // CRLF line endings: str::lines strips the \r, extraction is identical
    // to the LF form — the verbatim quote never carries the carriage return.
    let crlf = "〇前提環境\r\n・liltoon 1.2.3~\r\n";
    let leads = extract_dependency_leads(crlf);
    assert_eq!(leads.len(), 1, "{leads:?}");
    assert_eq!(leads[0].raw_quote, "・liltoon 1.2.3~");
    assert_eq!(leads[0].extraction_method, METHOD_EXPLICIT_HEADING);
    assert_eq!(leads[0].version_hint.as_deref(), Some("1.2.3~"));

    // Lone-CR text (no \n at all) is ONE line to the line scanner: no
    // fabricated split — the line either reads as a heading (nothing after
    // it) or as prose, and either way the answer is the honest empty set.
    let lone_cr = "〇前提環境\r・liltoon 1.2.3~";
    assert!(extract_dependency_leads(lone_cr).is_empty());
}

#[test]
fn leads_land_through_the_existing_store_write_face_as_unconfirmed_clues() {
    let store = BdlStore::open_in_memory().expect("in-memory store opens");
    store
        .seed_product("booth:2000001", "2000001")
        .expect("seed product");

    let leads = extract_dependency_leads(HEADING_SECTION_FIXTURE);
    assert!(!leads.is_empty());

    for lead in &leads {
        let new_observation = lead_to_new_observation(
            lead,
            "booth:2000001",
            "2026-09-23T02:40:00.000Z",
            "dep-extract-test",
            Some("sha256:1111111111111111111111111111111111111111111111111111111111111111"),
            None,
        );
        // The conversion stamps the extractor identity and leaves the
        // resolution fields empty (clue by construction).
        assert_eq!(new_observation.extracted_by, EXTRACTOR_ID);
        assert_eq!(new_observation.resolved_ref_product_id, None);
        assert!(new_observation.resolution_evidence.is_empty());

        let stored = store
            .record_dependency_observation(&new_observation)
            .expect("a lead lands through the existing write face");
        assert_eq!(stored.dep_kind, lead.dep_kind);
        assert_eq!(stored.dep_name, lead.dep_name);
        assert_eq!(stored.version_hint, lead.version_hint);
        assert_eq!(stored.raw_quote, lead.raw_quote);
        assert_eq!(stored.source_span, lead.source_span);
        assert_eq!(stored.extraction_method, lead.extraction_method);
        assert_eq!(stored.extracted_by, EXTRACTOR_ID);
        // Clue, never a conclusion: nothing in the extraction path confirms.
        assert!(!stored.confirmed_by_human);
        assert_eq!(stored.resolved_ref_product_id, None);
        assert_eq!(stored.resolution_evidence, None);
    }

    let rows = store
        .dependency_observations("booth:2000001")
        .expect("read face");
    assert_eq!(
        rows.len(),
        leads.len(),
        "verbatim round-trip, one row per lead"
    );

    // The landing goes through the real schema: an unknown product is
    // refused by the foreign key, never silently written.
    let orphan = lead_to_new_observation(
        &leads[0],
        "booth:9999999",
        "2026-09-23T02:40:00.000Z",
        "dep-extract-test",
        None,
        None,
    );
    assert!(
        store.record_dependency_observation(&orphan).is_err(),
        "a lead for an unobserved product must not land"
    );
}

#[test]
fn lead_type_matches_the_frozen_word_face() {
    // A type-level guard: a lead only ever carries the frozen closed-set
    // members it documents (the schema CHECK remains the real authority at
    // landing time — pinned again in the store test above).
    let fixture = "〇必要環境\n・dummyglow 1.0.0~";
    let leads: Vec<DependencyLead> = extract_dependency_leads(fixture);
    assert_eq!(leads.len(), 1);
    let lead = &leads[0];
    assert!(matches!(
        lead.dep_kind.as_str(),
        "shader" | "tool_package" | "avatar_base" | "other"
    ));
    assert!(matches!(
        lead.extraction_method.as_str(),
        "explicit_heading" | "bullet" | "one_line" | "prose" | "title" | "link"
    ));
    assert_eq!(lead.source_span, "body");
}
