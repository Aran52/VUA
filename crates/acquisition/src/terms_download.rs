//! Credential-free, bounded public document transport. Tests use a private loopback endpoint.
use std::{
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};
use vua_bdl_store::terms::TermsLink;

pub const MAX_PDF_BYTES: usize = 20 * 1024 * 1024;
pub enum FetchOutcome {
    Pdf(Vec<u8>),
    Observed {
        status: &'static str,
        problem: &'static str,
    },
    Cancelled,
}
pub trait TermsTransport: Send + Sync {
    fn fetch(&self, link: &TermsLink, cancelled: &(dyn Fn() -> bool + Send + Sync))
        -> FetchOutcome;
}
type Pacer = Arc<Mutex<Option<Instant>>>;
pub struct PublicTermsDownloader {
    pacer: Pacer,
    interval: Duration,
    timeout: Duration,
    limit: usize,
    #[cfg(test)]
    endpoint: Option<String>,
}
impl Default for PublicTermsDownloader {
    fn default() -> Self {
        static PACER: OnceLock<Pacer> = OnceLock::new();
        Self {
            pacer: PACER.get_or_init(|| Arc::new(Mutex::new(None))).clone(),
            interval: Duration::from_secs(3),
            timeout: Duration::from_secs(30),
            limit: MAX_PDF_BYTES,
            #[cfg(test)]
            endpoint: None,
        }
    }
}

fn observed(status: &'static str, problem: &'static str) -> FetchOutcome {
    FetchOutcome::Observed { status, problem }
}
pub fn public_download_url(link: &TermsLink) -> Option<String> {
    let id = link.google_file_id.as_ref()?;
    if id.is_empty()
        || id.len() > 200
        || !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"_-".contains(&c))
    {
        return None;
    }
    match link.source_kind.as_str() {
        "drive" => Some(format!(
            "https://drive.usercontent.google.com/download?id={id}&export=download"
        )),
        "docs" => Some(format!(
            "https://docs.google.com/document/d/{id}/export?format=pdf"
        )),
        _ => None,
    }
}

impl TermsTransport for PublicTermsDownloader {
    fn fetch(
        &self,
        link: &TermsLink,
        cancelled: &(dyn Fn() -> bool + Send + Sync),
    ) -> FetchOutcome {
        let Some(url) = public_download_url(link) else {
            return observed("unsupported", "unsupported_source");
        };
        #[cfg(test)]
        let url = self.endpoint.clone().unwrap_or(url);
        let mut guard = loop {
            if cancelled() {
                return FetchOutcome::Cancelled;
            }
            match self.pacer.try_lock() {
                Ok(guard) => break guard,
                Err(std::sync::TryLockError::Poisoned(_)) => {
                    return observed("failed", "transport_unavailable")
                }
                Err(std::sync::TryLockError::WouldBlock) => {
                    std::thread::sleep(Duration::from_millis(20))
                }
            }
        };
        while guard.is_some_and(|ended| ended.elapsed() < self.interval) {
            if cancelled() {
                return FetchOutcome::Cancelled;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        let result = match tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
        {
            Ok(runtime) => runtime.block_on(self.request(&url, cancelled)),
            Err(_) => observed("failed", "transport_unavailable"),
        };
        *guard = Some(Instant::now());
        result
    }
}

impl PublicTermsDownloader {
    async fn request(
        &self,
        url: &str,
        cancelled: &(dyn Fn() -> bool + Send + Sync),
    ) -> FetchOutcome {
        let client = match reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(self.timeout)
            .build()
        {
            Ok(client) => client,
            Err(_) => return observed("failed", "transport_unavailable"),
        };
        let work = async {
            let mut response = match client
                .get(url)
                .header(reqwest::header::ACCEPT, "application/pdf")
                .header(reqwest::header::USER_AGENT, "VUA-VN3-terms/1")
                .send()
                .await
            {
                Ok(response) => response,
                Err(e) => {
                    return observed(
                        "failed",
                        if e.is_timeout() {
                            "timeout"
                        } else {
                            "network_failure"
                        },
                    )
                }
            };
            let status = response.status();
            if status.is_redirection() {
                let login = response
                    .headers()
                    .get(reqwest::header::LOCATION)
                    .and_then(|v| v.to_str().ok())
                    .is_some_and(|v| v.starts_with("https://accounts.google.com/"));
                return observed(
                    if login { "not_public" } else { "unsupported" },
                    if login {
                        "login_required"
                    } else {
                        "redirect_not_supported"
                    },
                );
            }
            if [401, 403, 404].contains(&status.as_u16()) {
                return observed("not_public", "not_public");
            }
            if !status.is_success() {
                return observed("failed", "http_error");
            }
            if response
                .content_length()
                .is_some_and(|n| n > self.limit as u64)
            {
                return observed("failed", "size_limit");
            }
            let mut bytes = Vec::new();
            loop {
                let chunk = match response.chunk().await {
                    Ok(chunk) => chunk,
                    Err(e) => {
                        return observed(
                            "failed",
                            if e.is_timeout() {
                                "timeout"
                            } else {
                                "network_failure"
                            },
                        )
                    }
                };
                let Some(chunk) = chunk else { break };
                if bytes.len().saturating_add(chunk.len()) > self.limit {
                    return observed("failed", "size_limit");
                }
                bytes.extend_from_slice(&chunk);
            }
            if bytes.starts_with(b"%PDF-") {
                return FetchOutcome::Pdf(bytes);
            }
            let text = String::from_utf8_lossy(&bytes).to_ascii_lowercase();
            let private = [
                "accounts.google.com",
                "you need access",
                "request access",
                "sign in",
                "virus scan",
            ]
            .iter()
            .any(|m| text.contains(m));
            observed(
                if private { "not_public" } else { "unsupported" },
                if private { "not_public" } else { "not_pdf" },
            )
        };
        tokio::pin!(work);
        let deadline = tokio::time::sleep(self.timeout);
        tokio::pin!(deadline);
        loop {
            tokio::select! {
                value=&mut work=>return value,
                _=&mut deadline=>return observed("failed","timeout"),
                _=tokio::time::sleep(Duration::from_millis(20))=>if cancelled(){return FetchOutcome::Cancelled;},
            }
        }
    }
}

#[cfg(test)]
#[path = "terms_download_tests.rs"]
mod tests;
