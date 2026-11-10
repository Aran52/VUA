//! Thin product-observation and retry-admission application boundary.
use super::*;
use serde::Deserialize;
use vua_orchestrator::{extract_product_json, extract_product_page, ExtractedProduct};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Queue {
    schema_version: String,
    limit: u32,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Begin {
    schema_version: String,
    product_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Observe {
    schema_version: String,
    product_id: String,
    format: String,
    source_url: String,
    body: String,
    fetched_at: String,
}

fn valid_id(id: &str) -> bool {
    id.strip_prefix("booth:").is_some_and(|id| {
        !id.is_empty() && id.len() <= 20 && id.bytes().all(|b| b.is_ascii_digit())
    })
}

pub(super) fn request(
    warehouse: &WarehouseServices,
    method: &str,
    request: &Value,
    request_id: &str,
    correlation_id: &str,
) -> FrameOutcome {
    let query = method == "catalog.enrichmentQueue";
    let error = |code: &str, category: &str| {
        FrameOutcome::Response(application_error(
            request_id,
            correlation_id,
            &format!("vua.catalog.{code}"),
            if code == "store_failed" {
                "errors.catalog.storeFailed"
            } else {
                "errors.catalog.invalidParams"
            },
            category,
        ))
    };
    if request["kind"] != if query { "query" } else { "command" }
        || !query && request["commandId"].as_str().is_none_or(str::is_empty)
    {
        return error("invalid_params", "validation");
    }
    let now = vua_orchestrator::Clock::now_rfc3339(&SystemClock);
    let value = match method {
        "catalog.enrichmentQueue" => {
            let Ok(params) = serde_json::from_value::<Queue>(request["params"].clone()) else {
                return error("invalid_params", "validation");
            };
            if params.schema_version != "0.1" || !(1..=60).contains(&params.limit) {
                return error("invalid_params", "validation");
            }
            let Ok(product_ids) = warehouse.bdl.enrichment_queue(params.limit, &now) else {
                return error("store_failed", "internal");
            };
            json!({"schemaVersion":"0.1","productIds":product_ids})
        }
        "catalog.beginEnrichment" => {
            let Ok(params) = serde_json::from_value::<Begin>(request["params"].clone()) else {
                return error("invalid_params", "validation");
            };
            if params.schema_version != "0.1" || !valid_id(&params.product_id) {
                return error("invalid_params", "validation");
            }
            let Ok(claimed) = warehouse
                .bdl
                .begin_product_enrichment(&params.product_id, &now)
            else {
                return error("store_failed", "internal");
            };
            json!({"schemaVersion":"0.1","productId":params.product_id,"claimed":claimed})
        }
        "catalog.observeProduct" => {
            let Ok(params) = serde_json::from_value::<Observe>(request["params"].clone()) else {
                return error("invalid_params", "validation");
            };
            if params.schema_version != "0.1"
                || !valid_id(&params.product_id)
                || params.body.is_empty()
                || params.body.len() > 5_000_000
                || vua_orchestrator::rfc3339_millis(&params.fetched_at).is_none()
                || params.fetched_at.get(17..19) == Some("60")
                || !["html", "json"].contains(&params.format.as_str())
            {
                return error("invalid_params", "validation");
            }
            let native_id = &params.product_id[6..];
            let suffix = if params.format == "json" { ".json" } else { "" };
            if !["ja", "en", "ko", "zh-cn", "zh-tw"].iter().any(|locale| {
                params.source_url == format!("https://booth.pm/{locale}/items/{native_id}{suffix}")
            }) {
                return error("invalid_params", "validation");
            }
            let extracted = if params.format == "json" {
                extract_product_json(&params.body)
            } else {
                extract_product_page(&params.body)
            };
            let Ok(extracted) = extracted else {
                return error("invalid_product", "validation");
            };
            if extracted.native_product_id != native_id {
                return error("product_mismatch", "validation");
            }
            let mut observation = product_observation(extracted, &params.body, None);
            observation.source_url = params.source_url;
            observation.observed_at = params.fetched_at;
            observation.processor_version = format!("catalog-enrichment/0.1-{}", params.format);
            if warehouse
                .bdl
                .record_product_observation(&observation)
                .is_err()
            {
                return error("store_failed", "internal");
            }
            let links = if params.format == "html" {
                vua_acquisition::description_dependencies::record_description_dependencies(
                    &warehouse.bdl,
                    &params.body,
                    &observation.product_id,
                    &observation.content_hash,
                    &observation.observed_at,
                )
            } else if let Some(description) = observation.description.as_deref() {
                vua_acquisition::description_dependencies::record_description_text_dependencies(
                    &warehouse.bdl,
                    description,
                    &observation.product_id,
                    &observation.content_hash,
                    &observation.observed_at,
                )
            } else {
                Ok(())
            };
            if links.is_err() {
                return error("store_failed", "internal");
            }
            json!({"schemaVersion":"0.1","productId":observation.product_id})
        }
        _ => return error("invalid_params", "validation"),
    };
    FrameOutcome::Response(application_success(request_id, value))
}

pub(super) fn product_observation(
    extracted: ExtractedProduct,
    body: &str,
    library_type: Option<String>,
) -> vua_bdl_store::ProductObservation {
    vua_bdl_store::ProductObservation {
        product_id: format!("booth:{}", extracted.native_product_id),
        source_url: format!("https://booth.pm/en/items/{}", extracted.native_product_id),
        native_product_id: extracted.native_product_id,
        library_type,
        variant_name: None,
        final_url: None,
        status: vua_bdl_store::ProductObservationStatus::Complete,
        source_locale: None,
        source_category: extracted.source_category,
        title: extracted.title,
        description: extracted.body_text,
        age_restriction: None,
        adult: extracted.adult,
        availability: extracted.availability,
        price_amount: extracted.price_amount,
        price_currency: extracted.price_currency,
        shop_name: extracted.shop_name,
        shop_url: extracted.shop_url,
        image_urls: extracted.image_urls,
        video_urls: extracted.video_urls,
        subproducts: extracted
            .subproducts
            .into_iter()
            .map(|v| vua_bdl_store::SubproductObservation {
                variation_id: v.variation_id,
                name: v.name,
                price_amount: v.price_amount,
                price_currency: v.price_currency,
                availability: v.availability,
            })
            .collect(),
        source_published_at: extracted.published_date_raw,
        content_hash: vua_acquisition::library_page::page_content_hash(body),
        observed_at: chrono_now_rfc3339(),
        run_id: None,
        processor_version: "catalog-sync/0.2-product".into(),
        missing_fields: extracted
            .missing_fields
            .into_iter()
            .map(str::to_owned)
            .collect(),
    }
}
