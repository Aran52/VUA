//! Advisory read caches. Execution still verifies selected input bytes independently.
use super::{err, AppErrorV1, ProductionEditor, ProductionSource};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    sync::Mutex,
    time::UNIX_EPOCH,
};

const HASH_CACHE_FILE: &str = "material-hashes.json";
const MAX_CACHE_BYTES: usize = 4 * 1024 * 1024;
const MAX_HASHES: usize = 8192;
const MAX_EDITORS: usize = 128;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct FileStamp {
    size: u64,
    modified_secs: u64,
    modified_nanos: u32,
}

pub(super) fn file_stamp(path: &Path) -> Option<FileStamp> {
    let metadata = fs::symlink_metadata(path).ok()?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return None;
    }
    let modified = metadata.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    Some(FileStamp {
        size: metadata.len(),
        modified_secs: modified.as_secs(),
        modified_nanos: modified.subsec_nanos(),
    })
}

fn hash_key(path: &Path, member: Option<&str>) -> String {
    let mut digest = Sha256::new();
    digest.update(path.as_os_str().as_encoded_bytes());
    digest.update([0]);
    if let Some(member) = member {
        digest.update(member.as_bytes());
    }
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[derive(Clone, Serialize, Deserialize)]
struct HashEntry {
    stamp: FileStamp,
    sha256: String,
}

#[derive(Serialize, Deserialize)]
struct HashDocument {
    version: u8,
    entries: BTreeMap<String, HashEntry>,
}

#[derive(Clone, Default)]
pub(super) struct MaterialHashes {
    entries: BTreeMap<String, HashEntry>,
    changed: BTreeSet<String>,
}

impl MaterialHashes {
    pub(super) fn known(&self, path: &Path, member: Option<&str>) -> Option<String> {
        let stamp = file_stamp(path)?;
        self.entries
            .get(&hash_key(path, member))
            .filter(|entry| entry.stamp == stamp)
            .map(|entry| entry.sha256.clone())
    }

    /// The source stamp refers to the archive, while the digest refers to its selected member.
    pub(super) fn digest(
        &mut self,
        path: &Path,
        member: Option<&str>,
        compute: impl FnOnce() -> Result<String, AppErrorV1>,
    ) -> Result<String, AppErrorV1> {
        let before = file_stamp(path);
        let key = hash_key(path, member);
        if let Some(entry) = self
            .entries
            .get(&key)
            .filter(|entry| Some(&entry.stamp) == before.as_ref())
        {
            return Ok(entry.sha256.clone());
        }
        let computed = compute()?;
        let digest = computed.strip_prefix("sha256:").unwrap_or(&computed);
        if !valid_hash(digest) {
            return Err(err(
                "source_invalid",
                "Material hash is not a SHA-256 digest",
            ));
        }
        if let Some(stamp) = before {
            if file_stamp(path).as_ref() != Some(&stamp) {
                return Err(err(
                    "source_drift",
                    "Material changed during hashing; retry the read",
                ));
            }
            self.entries.insert(
                key.clone(),
                HashEntry {
                    stamp,
                    sha256: digest.into(),
                },
            );
            self.changed.insert(key);
        }
        Ok(digest.into())
    }

    fn read(root: &Path) -> Self {
        let load = || -> Option<HashDocument> {
            let mut bytes = Vec::new();
            fs::File::open(root.join(HASH_CACHE_FILE))
                .ok()?
                .take(MAX_CACHE_BYTES as u64 + 1)
                .read_to_end(&mut bytes)
                .ok()?;
            if bytes.len() > MAX_CACHE_BYTES {
                return None;
            }
            let document: HashDocument = serde_json::from_slice(&bytes).ok()?;
            (document.version == 1 && document.entries.len() <= MAX_HASHES).then_some(document)
        };
        let Some(mut document) = load() else {
            return Self::default();
        };
        document.entries.retain(|key, entry| {
            valid_hash(key)
                && valid_hash(&entry.sha256)
                && entry.stamp.modified_nanos < 1_000_000_000
        });
        Self {
            entries: document.entries,
            changed: BTreeSet::new(),
        }
    }

    fn write(&self, root: &Path) -> io::Result<()> {
        let bytes = serde_json::to_vec(&HashDocument {
            version: 1,
            entries: self.entries.clone(),
        })?;
        if bytes.len() > MAX_CACHE_BYTES {
            return Ok(());
        }
        fs::create_dir_all(root)?;
        let target = root.join(HASH_CACHE_FILE);
        let pending = target.with_extension("pending");
        let result = (|| {
            fs::write(&pending, bytes)?;
            fs::rename(&pending, &target)
        })();
        if result.is_err() {
            let _ = fs::remove_file(pending);
        }
        result
    }
}

struct SourcesMemo {
    key: String,
    sources: Vec<ProductionSource>,
}

type EditorMemo = BTreeMap<PathBuf, (FileStamp, Option<ProductionEditor>)>;

#[derive(Default)]
pub struct ProductionReadCache {
    hashes: Mutex<Option<MaterialHashes>>,
    sources: Mutex<Option<SourcesMemo>>,
    editors: Mutex<EditorMemo>,
}

impl ProductionReadCache {
    /// A query hashes outside the shared lock so other readers do not wait for file contents.
    pub(super) fn material_hashes(&self, root: &Path) -> MaterialHashes {
        self.hashes
            .lock()
            .expect("material hash cache poisoned")
            .get_or_insert_with(|| MaterialHashes::read(root))
            .clone()
    }

    pub(super) fn remember_hashes(&self, root: &Path, incoming: MaterialHashes) {
        if incoming.changed.is_empty() {
            return;
        }
        let mut guard = self.hashes.lock().expect("material hash cache poisoned");
        let current = guard.get_or_insert_with(|| MaterialHashes::read(root));
        for key in incoming.changed {
            if let Some(entry) = incoming.entries.get(&key) {
                current.entries.insert(key, entry.clone());
            }
        }
        while current.entries.len() > MAX_HASHES {
            current.entries.pop_first();
        }
        // This is an optimization. An unwritable cache never discards a computed result.
        let _ = current.write(root);
    }

    pub(super) fn sources(&self, key: &str) -> Option<Vec<ProductionSource>> {
        self.sources
            .lock()
            .expect("source cache poisoned")
            .as_ref()
            .filter(|memo| memo.key == key)
            .map(|memo| memo.sources.clone())
    }

    pub(super) fn remember_sources(&self, key: String, sources: &[ProductionSource]) {
        *self.sources.lock().expect("source cache poisoned") = Some(SourcesMemo {
            key,
            sources: sources.to_vec(),
        });
    }

    pub(super) fn editor(
        &self,
        path: &Path,
        verify: impl FnOnce() -> Option<ProductionEditor>,
    ) -> Option<ProductionEditor> {
        // Directory inputs are still resolved and verified normally; known executable paths
        // are reusable only while their actual file metadata is unchanged.
        let Some(before) = file_stamp(path) else {
            return verify();
        };
        if let Some((_, result)) = self
            .editors
            .lock()
            .expect("editor cache poisoned")
            .get(path)
            .filter(|(stamp, _)| stamp == &before)
        {
            return result.clone();
        }
        let result = verify();
        if file_stamp(path).as_ref() == Some(&before) {
            let mut memo = self.editors.lock().expect("editor cache poisoned");
            memo.insert(path.to_owned(), (before, result.clone()));
            while memo.len() > MAX_EDITORS {
                memo.pop_first();
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;
    use std::{
        cell::Cell,
        sync::{Arc, Barrier},
        time::{Duration, SystemTime},
    };

    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!(
                "vua-read-cache-{}-{}",
                std::process::id(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir(&root).unwrap();
            Self(root)
        }
        fn file(&self, name: &str) -> PathBuf {
            let path = self.0.join(name);
            fs::write(&path, b"synthetic").unwrap();
            path
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }
    fn hash(byte: char) -> String {
        byte.to_string().repeat(64)
    }

    #[test]
    fn hashes_reuse_matching_metadata_and_survive_restart() {
        let fixture = Fixture::new();
        let path = fixture.file("avatar.unitypackage");
        let cache = ProductionReadCache::default();
        let mut hashes = cache.material_hashes(&fixture.0);
        assert_eq!(
            hashes
                .digest(&path, None, || Ok(format!("sha256:{}", hash('a'))))
                .unwrap(),
            hash('a')
        );
        assert_eq!(
            hashes
                .digest(&path, None, || panic!("warm read must not hash contents"))
                .unwrap(),
            hash('a')
        );
        cache.remember_hashes(&fixture.0, hashes);
        assert!(!fixture.0.join("material-hashes.pending").exists());
        let restarted = ProductionReadCache::default();
        assert_eq!(
            restarted.material_hashes(&fixture.0).known(&path, None),
            Some(hash('a'))
        );
    }

    #[test]
    fn file_size_time_rename_and_removal_invalidate_hashes() {
        let fixture = Fixture::new();
        let path = fixture.file("avatar.unitypackage");
        let mut hashes = MaterialHashes::default();
        hashes.digest(&path, None, || Ok(hash('a'))).unwrap();
        fs::write(&path, b"longer synthetic content").unwrap();
        assert_eq!(hashes.known(&path, None), None);
        hashes.digest(&path, None, || Ok(hash('b'))).unwrap();
        let changed = fs::metadata(&path).unwrap().modified().unwrap() + Duration::from_secs(2);
        fs::File::options()
            .write(true)
            .open(&path)
            .unwrap()
            .set_modified(changed)
            .unwrap();
        assert_eq!(
            hashes.known(&path, None),
            None,
            "same size with a changed timestamp is stale"
        );
        hashes.digest(&path, None, || Ok(hash('c'))).unwrap();
        let renamed = fixture.0.join("renamed.unitypackage");
        fs::rename(&path, &renamed).unwrap();
        assert_eq!(hashes.known(&path, None), None);
        assert_eq!(hashes.known(&renamed, None), None);
        hashes.digest(&renamed, None, || Ok(hash('d'))).unwrap();
        fs::remove_file(&renamed).unwrap();
        assert_eq!(hashes.known(&renamed, None), None);
    }

    #[test]
    fn archive_members_are_separate_and_follow_the_archive_stamp() {
        let fixture = Fixture::new();
        let path = fixture.file("materials.zip");
        let mut hashes = MaterialHashes::default();
        for (member, byte) in [
            (None, 'a'),
            (Some("avatar.unitypackage"), 'b'),
            (Some("outfit.unitypackage"), 'c'),
        ] {
            hashes.digest(&path, member, || Ok(hash(byte))).unwrap();
            assert_eq!(hashes.known(&path, member), Some(hash(byte)));
        }
        fs::write(&path, b"changed archive").unwrap();
        for member in [
            None,
            Some("avatar.unitypackage"),
            Some("outfit.unitypackage"),
        ] {
            assert_eq!(hashes.known(&path, member), None);
        }
    }

    #[test]
    fn changes_during_hashing_and_invalid_digests_are_never_remembered() {
        let fixture = Fixture::new();
        let path = fixture.file("avatar.unitypackage");
        let mut hashes = MaterialHashes::default();
        let error = hashes
            .digest(&path, None, || {
                fs::write(&path, b"changed while reading").unwrap();
                Ok(hash('a'))
            })
            .unwrap_err();
        assert_eq!(error.code, "vua.amf_production.source_drift");
        assert_eq!(hashes.known(&path, None), None);
        assert_eq!(
            hashes
                .digest(&path, None, || Ok("invalid".into()))
                .unwrap_err()
                .code,
            "vua.amf_production.source_invalid"
        );
        assert!(hashes.entries.is_empty());
        assert!(
            file_stamp(&fixture.0).is_none(),
            "unknown metadata cannot identify file contents"
        );
    }

    #[test]
    fn missing_corrupt_future_and_oversized_documents_fall_back_to_hashing() {
        let fixture = Fixture::new();
        let target = fixture.0.join(HASH_CACHE_FILE);
        assert!(MaterialHashes::read(&fixture.0).entries.is_empty());
        for bytes in [
            b"broken".to_vec(),
            br#"{"version":2,"entries":{}}"#.to_vec(),
            vec![b' '; MAX_CACHE_BYTES + 1],
        ] {
            fs::write(&target, bytes).unwrap();
            assert!(MaterialHashes::read(&fixture.0).entries.is_empty());
        }
        let path = fixture.file("material");
        let mut hashes = MaterialHashes::default();
        hashes.digest(&path, None, || Ok(hash('a'))).unwrap();
        hashes.write(&fixture.0).unwrap();
        let mut document: Value = serde_json::from_slice(&fs::read(&target).unwrap()).unwrap();
        document["entries"][hash_key(&path, None)]["sha256"] = serde_json::json!("bad");
        fs::write(&target, serde_json::to_vec(&document).unwrap()).unwrap();
        assert!(MaterialHashes::read(&fixture.0).entries.is_empty());
    }

    #[test]
    fn an_unwritable_cache_keeps_the_computed_result() {
        let fixture = Fixture::new();
        let path = fixture.file("material");
        let root = fixture.file("not-a-directory");
        let cache = ProductionReadCache::default();
        let mut hashes = cache.material_hashes(&root);
        hashes.digest(&path, None, || Ok(hash('a'))).unwrap();
        cache.remember_hashes(&root, hashes);
        assert_eq!(
            cache.material_hashes(&root).known(&path, None),
            Some(hash('a'))
        );
    }

    #[test]
    fn concurrent_readers_merge_independent_hashes_without_lost_entries() {
        let fixture = Fixture::new();
        let path = fixture.file("materials.zip");
        let cache = Arc::new(ProductionReadCache::default());
        let gate = Arc::new(Barrier::new(2));
        std::thread::scope(|scope| {
            for (member, byte) in [("avatar", 'a'), ("outfit", 'b')] {
                let cache = cache.clone();
                let gate = gate.clone();
                let root = &fixture.0;
                let path = &path;
                scope.spawn(move || {
                    let mut hashes = cache.material_hashes(root);
                    gate.wait();
                    hashes
                        .digest(path, Some(member), || Ok(hash(byte)))
                        .unwrap();
                    cache.remember_hashes(root, hashes);
                });
            }
        });
        let hashes = MaterialHashes::read(&fixture.0);
        assert_eq!(hashes.known(&path, Some("avatar")), Some(hash('a')));
        assert_eq!(hashes.known(&path, Some("outfit")), Some(hash('b')));
    }

    #[test]
    fn editor_cache_rechecks_changes_removals_and_directory_inputs() {
        let fixture = Fixture::new();
        let path = fixture.file("Unity.exe");
        let cache = ProductionReadCache::default();
        let reads = Cell::new(0);
        let verify = || {
            reads.set(reads.get() + 1);
            Some(ProductionEditor {
                path: path.to_string_lossy().into(),
                version: "2022.3.22f1".into(),
            })
        };
        assert!(cache.editor(&path, verify).is_some());
        assert!(cache.editor(&path, verify).is_some());
        assert_eq!(reads.get(), 1);
        fs::write(&path, b"different executable").unwrap();
        cache.editor(&path, verify);
        assert_eq!(reads.get(), 2);
        fs::remove_file(&path).unwrap();
        assert!(cache.editor(&path, || None).is_none());
        cache.editor(&fixture.0, verify);
        cache.editor(&fixture.0, verify);
        assert_eq!(
            reads.get(),
            4,
            "directories are resolved again on every query"
        );
        fixture.file("Unity.exe");
        assert!(cache.editor(&path, || None).is_none());
        assert!(cache
            .editor(&path, || panic!("matching failed probe should be cached"))
            .is_none());
    }
}
