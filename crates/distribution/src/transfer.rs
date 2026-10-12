use crate::{
    manifest::{FileObject, Pack},
    paths::{digest, file_digest, owned_path},
    Error, Result,
};
use serde::Serialize;
use std::{
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub phase: String,
    pub completed: u64,
    pub total: u64,
    pub bytes_per_second: u64,
    pub reused_bytes: u64,
    pub full_pack: bool,
}
pub struct Transfer {
    pub cancel: Arc<AtomicBool>,
    pub report: Box<dyn Fn(Progress) + Send + Sync>,
    client: reqwest::Client,
    started: Instant,
    received: u64,
    reused: u64,
    full_pack: bool,
}
impl Transfer {
    pub fn new(
        cancel: Arc<AtomicBool>,
        report: impl Fn(Progress) + Send + Sync + 'static,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .https_only(true)
            .connect_timeout(Duration::from_secs(8))
            .read_timeout(Duration::from_secs(20))
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() >= 6 || attempt.url().scheme() != "https" {
                    attempt.stop()
                } else {
                    attempt.follow()
                }
            }))
            .user_agent("VUA-distribution/0.1")
            .build()?;
        Ok(Self {
            cancel,
            report: Box::new(report),
            client,
            started: Instant::now(),
            received: 0,
            reused: 0,
            full_pack: false,
        })
    }
    pub fn checkpoint(&self) -> Result<()> {
        if self.cancel.load(Ordering::Relaxed) {
            Err(Error("cancelled".into()))
        } else {
            Ok(())
        }
    }
    pub fn progress(&self, phase: &str, completed: u64, total: u64) {
        (self.report)(Progress {
            phase: phase.into(),
            completed,
            total,
            bytes_per_second: (self.received as f64
                / self.started.elapsed().as_secs_f64().max(0.001))
                as u64,
            reused_bytes: self.reused,
            full_pack: self.full_pack,
        });
    }
    async fn interruptible<T>(
        &self,
        future: impl std::future::Future<Output = reqwest::Result<T>>,
    ) -> Result<T> {
        tokio::pin!(future);
        loop {
            tokio::select! { value = &mut future => return Ok(value?), _ = tokio::time::sleep(Duration::from_millis(100)) => self.checkpoint()? }
        }
    }
    pub async fn manifest(&self, url: &str) -> Result<Vec<u8>> {
        let mut response = self.interruptible(self.client.get(url).send()).await?;
        if !response.status().is_success() {
            return Err(Error("feed_unavailable".into()));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = self.interruptible(response.chunk()).await? {
            if bytes.len() + chunk.len() > crate::manifest::MAX_MANIFEST_BYTES {
                return Err(Error("manifest_too_large".into()));
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
    /// Bounded sample probes test the exact object, never an ICMP or mirror homepage.
    async fn ranked_sources(&self, pack: &Pack) -> Vec<String> {
        let mut sources = Vec::new();
        let mut probes = tokio::task::JoinSet::new();
        for source in &pack.sources {
            let client = self.client.clone();
            let source = source.clone();
            let total = pack.bytes;
            probes.spawn(async move {
                let start = Instant::now();
                let probe = async {
                    let end = (total - 1).min(65535);
                    let mut response = client
                        .get(&source)
                        .header("Range", format!("bytes=0-{end}"))
                        .header("Accept-Encoding", "identity")
                        .send()
                        .await?;
                    if !range_matches(&response, 0, end, total) {
                        return Ok::<bool, reqwest::Error>(false);
                    }
                    let sample = response.chunk().await?;
                    Ok(sample.is_some_and(|chunk| !chunk.is_empty() && chunk.len() <= 65536))
                };
                let elapsed = tokio::time::timeout(Duration::from_secs(5), probe)
                    .await
                    .is_ok_and(|r| r.unwrap_or(false))
                    .then(|| start.elapsed());
                (elapsed, source)
            });
        }
        while !probes.is_empty() {
            tokio::select! {
                result = probes.join_next() => { if let Some(Ok((Some(elapsed), source))) = result { sources.push((elapsed, source)); } }
                _ = tokio::time::sleep(Duration::from_millis(100)) => { if self.checkpoint().is_err() { probes.abort_all(); break; } }
            }
        }
        sources.sort_by_key(|source| source.0);
        // A source that timed out during a short probe may still be usable for the download.
        for source in &pack.sources {
            if !sources.iter().any(|(_, url)| url == source) {
                sources.push((Duration::MAX, source.clone()));
            }
        }
        sources.into_iter().map(|(_, source)| source).collect()
    }
    #[cfg(test)]
    pub(crate) fn loopback(
        cancel: Arc<AtomicBool>,
        report: impl Fn(Progress) + Send + Sync + 'static,
    ) -> Self {
        let mut transfer = Self::new(cancel, report).unwrap();
        transfer.client = reqwest::Client::builder()
            .no_proxy()
            .connect_timeout(Duration::from_secs(2))
            .read_timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        transfer
    }
    pub async fn archive(&mut self, pack: &Pack, cache: &Path) -> Result<()> {
        if verified_file(cache, pack.bytes, &pack.sha256) {
            self.reused += pack.bytes;
            return Ok(());
        }
        let pending = cache.with_extension("partial");
        for source in self.ranked_sources(pack).await {
            self.checkpoint()?;
            let result = self.download_whole(&source, pack, &pending).await;
            if result.is_ok() {
                fs::rename(&pending, cache)?;
                return Ok(());
            }
            if result.as_ref().err().is_some_and(|e| e.0 == "cancelled") {
                return result;
            }
        }
        Err(Error("sources_failed_retry_or_offline".into()))
    }
    async fn download_whole(&mut self, source: &str, pack: &Pack, target: &Path) -> Result<()> {
        // Append only when the server proves the same signed object's total and offset.
        let mut offset = fs::metadata(target).map_or(0, |m| m.len());
        if offset >= pack.bytes {
            if verified_file(target, pack.bytes, &pack.sha256) {
                return Ok(());
            }
            fs::remove_file(target)?;
            offset = 0;
        }
        let request = self
            .client
            .get(source)
            .header("Accept-Encoding", "identity");
        let mut response = self
            .interruptible(
                if offset > 0 {
                    request.header("Range", format!("bytes={offset}-"))
                } else {
                    request
                }
                .send(),
            )
            .await?;
        let resume = offset > 0 && range_matches(&response, offset, pack.bytes - 1, pack.bytes);
        if !response.status().is_success() {
            return Err(Error("source_unavailable".into()));
        }
        // An unproved partial response cannot safely be written as a complete object.
        if response.status() == reqwest::StatusCode::PARTIAL_CONTENT && !resume {
            return Err(Error("invalid_content_range".into()));
        }
        let mut file = fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(!resume)
            .append(resume)
            .open(target)?;
        let mut count = if resume { offset } else { 0 };
        while let Some(chunk) = self.interruptible(response.chunk()).await? {
            self.checkpoint()?;
            count += chunk.len() as u64;
            if count > pack.bytes {
                return Err(Error("unexpected_download_length".into()));
            }
            file.write_all(&chunk)?;
            self.received += chunk.len() as u64;
            self.progress("downloading", count, pack.bytes);
        }
        file.sync_all()?;
        drop(file);
        self.progress("verifying", count, pack.bytes);
        if !verified_file(target, pack.bytes, &pack.sha256) {
            fs::remove_file(target)?;
            return Err(Error("download_integrity_failed".into()));
        }
        Ok(())
    }
    /// Reuse verified unchanged files. Range objects contain only changed file bytes.
    /// If a server cannot supply ranges, explicitly report one verified full-pack fallback.
    pub async fn component(
        &mut self,
        files: &[FileObject],
        pack: &Pack,
        previous: Option<&Path>,
        output: &Path,
        local_pack: Option<&Path>,
        cache: &Path,
    ) -> Result<()> {
        fs::create_dir_all(output)?;
        let total: u64 = files.iter().map(|file| file.bytes).sum();
        let mut completed = 0;
        let mut full = local_pack
            .filter(|file| verified_file(file, pack.bytes, &pack.sha256))
            .map(Path::to_path_buf);
        for file in files {
            self.checkpoint()?;
            let target = owned_path(output, &file.path)?;
            if !verified_file(&target, file.bytes, &file.sha256) {
                let reusable = previous
                    .and_then(|root| owned_path(root, &file.path).ok())
                    .filter(|path| verified_file(path, file.bytes, &file.sha256));
                fs::create_dir_all(
                    target
                        .parent()
                        .ok_or_else(|| Error("invalid_release_path".into()))?,
                )?;
                if let Some(path) = reusable {
                    fs::copy(path, &target)?;
                    self.reused += file.bytes;
                } else if file.bytes == 0 {
                    fs::write(&target, [])?;
                } else {
                    let mut range_written = false;
                    if full.is_none() {
                        for source in &pack.sources {
                            self.checkpoint()?;
                            if let Ok(bytes) = self.file_object(source, file, pack.bytes).await {
                                fs::write(&target, bytes)?;
                                range_written = true;
                                break;
                            }
                        }
                    }
                    if !range_written {
                        if full.is_none() {
                            self.full_pack = true;
                            self.progress("full-pack", completed, total);
                            self.archive(pack, cache).await?;
                            full = Some(cache.to_path_buf());
                        }
                        let mut source = fs::File::open(
                            full.as_ref().ok_or_else(|| Error("missing_pack".into()))?,
                        )?;
                        source.seek(SeekFrom::Start(file.offset))?;
                        let mut dest = fs::File::create(&target)?;
                        let written = std::io::copy(&mut source.take(file.bytes), &mut dest)?;
                        dest.sync_all()?;
                        if written != file.bytes {
                            return Err(Error("incomplete_release_pack".into()));
                        }
                    }
                }
            } else {
                self.reused += file.bytes;
            }
            if !verified_file(&target, file.bytes, &file.sha256) {
                return Err(Error("component_integrity_failed".into()));
            }
            completed += file.bytes;
            self.progress("staging", completed, total);
        }
        Ok(())
    }
    async fn file_object(
        &mut self,
        source: &str,
        file: &FileObject,
        total: u64,
    ) -> Result<Vec<u8>> {
        let end = file.offset + file.bytes - 1;
        let mut response = self
            .interruptible(
                self.client
                    .get(source)
                    .header("Accept-Encoding", "identity")
                    .header("Range", format!("bytes={}-{}", file.offset, end))
                    .send(),
            )
            .await?;
        if !range_matches(&response, file.offset, end, total) {
            return Err(Error("range_unavailable".into()));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = self.interruptible(response.chunk()).await? {
            if bytes.len() as u64 + chunk.len() as u64 > file.bytes {
                return Err(Error("unexpected_download_length".into()));
            }
            self.received += chunk.len() as u64;
            bytes.extend_from_slice(&chunk);
            self.progress("downloading", bytes.len() as u64, file.bytes);
        }
        if bytes.len() as u64 != file.bytes || digest(&bytes) != file.sha256 {
            return Err(Error("download_integrity_failed".into()));
        }
        Ok(bytes)
    }
}
fn range_matches(response: &reqwest::Response, start: u64, end: u64, total: u64) -> bool {
    response.status() == reqwest::StatusCode::PARTIAL_CONTENT
        && response
            .headers()
            .get("Content-Range")
            .and_then(|value| value.to_str().ok())
            == Some(format!("bytes {start}-{end}/{total}").as_str())
}
pub fn verified_file(path: &Path, bytes: u64, sha256: &str) -> bool {
    fs::symlink_metadata(path)
        .is_ok_and(|meta| meta.is_file() && !meta.file_type().is_symlink() && meta.len() == bytes)
        && file_digest(path).is_ok_and(|value| value == sha256)
}
