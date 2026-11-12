//! BOOTH's purchase history (catalog-orders v0.1): which product was ordered when.
//!
//! Each order is one link to `/orders/<number>` holding the product's picture, whose address carries
//! the product's number, and the order's time. Only those two facts leave this parser: names, prices
//! and order numbers stay behind. BOOTH prints times in Japan time on every language's page.

use regex::Regex;
use scraper::{Html, Selector};
use std::sync::OnceLock;
use std::time::{Duration, UNIX_EPOCH};

const JAPAN_OFFSET_MS: i64 = 9 * 3_600 * 1000;

#[derive(Debug, PartialEq, Eq)]
pub struct OrderedProduct {
    pub product_id: String,
    pub ordered_at: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct OrdersPage {
    pub orders: Vec<OrderedProduct>,
    pub next_page_url: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct NotAnOrdersPage;

fn order_link() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"^(?:https://accounts\.booth\.pm)?/orders/\d+/?$").expect("static pattern")
    })
}

fn product_picture() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| Regex::new(r"/i/(\d+)/").expect("static pattern"))
}

fn order_time() -> &'static Regex {
    static PATTERN: OnceLock<Regex> = OnceLock::new();
    PATTERN.get_or_init(|| {
        Regex::new(r"(\d{4})/(\d{2})/(\d{2}) (\d{2}):(\d{2}):(\d{2})").expect("static pattern")
    })
}

fn ordered_at(text: &str) -> Option<String> {
    let found = order_time().captures(text)?;
    let as_written = format!(
        "{}-{}-{}T{}:{}:{}Z",
        &found[1], &found[2], &found[3], &found[4], &found[5], &found[6]
    );
    let millis = vua_orchestrator::rfc3339_millis(&as_written)? - JAPAN_OFFSET_MS;
    Some(vua_orchestrator::rfc3339(
        UNIX_EPOCH + Duration::from_millis(u64::try_from(millis).ok()?),
    ))
}

pub fn extract_orders_page(html: &str) -> Result<OrdersPage, NotAnOrdersPage> {
    let document = Html::parse_document(html);
    let links = Selector::parse("a[href]").expect("static selector");
    let pictures = Selector::parse("img[src]").expect("static selector");
    let mut orders = Vec::new();
    let mut seen_order = false;
    for link in document.select(&links).filter(|link| {
        link.value()
            .attr("href")
            .is_some_and(|href| order_link().is_match(href))
    }) {
        seen_order = true;
        let Some(time) = ordered_at(&link.text().collect::<String>()) else {
            continue;
        };
        for picture in link.select(&pictures) {
            let Some(found) = picture
                .value()
                .attr("src")
                .and_then(|src| product_picture().captures(src))
            else {
                continue;
            };
            orders.push(OrderedProduct {
                product_id: format!("booth:{}", &found[1]),
                ordered_at: time.clone(),
            });
        }
    }
    let shell = Selector::parse(".l-orders-index").expect("static selector");
    if !seen_order && document.select(&shell).next().is_none() {
        return Err(NotAnOrdersPage);
    }
    let next = Selector::parse("a[rel='next']").expect("static selector");
    let next_page_url = document
        .select(&next)
        .next()
        .and_then(|link| link.value().attr("href"))
        .filter(|href| !href.is_empty())
        .map(str::to_owned);
    Ok(OrdersPage {
        orders,
        next_page_url,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn order(number: u32, item: u32, stamp: &str) -> String {
        format!(
            r#"<a class="nav-reverse" href="/orders/{number}"><div class="sheet"><div class="on-white-bg"><img src="https://booth.pximg.net/c/76x76_a2_g5/00000000-0000-0000-0000-000000000000/i/{item}/00000000-0000-0000-0000-000000000000_base_resized.jpg"></div><div class="badge order-state completed">Completed</div><div class="u-tpg-caption1">Synthetic name 2020/01/01</div><div class="u-tpg-caption2">Created At: {stamp}</div></div></a>"#
        )
    }

    fn page(rows: &str, pager: &str) -> String {
        format!(
            r#"<html><body><div class="container l-orders-index">{rows}<div class="pager">{pager}</div></div></body></html>"#
        )
    }

    #[test]
    fn each_order_gives_its_products_number_and_time_in_utc() {
        let html = page(
            &(order(100, 901, "2026/10/16 12:34:56") + &order(99, 902, "2026/10/15 00:10:00")),
            "",
        );
        let parsed = extract_orders_page(&html).unwrap();
        assert_eq!(parsed.next_page_url, None);
        assert_eq!(
            parsed
                .orders
                .iter()
                .map(|o| o.product_id.as_str())
                .collect::<Vec<_>>(),
            ["booth:901", "booth:902"]
        );
        assert_eq!(
            vua_orchestrator::rfc3339_millis(&parsed.orders[0].ordered_at),
            vua_orchestrator::rfc3339_millis("2026-10-16T03:34:56Z")
        );
        assert_eq!(
            vua_orchestrator::rfc3339_millis(&parsed.orders[1].ordered_at),
            vua_orchestrator::rfc3339_millis("2026-10-14T15:10:00Z")
        );
    }

    #[test]
    fn the_order_time_is_read_after_any_date_in_the_name_only_when_it_has_a_clock() {
        let html = page(&order(100, 901, "2026/10/16 12:34:56"), "");
        let parsed = extract_orders_page(&html).unwrap();
        assert_eq!(
            vua_orchestrator::rfc3339_millis(&parsed.orders[0].ordered_at),
            vua_orchestrator::rfc3339_millis("2026-10-16T03:34:56Z")
        );
    }

    #[test]
    fn the_next_page_and_rows_without_a_picture_or_time_are_handled() {
        let no_picture = r#"<a href="/orders/1"><div>Created At: 2026/10/16 12:00:00</div></a>"#;
        let no_time = r#"<a href="/orders/2"><img src="https://booth.pximg.net/i/903/a.jpg"></a>"#;
        let other_link = r#"<a href="/orders/new"><img src="https://booth.pximg.net/i/904/a.jpg">2026/10/16 12:00:00</a>"#;
        let html = page(
            &format!("{no_picture}{no_time}{other_link}"),
            r#"<a rel="next" class="nav-item" href="/orders?page=2"></a>"#,
        );
        let parsed = extract_orders_page(&html).unwrap();
        assert!(parsed.orders.is_empty());
        assert_eq!(parsed.next_page_url.as_deref(), Some("/orders?page=2"));
    }

    #[test]
    fn an_empty_history_is_a_page_and_anything_else_is_not() {
        assert_eq!(extract_orders_page(&page("", "")).unwrap().orders, []);
        assert_eq!(
            extract_orders_page("<html><body><p>Sign in</p></body></html>"),
            Err(NotAnOrdersPage)
        );
    }
}
