# AMF data recovery v0.1

> Version: 0.1 · Candidate · Owner: Integration · Reviewed: 2026-10-26

`amf-data/0.1` coordinates an offline AMF database snapshot and migration trial. AMF owns SQLite
content and migration. Distribution coordinates signed component selections and opaque recovery
receipts; Main supplies its actual profile directory. Renderer code cannot supply a path, token,
executable or maintenance command. The frozen BDL formats and installed layout remain unchanged.

## Admission and snapshot

The signed AMF UI metadata and executable identity must advertise the same recovery ABI. The
selected host must carry a signed `dist/renderer/update-runtime.json` identifying its source
build and data-trial fence. The authenticated root launcher must support data recovery v0.1.
Older combinations require the transition installer before writer-changing activation.

Main registers the profile against the current manifest pair before requesting activation.
After all AMF writers stop, native code records the prepared activation journal and a separate
data intent binding both manifest pairs, profile, fresh token and binding digest. Distribution
invokes the verified target provider's closed `--data-snapshot`, `--data-migrate`,
`--data-restore` and `--data-commit` entries. These execute before normal provider startup.

AMF reads its existing registration to select legacy or isolated data layout. Its closed
allowlist contains the BDL database and AMF task database only. SQLite's backup API includes WAL
content, checks integrity and flushes independent snapshots. Missing databases are recorded as
absent. Private snapshots live under the profile's `modules/amf/update-backups/<token>` and carry
length/digest receipts. Native code retains a digest of the immutable database inventory and
requires it on every later maintenance call, including absent-database and layout facts.
Source/destination links, traversal, foreign identities, duplicate
maintenance and incomplete/corrupt backups fail closed. Assets, Unity projects, accounts and
host task databases are never copied or restored.

## Trial, health and recovery

The durable data intent progresses from `snapshotting` to `prepared` before migration or new
selection can run. AMF executes its existing transactional BDL migration chain offline, including
for disabled AMF, while the snapshot is still restorable. Disabled AMF is not started or enabled
merely to update it. An incomplete snapshot cannot be mistaken for an activated data trial.

During restart verification, Main fences AMF user calls, shell work, enable/disable and further
updates. Host queries remain usable. Actual provider identity and a candidate UI import must
succeed before acknowledging the exact installed pair. This prevents rollback from erasing
user work performed between migration and health acknowledgement.

Before health, failure stops the candidate, verifies every backup, restores only the two allowed
database paths, removes trial WAL/SHM files, and restores selection. A restored absent database
returns to absence. No old AMF reader opens the migrated database to attempt recovery. Restore
can resume after interruption and completed retries do not erase later work. Snapshotting-only
failure with unchanged selection can be abandoned without restoring data.

The activation journal's healthy state is the commit point. Backup commit is retried if its reply
or final metadata write is lost. Committed backup receipts refuse restore. An explicit later
component rollback still requires the retained reader to accept the current writer; it does not
rewind a committed snapshot over newer user work. Incompatible rollback requires repair.

Receipts and snapshots stay local for recovery inspection. No upload or automatic removal of
unknown files is authorized. Actual low-disk/interruption behavior, signed remote delivery and
live process acceptance remain U6 evidence gates; synthetic database checks do not pass them.
