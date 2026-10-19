//! Pure recognition of captured public links; fetching belongs to an explicit refresh task.
use regex::Regex;
use scraper::{Html, Selector};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, sync::OnceLock};
use url::Url;
use vua_bdl_store::terms::TermsLink;

pub fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn file_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 200
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
}

pub fn recognize_link(input: &str) -> Option<TermsLink> {
    let mut url = Url::parse(input.trim()).ok()?;
    if !["https", "http"].contains(&url.scheme())
        || !url.username().is_empty()
        || url.password().is_some()
        || input.len() > 4096
    {
        return None;
    }
    url.set_fragment(None);
    if url.as_str().len() > 4096 {
        return None;
    }
    let host = url.host_str()?.to_ascii_lowercase();
    let segments: Vec<_> = url.path_segments()?.filter(|s| !s.is_empty()).collect();
    let candidate = if url.port().is_none()
        && host == "drive.google.com"
        && segments.len() >= 3
        && segments[..2] == ["file", "d"]
        && segments.len() <= 4
        && segments
            .get(3)
            .is_none_or(|s| ["view", "edit", "preview"].contains(s))
    {
        Some(("drive", segments[2].to_owned()))
    } else if url.port().is_none()
        && host == "drive.google.com"
        && ["/open", "/uc"].contains(&url.path())
        || url.port().is_none()
            && host == "drive.usercontent.google.com"
            && url.path() == "/download"
    {
        url.query_pairs()
            .find(|(k, _)| k == "id")
            .map(|(_, v)| ("drive", v.into_owned()))
    } else if url.port().is_none()
        && host == "docs.google.com"
        && segments.len() >= 3
        && segments[..2] == ["document", "d"]
        && segments.len() <= 4
        && segments
            .get(3)
            .is_none_or(|s| ["edit", "preview", "export"].contains(s))
    {
        Some(("docs", segments[2].to_owned()))
    } else {
        None
    };
    if let Some((kind, id)) = candidate.filter(|(_, id)| file_id(id)) {
        return Some(TermsLink {
            source_id: format!("{kind}:{id}"),
            source_kind: kind.into(),
            source_url: if kind == "drive" {
                format!("https://drive.google.com/file/d/{id}/view")
            } else {
                format!("https://docs.google.com/document/d/{id}/edit")
            },
            google_file_id: Some(id),
        });
    }
    let kind = if ["vn3.org", "www.vn3.org"].contains(&host.as_str()) && url.port().is_none() {
        "vn3"
    } else {
        "link"
    };
    Some(TermsLink {
        source_id: format!("{kind}:{}", sha256(url.as_str().as_bytes())),
        source_kind: kind.into(),
        source_url: url.into(),
        google_file_id: None,
    })
}

pub fn description_text_links(text: &str) -> Vec<TermsLink> {
    static URLS: OnceLock<Regex> = OnceLock::new();
    let regex =
        URLS.get_or_init(|| Regex::new(r#"https?://[^\s<>"'`]+"#).expect("static URL recognizer"));
    let mut found = BTreeMap::new();
    for m in regex.find_iter(text) {
        let candidate = m
            .as_str()
            .trim_end_matches(['.', ',', ';', ')', ']', '}', '。', '、', '」', '）', '】']);
        if let Some(link) = recognize_link(candidate) {
            found.insert(link.source_id.clone(), link);
        }
    }
    found.into_values().collect()
}

pub fn description_html_links(html: &str) -> Option<Vec<TermsLink>> {
    let document = Html::parse_document(html);
    let selector = Selector::parse("#items article .js-market-item-detail-description.description")
        .expect("static description selector");
    let anchors = Selector::parse("a[href]").expect("static anchor selector");
    let bodies: Vec<_> = document.select(&selector).collect();
    if bodies.is_empty() {
        return None;
    }
    let mut found = BTreeMap::new();
    for body in bodies {
        for anchor in body.select(&anchors) {
            if let Some(link) = anchor.value().attr("href").and_then(recognize_link) {
                found.insert(link.source_id.clone(), link);
            }
        }
        for link in description_text_links(&body.text().collect::<Vec<_>>().join("\n")) {
            found.insert(link.source_id.clone(), link);
        }
    }
    Some(found.into_values().collect())
}
