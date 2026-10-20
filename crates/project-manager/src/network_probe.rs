//! Read-only, bounded first-play HTTPS probes. A fresh client never borrows browser
//! cookies. Regional targets are adapter-owned; website cards supply validated
//! HTTPS destinations. Responses return classifications, never page bodies or logs.
use reqwest::{redirect::Policy, Client};
use std::net::SocketAddr;
use std::time::{Duration, Instant};
use vua_orchestrator::network::{
    NetworkObservation, NetworkProbe, NetworkRegion, NetworkStatus, NetworkTarget,
    RegionalMeasurement, WebsiteObservation,
};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(6);
const BATCH_TIMEOUT: Duration = Duration::from_secs(8);
/// A batch containing regional references may spend this long from batch start
/// on its ordinary plus coordinated regional phases.
const REGIONAL_BATCH_TIMEOUT: Duration = Duration::from_secs(20);
/// Each regional reference opens this many independent connections and keeps
/// one discarded warm-up plus this many samples per connection.
const REGIONAL_CONNECTIONS: usize = 3;
const REGIONAL_ROUNDS: usize = 9;
const REGIONAL_SAMPLE_SPACING: Duration = Duration::from_millis(150);
/// A regional value needs at least this many kept samples across at least
/// this many connections; fewer samples report the failure instead.
const MIN_REGIONAL_SAMPLES: usize = 5;
const MIN_REGIONAL_CONNECTIONS: usize = 2;
/// The proxy hints' DNS lookups run before warm-up; they never hold the batch longer.
const PROXY_HINT_TIMEOUT: Duration = Duration::from_secs(2);

/// Regional references in one batch measure in this order, matching the tile's
/// Tokyo, US West, US East, Europe reading order.
const REGIONAL_REFERENCE_URLS: [&str; 4] = [
    "https://objectstorage.ap-tokyo-1.oraclecloud.com/",
    "https://objectstorage.us-sanjose-1.oraclecloud.com/",
    "https://objectstorage.us-ashburn-1.oraclecloud.com/",
    "https://objectstorage.eu-amsterdam-1.oraclecloud.com/",
];

/// Ordinary batches share a read lock; batches with regional references hold
/// the write lock so interleaved regional rounds see one network at a time.
fn website_batch_lock() -> &'static std::sync::RwLock<()> {
    static LOCK: std::sync::OnceLock<std::sync::RwLock<()>> = std::sync::OnceLock::new();
    LOCK.get_or_init(|| std::sync::RwLock::new(()))
}

pub struct HttpsNetworkProbe;

/// Same boundary as the desktop contract: explicit HTTPS URLs, no embedded login.
pub fn valid_website_url(value: &str) -> bool {
    value.len() <= 2048
        && value.trim() == value
        && url::Url::parse(value).is_ok_and(|u| {
            u.scheme() == "https"
                && u.host_str().is_some()
                && u.username().is_empty()
                && u.password().is_none()
                && u.fragment().is_none()
                && u.port_or_known_default() == Some(443)
        })
}

fn website_failure(url: &str, status: NetworkStatus, elapsed_ms: u64) -> WebsiteObservation {
    WebsiteObservation {
        url: url.to_owned(),
        status,
        elapsed_ms,
        http_status: None,
        regional: None,
    }
}

/// Fetch headers only, with the OS/process network route. No cookies or credentials
/// are installed on this fresh client. A HEAD-only denial retries GET to headers,
/// then drops the response without consuming the page body.
async fn test_website(value: String) -> WebsiteObservation {
    let started = Instant::now();
    let source = value.clone();
    let client = Client::builder()
        .https_only(true)
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(Duration::from_secs(4))
        .user_agent("VUA-Website-Test/0.1")
        .redirect(Policy::custom(move |a| {
            if follow_website_redirect(&source, a.url().as_str(), a.previous().len()) {
                a.follow()
            } else {
                a.stop()
            }
        }))
        .build();
    let Ok(client) = client else {
        return website_failure(&value, NetworkStatus::ProbeError, 0);
    };
    let result = tokio::time::timeout(REQUEST_TIMEOUT, async {
        let response = client.head(&value).send().await?;
        if matches!(response.status().as_u16(), 405 | 501) {
            drop(response);
            client.get(&value).send().await
        } else {
            Ok(response)
        }
    })
    .await;
    let elapsed_ms = started.elapsed().as_millis() as u64;
    match result {
        Ok(Ok(response)) => WebsiteObservation {
            url: value,
            status: classify_http(response.status().as_u16()),
            elapsed_ms,
            http_status: Some(response.status().as_u16()),
            regional: None,
        },
        Ok(Err(error)) => website_failure(
            &value,
            if error.is_timeout() {
                NetworkStatus::Timeout
            } else {
                NetworkStatus::ConnectionFailed
            },
            elapsed_ms,
        ),
        Err(_) => website_failure(&value, NetworkStatus::Timeout, elapsed_ms),
    }
}

fn regional_reference(source: &str) -> bool {
    REGIONAL_REFERENCE_URLS.contains(&source)
}

/// Positions of the regional references in a batch, in Tokyo, US West,
/// US East, Europe order.
fn regional_probe_order(urls: &[String]) -> Vec<usize> {
    REGIONAL_REFERENCE_URLS
        .iter()
        .filter_map(|reference| urls.iter().position(|url| url == reference))
        .collect()
}

/// One timed regional HEAD. Success and the expected anonymous root 404
/// leave a reusable connection; anything else retires it.
enum RegionalSample {
    Timed(u64, u16),
    Closed,
    Status(u16),
    Transport(NetworkStatus),
}

async fn regional_head(client: &Client, url: &str) -> RegionalSample {
    let started = Instant::now();
    match client.head(url).send().await {
        Ok(response) => {
            let code = response.status().as_u16();
            // A server-demanded closure can no longer warm a connection,
            // and HEAD has no body to download or retain.
            if response
                .headers()
                .get(reqwest::header::CONNECTION)
                .is_some_and(|header| {
                    header.to_str().is_ok_and(|s| {
                        s.split(',')
                            .any(|part| part.trim().eq_ignore_ascii_case("close"))
                    })
                })
            {
                return RegionalSample::Closed;
            }
            drop(response);
            if code == 404 || response_status_is_success(code) {
                RegionalSample::Timed(started.elapsed().as_millis() as u64, code)
            } else {
                RegionalSample::Status(code)
            }
        }
        Err(error) => RegionalSample::Transport(if error.is_timeout() {
            NetworkStatus::Timeout
        } else {
            NetworkStatus::ConnectionFailed
        }),
    }
}

fn response_status_is_success(code: u16) -> bool {
    (200..=299).contains(&code)
}

fn regional_client() -> Result<Client, reqwest::Error> {
    Client::builder()
        .https_only(true)
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(Duration::from_secs(4))
        .pool_max_idle_per_host(1)
        .user_agent("VUA-Regional-Reference/0.1")
        .redirect(Policy::none())
        .build()
}

/// Live per-connection state for one regional reference.
struct RegionalState {
    url: String,
    via_proxy: bool,
    connections: Vec<Option<Client>>,
    next_connection: usize,
    samples: Vec<(u64, usize)>,
    failures: Vec<(NetworkStatus, Option<u16>)>,
    last_status: Option<u16>,
    last_sample_end: Option<Instant>,
}

impl RegionalState {
    fn new(url: &str, via_proxy: bool, connections: Vec<Client>) -> Self {
        let mut failures = Vec::new();
        let slots = (0..REGIONAL_CONNECTIONS)
            .map(|slot| {
                let client = connections.get(slot).cloned();
                if client.is_none() {
                    failures.push((NetworkStatus::ProbeError, None));
                }
                client
            })
            .collect();
        Self {
            url: url.to_owned(),
            via_proxy,
            connections: slots,
            next_connection: 0,
            samples: Vec::new(),
            failures,
            last_status: None,
            last_sample_end: None,
        }
    }

    /// A failed connection's samples can hide the slow route it was on, so
    /// retiring drops the samples it already produced.
    fn retire(&mut self, index: usize) {
        self.connections[index] = None;
        self.samples.retain(|(_, connection)| *connection != index);
    }

    fn live_connection(&mut self) -> Option<usize> {
        for _ in 0..self.connections.len() {
            let candidate = self.next_connection;
            self.next_connection = (self.next_connection + 1) % self.connections.len();
            if self.connections[candidate].is_some() {
                return Some(candidate);
            }
        }
        None
    }
}

fn retire_connection(
    regions: &mut [RegionalState],
    region_index: usize,
    connection_index: usize,
    failure: (NetworkStatus, Option<u16>),
) {
    if let Some(region) = regions.get_mut(region_index) {
        region.failures.push(failure);
        region.retire(connection_index);
    }
}

/// Warm every connection concurrently, so no later sample carries DNS, TCP or
/// TLS setup and cross-region rounds start together.
async fn warm_regional_connections(regions: &mut [RegionalState], deadline: Instant) {
    let mut warming = tokio::task::JoinSet::new();
    for (region_index, region) in regions.iter().enumerate() {
        for (connection_index, client) in region.connections.iter().enumerate() {
            // Client clones share one pool, so the warmed connection is reused.
            let Some(client) = client.clone() else {
                continue;
            };
            let url = region.url.clone();
            warming.spawn(async move {
                (
                    region_index,
                    connection_index,
                    regional_head(&client, &url).await,
                )
            });
        }
    }
    let drain = async {
        while let Some(joined) = warming.join_next().await {
            let Ok((region_index, connection_index, outcome)) = joined else {
                continue;
            };
            match outcome {
                RegionalSample::Timed(_, _) => {}
                RegionalSample::Closed => retire_connection(
                    regions,
                    region_index,
                    connection_index,
                    (NetworkStatus::ProbeError, None),
                ),
                RegionalSample::Status(code) => retire_connection(
                    regions,
                    region_index,
                    connection_index,
                    (classify_http(code), Some(code)),
                ),
                RegionalSample::Transport(status) => {
                    retire_connection(regions, region_index, connection_index, (status, None))
                }
            }
        }
    };
    let remaining = deadline.saturating_duration_since(Instant::now());
    if tokio::time::timeout(remaining, drain).await.is_err() {
        warming.abort_all();
    }
}

/// Sample every region once per round, in the fixed region order, so all
/// regions experience the same moments of the network. A region waits out the
/// spacing interval since its previous sample before sampling again.
async fn run_regional_rounds(regions: &mut [RegionalState], deadline: Instant, spacing: Duration) {
    for _ in 0..REGIONAL_ROUNDS {
        for region in regions.iter_mut() {
            let now = Instant::now();
            if now >= deadline {
                return;
            }
            if let Some(end) = region.last_sample_end {
                let earliest = end + spacing;
                if now < earliest {
                    tokio::time::sleep(earliest - now).await;
                    if Instant::now() >= deadline {
                        return;
                    }
                }
            }
            let Some(connection_index) = region.live_connection() else {
                continue;
            };
            let Some(client) = region.connections[connection_index].clone() else {
                continue;
            };
            let url = region.url.clone();
            let outcome = regional_head(&client, &url).await;
            let ended = Instant::now();
            match outcome {
                RegionalSample::Timed(value, code) => {
                    region.samples.push((value, connection_index));
                    region.last_status = Some(code);
                    region.last_sample_end = Some(ended);
                }
                RegionalSample::Closed => {
                    region.failures.push((NetworkStatus::ProbeError, None));
                    region.retire(connection_index);
                }
                RegionalSample::Status(code) => {
                    region.failures.push((classify_http(code), Some(code)));
                    region.retire(connection_index);
                }
                RegionalSample::Transport(status) => {
                    region.failures.push((status, None));
                    region.retire(connection_index);
                }
            }
        }
    }
}

/// Nearest-rank 90th percentile: the ceil(0.9·n)-th smallest value, zero-based.
fn nearest_rank_p90(count: usize) -> usize {
    (count * 9).div_ceil(10) - 1
}

fn regional_observation(region: &RegionalState, phase: Instant) -> WebsiteObservation {
    let mut values: Vec<u64> = region.samples.iter().map(|(value, _)| *value).collect();
    values.sort_unstable();
    let connections = region
        .samples
        .iter()
        .map(|(_, connection)| *connection)
        .collect::<std::collections::HashSet<usize>>()
        .len();
    if values.len() >= MIN_REGIONAL_SAMPLES && connections >= MIN_REGIONAL_CONNECTIONS {
        let status = region.last_status.unwrap_or(404);
        return WebsiteObservation {
            url: region.url.clone(),
            status: classify_http(status),
            elapsed_ms: values[0],
            http_status: Some(status),
            regional: Some(RegionalMeasurement {
                median_ms: values[(values.len() - 1) / 2],
                p90_ms: values[nearest_rank_p90(values.len())],
                samples: values.len() as u32,
                connections: connections as u32,
                via_proxy: region.via_proxy,
            }),
        };
    }
    regional_failure(region, phase)
}

/// The reported failure keeps today's meaning per kind: transport timeouts and
/// connection losses outrank HTTP answers, which outrank protocol closures. An
/// incomplete set without any recorded failure means the phase deadline cut
/// sampling short, which reports as a timeout.
fn regional_failure(region: &RegionalState, phase: Instant) -> WebsiteObservation {
    let elapsed_ms = phase.elapsed().as_millis() as u64;
    for status in [NetworkStatus::Timeout, NetworkStatus::ConnectionFailed] {
        if region.failures.iter().any(|(failed, _)| *failed == status) {
            return website_failure(&region.url, status, elapsed_ms);
        }
    }
    for (status, code) in &region.failures {
        if matches!(status, NetworkStatus::HttpError | NetworkStatus::Redirected) {
            return WebsiteObservation {
                url: region.url.clone(),
                status: *status,
                elapsed_ms,
                http_status: *code,
                regional: None,
            };
        }
    }
    if region.failures.is_empty() {
        website_failure(&region.url, NetworkStatus::Timeout, elapsed_ms)
    } else {
        website_failure(&region.url, NetworkStatus::ProbeError, 0)
    }
}

/// Clash-style TUN/fake-IP proxies answer DNS in 198.18.0.0/15; a resolution
/// there means the measured route is this computer's proxy, not the game's.
fn is_fake_ip_address(address: &SocketAddr) -> bool {
    match address {
        SocketAddr::V4(address) => {
            let octets = address.ip().octets();
            octets[0] == 198 && (18..=19).contains(&octets[1])
        }
        SocketAddr::V6(_) => false,
    }
}

fn any_fake_ip_address(addresses: &[SocketAddr]) -> bool {
    addresses.iter().any(is_fake_ip_address)
}

/// Mirrors reqwest's environment proxy choice for an HTTPS URL: HTTPS_PROXY or
/// https_proxy, then ALL_PROXY or all_proxy, unless NO_PROXY or no_proxy
/// excludes the host. The Windows registry system proxy stays out of scope.
fn environment_proxy_applies(
    https_proxy: Option<&str>,
    all_proxy: Option<&str>,
    no_proxy: Option<&str>,
    host: &str,
) -> bool {
    if https_proxy.or(all_proxy).is_none() {
        return false;
    }
    !no_proxy_excludes_host(no_proxy, host)
}

/// Mirrors reqwest's documented NO_PROXY rules: comma-separated entries,
/// `*` matches everything, any other entry is a domain (an optional leading
/// dot is equivalent) matching the domain and its subdomains. IP entries
/// compare literally; subnet suffixes have no domain meaning.
fn no_proxy_excludes_host(no_proxy: Option<&str>, host: &str) -> bool {
    let Some(list) = no_proxy else {
        return false;
    };
    list.split(',').any(|entry| {
        let entry = entry.trim();
        if entry.is_empty() {
            return false;
        }
        if entry == "*" {
            return true;
        }
        let domain = entry.strip_prefix('.').unwrap_or(entry);
        host == domain || host.ends_with(&format!(".{domain}"))
    })
}

async fn regional_proxy_hint(url: &str) -> bool {
    let Some(host) = url::Url::parse(url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_owned))
    else {
        return false;
    };
    if let Ok(addresses) = tokio::net::lookup_host((host.as_str(), 443)).await {
        if any_fake_ip_address(&addresses.collect::<Vec<_>>()) {
            return true;
        }
    }
    let variable = |name: &str| std::env::var(name).ok().filter(|value| !value.is_empty());
    environment_proxy_applies(
        variable("HTTPS_PROXY")
            .or_else(|| variable("https_proxy"))
            .as_deref(),
        variable("ALL_PROXY")
            .or_else(|| variable("all_proxy"))
            .as_deref(),
        variable("NO_PROXY")
            .or_else(|| variable("no_proxy"))
            .as_deref(),
        &host,
    )
}

/// The lock-free sampling core: warm every connection concurrently, then take
/// nine interleaved rounds in region order. Tests drive this with loopback
/// clients; production reaches it through measure_regional_references.
async fn run_regional_measurement(
    regions: &mut [RegionalState],
    deadline: Instant,
    spacing: Duration,
) {
    warm_regional_connections(regions, deadline).await;
    run_regional_rounds(regions, deadline, spacing).await;
}

/// Measures the regional references as one coordinated group: proxy hints,
/// concurrent warm-up of every connection, then interleaved sampling rounds
/// within the deadline.
async fn measure_regional_references(
    urls: &[String],
    deadline: Instant,
    spacing: Duration,
) -> Vec<WebsiteObservation> {
    let phase = Instant::now();
    // Hints finish in any order; each keeps its URL's position. The OS resolver
    // can hang while this batch holds the regional lock, so the hints get a
    // bounded share of the deadline, and an unanswered lookup reads as no proxy.
    let mut hints = tokio::task::JoinSet::new();
    for (index, url) in urls.iter().enumerate() {
        let url = url.clone();
        hints.spawn(async move { (index, regional_proxy_hint(&url).await) });
    }
    let mut via_proxies = vec![false; urls.len()];
    let hint_deadline = deadline.min(Instant::now() + PROXY_HINT_TIMEOUT);
    let _ = tokio::time::timeout(hint_deadline.saturating_duration_since(Instant::now()), async {
        while let Some(joined) = hints.join_next().await {
            if let Ok((index, via_proxy)) = joined {
                via_proxies[index] = via_proxy;
            }
        }
    })
    .await;
    hints.abort_all();
    let mut regions = Vec::with_capacity(urls.len());
    for (url, via_proxy) in urls.iter().zip(via_proxies) {
        let connections = (0..REGIONAL_CONNECTIONS)
            .filter_map(|_| regional_client().ok())
            .collect();
        regions.push(RegionalState::new(url, via_proxy, connections));
    }
    run_regional_measurement(&mut regions, deadline, spacing).await;
    regions
        .iter()
        .map(|region| regional_observation(region, phase))
        .collect()
}

/// Approximate regional timing stays at the documented regional service origin.
/// Ordinary user-selected website tests retain the existing HTTPS redirect limit.
fn follow_website_redirect(source: &str, destination: &str, hops: usize) -> bool {
    !regional_reference(source) && hops <= 3 && valid_website_url(destination)
}

fn endpoint(target: NetworkTarget) -> &'static str {
    match target {
        NetworkTarget::SteamStore => "https://store.steampowered.com/join/",
        NetworkTarget::SteamCommunity => "https://steamcommunity.com/",
        NetworkTarget::SteamDownload => {
            "https://cdn.fastly.steamstatic.com/client/installer/SteamSetup.exe"
        }
        NetworkTarget::VrchatWeb => "https://vrchat.com/home",
        NetworkTarget::PicoConnect => "https://www.picoxr.com/global/software/pico-connect",
    }
}

fn vendor_host(target: NetworkTarget, host: &str) -> bool {
    let domains: &[&str] = match target {
        NetworkTarget::SteamStore => &["steampowered.com"],
        NetworkTarget::SteamCommunity => &["steamcommunity.com"],
        NetworkTarget::SteamDownload => &["steamstatic.com", "steamcontent.com"],
        NetworkTarget::VrchatWeb => &["vrchat.com"],
        NetworkTarget::PicoConnect => &["picoxr.com"],
    };
    domains
        .iter()
        .any(|d| host == *d || host.ends_with(&format!(".{d}")))
}

fn client(target: Option<NetworkTarget>) -> Result<Client, reqwest::Error> {
    Client::builder()
        .https_only(true)
        .timeout(REQUEST_TIMEOUT)
        .connect_timeout(Duration::from_secs(4))
        .user_agent("VUA-Network-Check/0.1")
        .redirect(Policy::custom(move |attempt| {
            // Stop, rather than fetch an untrusted redirect or convert it into success.
            if attempt.previous().len() <= 3
                && attempt.url().scheme() == "https"
                && attempt.url().username().is_empty()
                && attempt.url().password().is_none()
                && attempt.url().port_or_known_default() == Some(443)
                && target
                    .is_some_and(|t| attempt.url().host_str().is_some_and(|h| vendor_host(t, h)))
            {
                attempt.follow()
            } else {
                attempt.stop()
            }
        }))
        .build()
}

fn observation(
    target: NetworkTarget,
    status: NetworkStatus,
    started: Instant,
    http_status: Option<u16>,
) -> NetworkObservation {
    NetworkObservation {
        target,
        status,
        elapsed_ms: started.elapsed().as_millis() as u64,
        http_status,
    }
}

async fn probe(target: NetworkTarget) -> NetworkObservation {
    let started = Instant::now();
    let Ok(client) = client(Some(target)) else {
        return observation(target, NetworkStatus::ProbeError, started, None);
    };
    probe_url(&client, target, endpoint(target), started).await
}

// Kept private: production callers can only select the fixed target catalog.
// Tests use a loopback server to exercise transport behavior without live Internet.
async fn probe_url(
    client: &Client,
    target: NetworkTarget,
    url: &str,
    started: Instant,
) -> NetworkObservation {
    let result = client.head(url).send().await;
    match result {
        Ok(response) => {
            let code = response.status();
            let status = classify_http(code.as_u16());
            // HEAD reads no installer or page body. Even an HTTP denial is useful:
            // it means the server replied, not that the user's Internet is offline.
            observation(target, status, started, Some(code.as_u16()))
        }
        Err(error) => observation(
            target,
            if error.is_timeout() {
                NetworkStatus::Timeout
            } else {
                NetworkStatus::ConnectionFailed
            },
            started,
            None,
        ),
    }
}

fn classify_http(code: u16) -> NetworkStatus {
    match code {
        200..=299 => NetworkStatus::Reachable,
        300..=399 => NetworkStatus::Redirected,
        _ => NetworkStatus::HttpError,
    }
}

async fn region() -> NetworkRegion {
    let Ok(client) = client(None) else {
        return NetworkRegion::Unknown;
    };
    let Ok(mut response) = client
        .get("https://www.cloudflare.com/cdn-cgi/trace")
        .send()
        .await
    else {
        return NetworkRegion::Unknown;
    };
    if !response.status().is_success() {
        return NetworkRegion::Unknown;
    }
    // This endpoint includes an IP. Consume a maximum of 8 KiB in memory, extract
    // only loc, then discard the body. Never serialize, cache or log its contents.
    let mut bytes = Vec::new();
    loop {
        match response.chunk().await {
            Ok(Some(chunk)) if bytes.len() + chunk.len() <= 8192 => bytes.extend_from_slice(&chunk),
            Ok(None) => break,
            _ => return NetworkRegion::Unknown,
        }
    }
    match crate::unity_download_region::parse_region(&String::from_utf8_lossy(&bytes)) {
        Some(vua_orchestrator::deployment::DownloadRegion::ChinaMainland) => {
            NetworkRegion::ChinaMainland
        }
        Some(vua_orchestrator::deployment::DownloadRegion::Other) => NetworkRegion::Other,
        _ => NetworkRegion::Unknown,
    }
}

impl NetworkProbe for HttpsNetworkProbe {
    fn test_websites(&self, urls: &[String]) -> Vec<WebsiteObservation> {
        // Validate again at the IO boundary, even when called outside Provider.
        if urls.is_empty() || urls.len() > 12 || urls.iter().any(|u| !valid_website_url(u)) {
            return urls
                .iter()
                .map(|u| website_failure(u, NetworkStatus::ProbeError, 0))
                .collect();
        }
        let started = Instant::now();
        let Ok(runtime) = tokio::runtime::Runtime::new() else {
            return urls
                .iter()
                .map(|u| website_failure(u, NetworkStatus::ProbeError, 0))
                .collect();
        };
        let mut results: Vec<Option<WebsiteObservation>> = vec![None; urls.len()];
        let regional = regional_probe_order(urls);
        // Ordinary destinations keep their concurrent batch and its bound. The
        // shared lock is taken and released in sync code so no await carries it.
        if regional.len() != urls.len() {
            let _ordinary = website_batch_lock()
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let mut done = runtime.block_on(async {
                let mut pending = tokio::task::JoinSet::new();
                for (index, value) in urls.iter().enumerate() {
                    if regional.contains(&index) {
                        continue;
                    }
                    let website = test_website(value.clone());
                    pending.spawn(async move { (index, website.await) });
                }
                let mut done = Vec::new();
                let _ = tokio::time::timeout(BATCH_TIMEOUT, async {
                    while let Some(result) = pending.join_next().await {
                        if let Ok(value) = result {
                            done.push(value);
                        }
                    }
                })
                .await;
                pending.abort_all();
                done
            });
            for (index, observation) in done.drain(..) {
                results[index] = Some(observation);
            }
        }
        // Regional references wait out ordinary batches, then measure together
        // under the longer regional bound.
        if !regional.is_empty() {
            let _exclusive = website_batch_lock()
                .write()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            let budget = REGIONAL_BATCH_TIMEOUT.saturating_sub(started.elapsed());
            let deadline = Instant::now() + budget;
            let targets = regional
                .iter()
                .map(|&index| urls[index].clone())
                .collect::<Vec<_>>();
            let measured = runtime.block_on(measure_regional_references(
                &targets,
                deadline,
                REGIONAL_SAMPLE_SPACING,
            ));
            for (slot, observation) in regional.iter().zip(measured) {
                results[*slot] = Some(observation);
            }
        }
        let observations = urls
            .iter()
            .enumerate()
            .map(|(index, value)| {
                results[index].clone().unwrap_or_else(|| {
                    website_failure(
                        value,
                        NetworkStatus::Timeout,
                        started.elapsed().as_millis() as u64,
                    )
                })
            })
            .collect();
        runtime.shutdown_background();
        observations
    }
    fn probe(
        &self,
        targets: &[NetworkTarget],
        detect_region: bool,
    ) -> (NetworkRegion, Vec<NetworkObservation>) {
        let started = Instant::now();
        let failures = |status| {
            targets
                .iter()
                .map(|&t| observation(t, status, started, None))
                .collect()
        };
        let Ok(runtime) = tokio::runtime::Runtime::new() else {
            return (NetworkRegion::Unknown, failures(NetworkStatus::ProbeError));
        };
        let result = runtime.block_on(async {
            let mut pending = tokio::task::JoinSet::new();
            for &target in targets {
                pending.spawn(probe(target));
            }
            let region_task = tokio::spawn(async move {
                if detect_region {
                    region().await
                } else {
                    NetworkRegion::Unknown
                }
            });
            let mut results = Vec::new();
            let _ = tokio::time::timeout(BATCH_TIMEOUT, async {
                while let Some(result) = pending.join_next().await {
                    if let Ok(value) = result {
                        results.push(value);
                    }
                }
            })
            .await;
            pending.abort_all();
            let detected =
                tokio::time::timeout(BATCH_TIMEOUT.saturating_sub(started.elapsed()), region_task)
                    .await
                    .ok()
                    .and_then(Result::ok)
                    .unwrap_or(NetworkRegion::Unknown);
            // Stable order and one result per target, including a batch deadline.
            let ordered = targets
                .iter()
                .map(|&target| {
                    results
                        .iter()
                        .find(|r| r.target == target)
                        .cloned()
                        .unwrap_or_else(|| {
                            observation(target, NetworkStatus::Timeout, started, None)
                        })
                })
                .collect();
            (detected, ordered)
        });
        // OS DNS resolution may outlive the cancelled request on a blocking
        // worker. Do not let runtime teardown turn the UI deadline into a hang.
        runtime.shutdown_background();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};

    #[test]
    fn website_destinations_exclude_credentials_and_non_https() {
        assert!(valid_website_url("https://github.com/"));
        for value in [
            "http://github.com",
            "https://user:secret@example.com",
            "file:///tmp/test",
            "https://example.com:8443",
            "https://example.com/#secret",
        ] {
            assert!(!valid_website_url(value), "{value}");
        }
    }

    fn serve_once(status: u16, delay: Duration) -> (String, std::thread::JoinHandle<String>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/probe", listener.local_addr().unwrap());
        let handle = std::thread::spawn(move || {
            let (mut socket, _) = listener.accept().unwrap();
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut bytes = [0; 4096];
            let count = socket.read(&mut bytes).unwrap();
            std::thread::sleep(delay);
            let _ = write!(
                socket,
                "HTTP/1.1 {status} Test\r\nContent-Length: 99999999\r\nConnection: close\r\n\r\n"
            );
            String::from_utf8_lossy(&bytes[..count]).into_owned()
        });
        (url, handle)
    }

    #[test]
    fn network_transport_reads_headers_only_and_returns_partial_errors() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let client = Client::builder()
            .no_proxy()
            .redirect(Policy::none())
            .timeout(Duration::from_millis(300))
            .build()
            .unwrap();
        for (code, expected) in [
            (200, NetworkStatus::Reachable),
            (403, NetworkStatus::HttpError),
            (302, NetworkStatus::Redirected),
        ] {
            let (url, server) = serve_once(code, Duration::ZERO);
            let result = runtime.block_on(probe_url(
                &client,
                NetworkTarget::SteamStore,
                &url,
                Instant::now(),
            ));
            assert_eq!(result.status, expected);
            assert_eq!(result.http_status, Some(code));
            let request = server.join().unwrap();
            assert!(request.starts_with("HEAD /probe "));
            assert!(!request.to_ascii_lowercase().contains("cookie:"));
        }
        let (url, server) = serve_once(200, Duration::from_millis(700));
        let result = runtime.block_on(probe_url(
            &client,
            NetworkTarget::VrchatWeb,
            &url,
            Instant::now(),
        ));
        assert_eq!(result.status, NetworkStatus::Timeout);
        assert_eq!(result.http_status, None);
        server.join().unwrap();
    }

    /// Loopback keep-alive server for regional sampling: one thread per scripted
    /// connection; a script entry is (status, delay, close), where status 0
    /// drops the socket instead of answering. Every request-header arrival is
    /// recorded with its instant.
    fn serve_regional(
        scripts: Vec<Vec<(u16, Duration, bool)>>,
    ) -> (
        String,
        std::sync::Arc<std::sync::Mutex<Vec<Instant>>>,
        std::thread::JoinHandle<()>,
    ) {
        use std::sync::{Arc, Mutex};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/reference", listener.local_addr().unwrap());
        let arrivals = Arc::new(Mutex::new(Vec::new()));
        let log = arrivals.clone();
        let handle = std::thread::spawn(move || {
            for script in scripts {
                let Ok((mut socket, _)) = listener.accept() else {
                    return;
                };
                socket.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
                let log = log.clone();
                std::thread::spawn(move || {
                    for (status, delay, close) in script {
                        let mut request = Vec::new();
                        while !request.ends_with(b"\r\n\r\n") {
                            let mut byte = [0];
                            if socket.read(&mut byte).unwrap_or(0) == 0 {
                                return;
                            }
                            request.push(byte[0]);
                        }
                        log.lock().unwrap().push(Instant::now());
                        if status == 0 {
                            return;
                        }
                        std::thread::sleep(delay);
                        let connection = if close { "close" } else { "keep-alive" };
                        let _ = write!(
                            socket,
                            "HTTP/1.1 {status} Test\r\nContent-Length: 0\r\nConnection: {connection}\r\n\r\n"
                        );
                    }
                });
            }
        });
        (url, arrivals, handle)
    }

    fn loopback_region(url: &str, connections: usize) -> RegionalState {
        let clients = (0..connections)
            .filter_map(|_| {
                Client::builder()
                    .no_proxy()
                    .redirect(Policy::none())
                    .build()
                    .ok()
            })
            .collect();
        RegionalState::new(url, false, clients)
    }

    async fn sample_regions(regions: &mut [RegionalState], spacing: Duration) -> Instant {
        let phase = Instant::now();
        let deadline = phase + Duration::from_secs(60);
        run_regional_measurement(regions, deadline, spacing).await;
        phase
    }

    fn wait_for_arrivals(log: &std::sync::Arc<std::sync::Mutex<Vec<Instant>>>, count: usize) {
        for _ in 0..400 {
            if log.lock().unwrap().len() >= count {
                return;
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    #[test]
    fn regional_measurement_reports_minimum_with_median_and_p90() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let script = vec![
            (404u16, Duration::from_millis(300), false),
            (404, Duration::from_millis(40), false),
            (404, Duration::from_millis(100), false),
            (404, Duration::from_millis(160), false),
        ];
        let (url, log, server) = serve_regional(vec![script; 3]);
        let mut regions = [loopback_region(&url, REGIONAL_CONNECTIONS)];
        let phase = runtime.block_on(sample_regions(&mut regions, Duration::ZERO));
        let result = regional_observation(&regions[0], phase);
        assert_eq!(result.status, NetworkStatus::HttpError);
        assert_eq!(result.http_status, Some(404));
        let regional = result.regional.as_ref().expect("nine samples succeed");
        assert_eq!(regional.samples, 9);
        assert_eq!(regional.connections, 3);
        assert!(!regional.via_proxy);
        assert!(
            (40..60).contains(&result.elapsed_ms),
            "minimum sample and no warm-up drives elapsed: {result:?}"
        );
        assert!(
            (90..130).contains(&regional.median_ms),
            "median of 40x3, 100x3, 160x3: {regional:?}"
        );
        assert!(
            (150..190).contains(&regional.p90_ms),
            "nearest-rank p90: {regional:?}"
        );
        wait_for_arrivals(&log, 12);
        server.join().unwrap();
    }

    #[test]
    fn a_failed_connection_drops_only_its_own_samples() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        // Surviving connections absorb the rounds the failed one no longer
        // takes, so their scripts run longer than three samples.
        let healthy = vec![(404u16, Duration::from_millis(30), false); 10];
        let failing = vec![
            (404, Duration::from_millis(30), false),
            (404, Duration::from_millis(30), false),
            (404, Duration::from_millis(30), true),
        ];
        let (url, log, server) = serve_regional(vec![failing, healthy.clone(), healthy]);
        let mut regions = [loopback_region(&url, REGIONAL_CONNECTIONS)];
        let phase = runtime.block_on(sample_regions(&mut regions, Duration::ZERO));
        let result = regional_observation(&regions[0], phase);
        assert_eq!(result.status, NetworkStatus::HttpError);
        let regional = result
            .regional
            .as_ref()
            .expect("seven samples on two connections");
        assert_eq!(regional.samples, 7);
        assert_eq!(regional.connections, 2);
        wait_for_arrivals(&log, 12);
        server.join().unwrap();
    }

    #[test]
    fn samples_from_a_single_connection_stay_a_failure() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let dying = vec![
            (404u16, Duration::from_millis(20), false),
            (0, Duration::ZERO, false),
        ];
        let absorbing = vec![(404, Duration::from_millis(20), false); 10];
        let (url, _log, server) = serve_regional(vec![dying.clone(), dying, absorbing]);
        let mut regions = [loopback_region(&url, REGIONAL_CONNECTIONS)];
        let phase = runtime.block_on(sample_regions(&mut regions, Duration::ZERO));
        let result = regional_observation(&regions[0], phase);
        assert_eq!(result.status, NetworkStatus::ConnectionFailed);
        assert_eq!(result.http_status, None);
        assert!(result.regional.is_none());
        server.join().unwrap();
    }

    #[test]
    fn regional_timeouts_http_errors_redirects_and_closures_stay_failures() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        for (scripts, status, code) in [
            (
                vec![
                    vec![
                        (404u16, Duration::ZERO, false),
                        (404, Duration::from_millis(200), false),
                    ];
                    3
                ],
                NetworkStatus::Timeout,
                None,
            ),
            (
                vec![
                    vec![
                        (404u16, Duration::ZERO, false),
                        (429, Duration::ZERO, false),
                    ];
                    3
                ],
                NetworkStatus::HttpError,
                Some(429),
            ),
            (
                vec![vec![(302u16, Duration::ZERO, false)]; 3],
                NetworkStatus::Redirected,
                Some(302),
            ),
            (
                vec![vec![(404u16, Duration::ZERO, true)]; 3],
                NetworkStatus::ProbeError,
                None,
            ),
        ] {
            let (url, _log, server) = serve_regional(scripts);
            let clients = (0..REGIONAL_CONNECTIONS)
                .filter_map(|_| {
                    Client::builder()
                        .no_proxy()
                        .redirect(Policy::none())
                        .timeout(Duration::from_millis(80))
                        .build()
                        .ok()
                })
                .collect();
            let mut regions = [RegionalState::new(&url, false, clients)];
            let phase = runtime.block_on(sample_regions(&mut regions, Duration::ZERO));
            let result = regional_observation(&regions[0], phase);
            assert_eq!(result.status, status);
            assert_eq!(result.http_status, code);
            assert!(result.regional.is_none());
            server.join().unwrap();
        }
    }

    #[test]
    fn regional_rounds_interleave_regions_in_reading_order() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let script = vec![(404u16, Duration::ZERO, false); 4];
        let mut servers = Vec::new();
        for _ in 0..4 {
            servers.push(serve_regional(vec![script.clone(); 3]));
        }
        let mut regions: Vec<RegionalState> = servers
            .iter()
            .map(|(url, _, _)| loopback_region(url, REGIONAL_CONNECTIONS))
            .collect();
        let phase = runtime.block_on(sample_regions(&mut regions, REGIONAL_SAMPLE_SPACING));
        for region in &regions {
            let result = regional_observation(region, phase);
            let regional = result.regional.as_ref().expect("all regions complete");
            assert_eq!(regional.samples, 9);
            assert_eq!(regional.connections, 3);
        }
        for (_, log, _) in &servers {
            wait_for_arrivals(log, 12);
        }
        let logs: Vec<std::sync::Arc<std::sync::Mutex<Vec<Instant>>>> =
            servers.iter().map(|(_, log, _)| log.clone()).collect();
        for (_, _, server) in servers {
            server.join().unwrap();
        }
        // Warm-ups are the first three arrivals per region; the next nine are
        // the interleaved samples, sorted by their recorded instants.
        let samples: Vec<Vec<Instant>> = logs
            .iter()
            .map(|log| {
                let mut arrivals = log.lock().unwrap().clone();
                arrivals.sort_unstable();
                arrivals.into_iter().skip(3).collect()
            })
            .collect();
        for round in 0..REGIONAL_ROUNDS {
            for region in 0..3 {
                assert!(
                    samples[region][round] < samples[region + 1][round],
                    "round {round} must follow region order"
                );
            }
            if round + 1 < REGIONAL_ROUNDS {
                assert!(samples[3][round] < samples[0][round + 1]);
            }
        }
        for (region, region_samples) in samples.iter().enumerate() {
            for round in 0..REGIONAL_ROUNDS - 1 {
                let gap = region_samples[round + 1] - region_samples[round];
                assert!(
                    gap >= REGIONAL_SAMPLE_SPACING - Duration::from_millis(1),
                    "region {region} round {round} gap {gap:?}"
                );
            }
        }
    }

    #[test]
    fn regional_probe_order_follows_the_reading_order() {
        let urls = [
            "https://objectstorage.eu-amsterdam-1.oraclecloud.com/",
            "https://github.com/",
            "https://objectstorage.us-sanjose-1.oraclecloud.com/",
            "https://objectstorage.ap-tokyo-1.oraclecloud.com/",
            "https://objectstorage.us-ashburn-1.oraclecloud.com/",
        ]
        .map(String::from);
        assert_eq!(regional_probe_order(&urls), vec![3, 2, 4, 0]);
        assert_eq!(regional_probe_order(&urls[1..2]), Vec::<usize>::new());
        assert_eq!(regional_probe_order(&urls[..1]), vec![0]);
    }

    #[test]
    fn fake_ip_ranges_and_environment_proxies_flag_via_proxy() {
        let address = |text: &str| text.parse::<SocketAddr>().unwrap();
        assert!(any_fake_ip_address(&[
            address("198.18.0.1:443"),
            address("8.8.8.8:443"),
        ]));
        assert!(any_fake_ip_address(&[address("198.19.255.255:443")]));
        assert!(!any_fake_ip_address(&[address("198.17.255.255:443")]));
        assert!(!any_fake_ip_address(&[address("198.20.0.0:443")]));
        assert!(!any_fake_ip_address(&[address("[2001:db8::1]:443")]));
        let host = "objectstorage.ap-tokyo-1.oraclecloud.com";
        assert!(!environment_proxy_applies(None, None, None, host));
        assert!(environment_proxy_applies(
            Some("http://127.0.0.1:7890"),
            None,
            None,
            host
        ));
        assert!(environment_proxy_applies(
            None,
            Some("http://127.0.0.1:7890"),
            None,
            host
        ));
        assert!(!environment_proxy_applies(
            Some("http://127.0.0.1:7890"),
            None,
            Some("*"),
            host
        ));
        assert!(!environment_proxy_applies(
            Some("http://127.0.0.1:7890"),
            None,
            Some("oraclecloud.com"),
            host
        ));
        assert!(environment_proxy_applies(
            Some("http://127.0.0.1:7890"),
            None,
            Some("github.com"),
            host
        ));
    }

    #[test]
    fn nearest_rank_p90_indexes_the_ceil_rank() {
        for (count, expected) in [(5, 4), (6, 5), (7, 6), (8, 7), (9, 8)] {
            assert_eq!(nearest_rank_p90(count), expected);
        }
    }

    #[test]
    #[ignore = "measures the four live Oracle regional origins; run manually, never in CI"]
    fn regional_reference_real_network_once() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let urls = REGIONAL_REFERENCE_URLS.map(str::to_owned);
        let observations = runtime.block_on(async {
            let deadline = Instant::now() + REGIONAL_BATCH_TIMEOUT;
            measure_regional_references(&urls, deadline, REGIONAL_SAMPLE_SPACING).await
        });
        for observation in observations {
            let Some(regional) = observation.regional else {
                println!("{} -> {:?}", observation.url, observation.status);
                continue;
            };
            println!(
                "{} -> value {} ms, median {} ms, p90 {} ms, samples {}, connections {}, viaProxy {}",
                observation.url,
                observation.elapsed_ms,
                regional.median_ms,
                regional.p90_ms,
                regional.samples,
                regional.connections,
                regional.via_proxy
            );
        }
    }

    #[test]
    fn http_errors_and_redirects_are_not_success() {
        assert_eq!(classify_http(200), NetworkStatus::Reachable);
        assert_eq!(classify_http(302), NetworkStatus::Redirected);
        for code in [403, 404, 405, 429, 503] {
            assert_eq!(classify_http(code), NetworkStatus::HttpError);
        }
    }
    #[test]
    fn regional_references_never_follow_a_redirect_out_of_their_region() {
        for source in [
            "https://objectstorage.us-sanjose-1.oraclecloud.com/",
            "https://objectstorage.us-ashburn-1.oraclecloud.com/",
            "https://objectstorage.ap-tokyo-1.oraclecloud.com/",
            "https://objectstorage.eu-amsterdam-1.oraclecloud.com/",
        ] {
            assert!(!follow_website_redirect(
                source,
                "https://www.oracle.com/",
                1
            ));
        }
        assert!(follow_website_redirect(
            "https://github.com/",
            "https://github.com/login",
            1
        ));
        assert!(!follow_website_redirect(
            "https://github.com/",
            "https://github.com/login",
            4
        ));
        assert!(!follow_website_redirect(
            "https://github.com/",
            "http://github.com/",
            1
        ));
    }
    #[test]
    fn vendor_redirects_require_a_domain_boundary() {
        assert!(vendor_host(
            NetworkTarget::SteamStore,
            "store.steampowered.com"
        ));
        for host in [
            "steampowered.com.evil.test",
            "notsteampowered.com",
            "127.0.0.1",
            "unity.com",
        ] {
            assert!(!vendor_host(NetworkTarget::SteamStore, host));
        }
    }
}
