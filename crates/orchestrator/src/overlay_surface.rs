//! The desktop Overlay Surface (M7, proposal 017 DRAFT — direction only,
//! nothing here is frozen): the read-only service projection an overlay
//! consumes. The overlay is never a business-logic host — it renders
//! stable snapshots and issues semantic actions through the existing
//! command faces; this module therefore only *projects* facts the
//! authority already serves, it never computes new ones.
//!
//! Boundary anchors (AGENTS architecture constraints, M7 outline row):
//! - overlays consume stable application services and never become
//!   business-logic hosts;
//! - an overlay failure must never block the desktop main line — the
//!   surface is stateless and read-only, so a broken overlay session
//!   cannot hold server-side state or writers;
//! - VR overlays stay outside this milestone (user ruling 2026-09-06);
//!   nothing here assumes a VR transport.
//!
//! What intentionally does NOT live here (kept for the cross-domain
//! proposal pending the desktop stance): the overlay transport/connection
//! face, subscription or push semantics, and any overlay-specific command
//! vocabulary. The read models below reuse the frozen word lists'
//! payloads verbatim — a field here that a frozen schema does not know
//! is a contract break.

use serde_json::Value;

/// The minimal read model an overlay renders for the task surface: the
/// same durable snapshots the desktop main line already sees
/// (`application.getSnapshot` / `task.list`), projected to the fields a
/// one-glance surface can render. No new facts, no aggregation beyond
/// ordering — honesty over convenience.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayTaskCard {
    pub task_id: String,
    pub state: String,
    pub correlation_id: String,
}

/// One production-status card half: the field-trimmed summary of the
/// *current* plan (proposal 017 batch 1). Every field is copied verbatim
/// from the stored plan document (production-use-case v0.2 plan face,
/// approved-plan 0.3 shape) — a field here the document does not carry is
/// a contract break; a field the document carries but this summary drops
/// is the intended trim (overlay renders a glance, not the document).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayPlanSummary {
    pub plan_id: String,
    /// The plan's own lifecycle word (`draft` / `approved` / `superseded`),
    /// passed through verbatim — the vocabulary belongs to the plan face,
    /// never re-derived here.
    pub plan_status: String,
    pub created_at: String,
    pub recipe_id: String,
}

/// The other production-status card half: the field-trimmed summary of
/// the *latest* build record. Same shape discipline as
/// [`OverlayPlanSummary`]; the four fields mirror the frozen
/// `record.list` entry exactly (buildId/planId/status/finishedAt), and
/// `status` passes the record face's own vocabulary through verbatim.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayRecordSummary {
    pub build_id: String,
    pub plan_id: String,
    pub status: String,
    pub finished_at: String,
}

/// One in-flight download row on the overlay download card (017 desktop
/// stance 3, batch 2). Every field is copied verbatim from the durable
/// task row (`dl-<downloadId>-a<attempt>`, the nine-state per-attempt task
/// the download face drives): `downloadId` is the task's own
/// `correlationId` (the port-assigned download identity), `state` passes
/// the task face's nine-state word through untouched, `updatedAt` is the
/// row's own RFC 3339 timestamp. The row deliberately carries **no byte
/// progress**: received/expected bytes live in the task *events* (the
/// `task.progressed` payload), not on the task row — the main-line
/// `TaskSnapshot` does not carry them either, and a projection that
/// reaches into the event stream to synthesize a progress figure would
/// cross from trimming into invention (the schema's `synthesized-progress`
/// negative vector pins the same discipline for the batch-1 card).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayDownloadProgress {
    pub download_id: String,
    pub state: String,
    pub updated_at: String,
}

/// The download/import progress card (017 desktop stance 3, batch 2). The
/// card holds the in-flight download attempts only — the presentation
/// stance is "rendered while items are in flight" (017 stance 3), so the
/// projection serves exactly the rows that decision consumes. Completed
/// deliveries stay off this card by the same stance: their authoritative
/// consumer is the import page's `downloads.listCompleted` read face, and
/// projecting them here would graft a second authority's read face onto
/// the overlay glance instead of trimming one. An empty `active_downloads`
/// is the honest empty card (no in-flight downloads), never an error.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayDownloadCard {
    pub active_downloads: Vec<OverlayDownloadProgress>,
}

/// The production-status card (017 desktop stance 3, batch 1): the
/// current-plan summary plus the latest build-record status. Both halves
/// are independent `None`s — a fresh environment has no plans and no
/// records, a planned-but-unbuilt one has only the plan; the honest empty
/// state is "the authority holds nothing", never a synthesized row.
///
/// "Current" and "latest" are defined HERE, on the service authority
/// side (017 batch-1 stance: the semantics are not derived across
/// sources by the consumer):
/// - **current plan** = the stored plan document with the
///   lexicographically greatest `createdAt`. Plan documents carry one
///   same-shaped RFC 3339 UTC timestamp, so string order IS time order —
///   the same ordering provenance the task store's `created_at` column
///   uses. The newest document is the newest planning activity, whatever
///   its lifecycle stage (`draft` included): the overlay renders activity,
///   and the lifecycle word travels alongside so the consumer never has
///   to guess;
/// - **latest record** = the stored record document with the
///   lexicographically greatest `finishedAt`, same timestamp provenance.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayProductionCard {
    pub current_plan: Option<OverlayPlanSummary>,
    pub latest_record: Option<OverlayRecordSummary>,
}

/// The read-only overlay service. Implementations wrap the authorities
/// (the SQLite task store and the production document stores today; the
/// frozen document faces later) and must stay side-effect free: every
/// call is a pure projection, so an overlay querying in a loop can never
/// change what it observes. The projection consequently carries **no
/// query instant and no aggregate revision** — a captured-at stamp or an
/// invented revision counter would make two unchanged queries observe
/// different payloads, breaking the pure-function discipline the
/// polling overlay depends on.
///
/// Read failures are passed through as typed errors, never folded into
/// an empty set — a store failure presented as "no tasks" would dress a
/// failure up as the honest empty state (honesty rule: failures are
/// presented as failures; the empty state is the final state only when
/// the authority actually returned nothing).
pub trait OverlayReadModel: Send + Sync {
    /// The task-surface cards, oldest first — the honest subset of
    /// `TaskSnapshot` an overlay renders (no revision bookkeeping, no
    /// cancel bookkeeping: those belong to the main-line surfaces).
    ///
    /// Errors: a store read failure propagates as
    /// [`crate::SqliteStoreError`] — the consumer (the overlay UI)
    /// renders the empty state and the failure state distinctly.
    fn task_cards(&self) -> Result<Vec<OverlayTaskCard>, crate::SqliteStoreError>;

    /// The production-status card (017 batch 1). Always returns a card:
    /// the card's two halves are `None` when the production authorities
    /// genuinely hold nothing, so only a *read failure* is an error.
    ///
    /// Errors: a document-store read failure propagates as
    /// [`std::io::Error`] — never folded into "no production activity".
    fn production_card(&self) -> Result<OverlayProductionCard, std::io::Error>;

    /// The download/import progress card (017 batch 2): the in-flight
    /// download attempts (`dl-` prefixed non-terminal tasks), oldest
    /// first. Always returns a card: an empty `active_downloads` is the
    /// honest "nothing in flight" state, so only a *read failure* is an
    /// error.
    ///
    /// Errors: a store read failure propagates as
    /// [`crate::SqliteStoreError`] — never folded into an empty card.
    fn download_card(&self) -> Result<OverlayDownloadCard, crate::SqliteStoreError>;
}

/// The first concrete read model: projects the durable task store plus
/// the production document stores (017 batch 1).
pub struct StoreOverlayReadModel {
    store: std::sync::Arc<crate::SqliteTaskStore>,
    plans: std::sync::Arc<crate::PlanDocumentStore>,
    records: std::sync::Arc<crate::RecipeRecordStore>,
}

impl StoreOverlayReadModel {
    pub fn new(
        store: std::sync::Arc<crate::SqliteTaskStore>,
        plans: std::sync::Arc<crate::PlanDocumentStore>,
        records: std::sync::Arc<crate::RecipeRecordStore>,
    ) -> Self {
        Self {
            store,
            plans,
            records,
        }
    }
}

impl OverlayReadModel for StoreOverlayReadModel {
    fn task_cards(&self) -> Result<Vec<OverlayTaskCard>, crate::SqliteStoreError> {
        // Ordering key provenance: `SqliteTaskStore::tasks` returns the
        // rows `ORDER BY created_at, task_id` — `created_at` is the task's
        // creation instant (an RFC 3339 timestamp written from
        // `NewTask.occurred_at`), so the store's own ordering IS the
        // enqueue order (oldest first). This projection adds no re-sorting
        // of its own: re-sorting by task_id here would silently replace
        // the enqueue order with hash-alphabetical order.
        let snapshots = self.store.tasks()?;
        Ok(snapshots
            .into_iter()
            .map(|snapshot| OverlayTaskCard {
                task_id: snapshot.task_id,
                state: serde_json::to_value(snapshot.state)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .unwrap_or_else(|| "unknown".to_owned()),
                correlation_id: snapshot.correlation_id,
            })
            .collect())
    }

    fn production_card(&self) -> Result<OverlayProductionCard, std::io::Error> {
        // Both authorities list whole documents; the projection picks the
        // current/latest by the timestamps the documents themselves carry
        // (see the struct doc: RFC 3339 UTC same-shape strings, so
        // lexicographic max = temporal max) and trims the rest. No
        // cross-source derivation happens: plan facts come from the plan
        // document, record facts from the record document.
        let plans = self.plans.list_documents()?;
        let current_plan = plans
            .into_iter()
            .max_by(|left, right| {
                let left_key = left.get("createdAt").and_then(Value::as_str).unwrap_or("");
                let right_key = right.get("createdAt").and_then(Value::as_str).unwrap_or("");
                left_key.cmp(right_key)
            })
            .map(|document| OverlayPlanSummary {
                plan_id: text_of(&document, "planId"),
                plan_status: text_of(&document, "status"),
                created_at: text_of(&document, "createdAt"),
                recipe_id: text_of(&document, "recipeId"),
            });
        let records = self.records.list_documents()?;
        let latest_record = records
            .into_iter()
            .max_by(|left, right| {
                let left_key = left.get("finishedAt").and_then(Value::as_str).unwrap_or("");
                let right_key = right
                    .get("finishedAt")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                left_key.cmp(right_key)
            })
            .map(|document| OverlayRecordSummary {
                build_id: text_of(&document, "buildId"),
                plan_id: text_of(&document, "planId"),
                status: text_of(&document, "status"),
                finished_at: text_of(&document, "finishedAt"),
            });
        Ok(OverlayProductionCard {
            current_plan,
            latest_record,
        })
    }

    fn download_card(&self) -> Result<OverlayDownloadCard, crate::SqliteStoreError> {
        // Same store, same ordering provenance as `task_cards`: the rows
        // arrive `ORDER BY created_at, task_id` (enqueue order, oldest
        // first) and the projection adds no re-sorting of its own. The
        // selection is the per-attempt download identity only — `dl-` is
        // the prefix the download face mints (`dl-<downloadId>-a<attempt>`)
        // — kept non-terminal, so a finished attempt leaves the card the
        // moment its terminal state lands (the presentation stance is
        // "while items are in flight", 017 stance 3). No cross-source
        // derivation: every field is the row's own.
        let active = self
            .store
            .tasks()?
            .into_iter()
            .filter(|task| task.task_id.starts_with("dl-") && !task.state.is_terminal())
            .map(|task| OverlayDownloadProgress {
                download_id: task.correlation_id,
                state: serde_json::to_value(task.state)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .unwrap_or_else(|| "unknown".to_owned()),
                updated_at: task.updated_at,
            })
            .collect::<Vec<_>>();
        Ok(OverlayDownloadCard {
            active_downloads: active,
        })
    }
}

/// Reads one document field as text; an absent or non-text field projects
/// as the empty string rather than being invented. The overlay face trims,
/// it never fabricates.
fn text_of(document: &serde_json::Value, field: &str) -> String {
    document
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn unique_database(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "vua-overlay-surface-{tag}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("tasks.sqlite")
    }

    fn unique_root(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "vua-overlay-production-{tag}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// A minimal stored plan document (approved-plan 0.3 face subset —
    /// exactly the fields the projection trims from). Published status is
    /// always `draft`; lifecycle acts (approve/supersede) flip it in store.
    fn plan_document(plan_id: &str, created_at: &str) -> Value {
        json!({
            "schemaVersion": "0.3",
            "planId": plan_id,
            "recipeId": "019e0000-0000-7000-8000-000000000001",
            "recipeRevision": 1,
            "createdAt": created_at,
            "approvedAt": "2026-09-12T00:00:00.000Z",
            "planHash": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "jobs": [],
            "status": "draft",
        })
    }

    /// A minimal stored record document (production-use-case v0.2 record
    /// face subset).
    fn record_document(build_id: &str, finished_at: &str, status: &str) -> Value {
        json!({
            "schemaVersion": "0.3",
            "buildId": build_id,
            "recipeId": "019e0000-0000-7000-8000-000000000001",
            "planId": "019e0000-0000-7000-8000-000000000301",
            "startedAt": "2026-09-12T00:00:00.000Z",
            "finishedAt": finished_at,
            "status": status,
            "jobs": [],
        })
    }

    #[test]
    fn task_cards_project_the_durable_snapshots_sorted_oldest_first() {
        let database = unique_database("cards");
        let store = std::sync::Arc::new(crate::SqliteTaskStore::open(&database).unwrap());
        let model = StoreOverlayReadModel::new(
            store,
            std::sync::Arc::new(crate::PlanDocumentStore::new(unique_root("cards-plans"))),
            std::sync::Arc::new(crate::RecipeRecordStore::new(unique_root("cards-records"))),
        );

        // 空态即终态: no tasks yet — the honest empty set, not an error.
        assert!(model.task_cards().unwrap().is_empty());

        // The projection is a pure function of the store: querying twice
        // without a mutation observes the same set (read-only discipline).
        assert_eq!(model.task_cards().unwrap(), model.task_cards().unwrap());
    }

    #[test]
    fn task_cards_order_follows_the_enqueue_time_not_the_id() {
        let database = unique_database("cards-order");
        let store = std::sync::Arc::new(crate::SqliteTaskStore::open(&database).unwrap());

        // Insert three tasks with DELIBERATELY out-of-order creation
        // instants and out-of-order id alphabet, so an id-sort would read
        // zulu / mike / alpha while the enqueue order is alpha / mike /
        // zulu.
        let seeds = [
            ("zulu", "2026-09-10T03:00:00.000Z"),
            ("mike", "2026-09-10T01:00:00.000Z"),
            ("alpha", "2026-09-10T02:00:00.000Z"),
        ];
        for (index, (suffix, occurred_at)) in seeds.iter().enumerate() {
            let new_task = crate::NewTask {
                task_id: format!("task-{suffix}"),
                correlation_id: format!("corr-{suffix}"),
                occurred_at: occurred_at.to_string(),
            };
            store
                .accept_idempotent_task(
                    "task.startDemo",
                    &format!("command-{suffix}"),
                    &format!("fingerprint-{index}"),
                    &new_task,
                    &serde_json::json!({"demo": true}),
                )
                .unwrap();
        }

        let model = StoreOverlayReadModel::new(
            store,
            std::sync::Arc::new(crate::PlanDocumentStore::new(unique_root("order-plans"))),
            std::sync::Arc::new(crate::RecipeRecordStore::new(unique_root("order-records"))),
        );
        let cards = model.task_cards().unwrap();
        let ids: Vec<&str> = cards.iter().map(|card| card.task_id.as_str()).collect();
        assert_eq!(
            ids,
            ["task-mike", "task-alpha", "task-zulu"],
            "cards must follow the enqueue instant (created_at ascending), never the id alphabet"
        );
    }

    #[test]
    fn the_read_model_never_writes() {
        // The trait object is Send + Sync and exposes no mut methods: the
        // type system carries the read-only boundary the overlay depends
        // on. A compile-time anchor plus the pure-function assertion above
        // is the honest scope of this skeleton — transport and push
        // semantics belong to the cross-domain proposal.
        fn assert_read_only<T: OverlayReadModel + ?Sized>() {}
        assert_read_only::<dyn OverlayReadModel>();
    }

    #[test]
    fn production_card_empty_authorities_are_the_honest_empty_card() {
        let database = unique_database("production-empty");
        let store = std::sync::Arc::new(crate::SqliteTaskStore::open(&database).unwrap());
        let model = StoreOverlayReadModel::new(
            store,
            std::sync::Arc::new(crate::PlanDocumentStore::new(unique_root("empty-plans"))),
            std::sync::Arc::new(crate::RecipeRecordStore::new(unique_root("empty-records"))),
        );

        // 空态即终态: no plans, no records — both halves are None, never
        // synthesized rows; the card itself still renders.
        let card = model.production_card().unwrap();
        assert_eq!(card.current_plan, None);
        assert_eq!(card.latest_record, None);

        // Pure function: two unchanged queries observe the same card.
        assert_eq!(model.production_card().unwrap(), card);
    }

    #[test]
    fn production_card_picks_current_plan_and_latest_record_by_their_own_timestamps() {
        let database = unique_database("production-pick");
        let store = std::sync::Arc::new(crate::SqliteTaskStore::open(&database).unwrap());
        let plans_root = unique_root("pick-plans");
        let records_root = unique_root("pick-records");
        let plans = std::sync::Arc::new(crate::PlanDocumentStore::new(&plans_root));
        let records = std::sync::Arc::new(crate::RecipeRecordStore::new(&records_root));

        // Three plans with deliberately out-of-order storage order and
        // ids: the newest createdAt must win regardless of either. Plans
        // are published as drafts (the store's discipline) and flipped
        // through their real lifecycle acts.
        plans
            .publish_draft(
                "plan-b-older",
                &plan_document("plan-b-older", "2026-09-12T01:00:00.000Z"),
            )
            .unwrap();
        plans.supersede("plan-b-older").unwrap();
        plans
            .publish_draft(
                "plan-c-newest",
                &plan_document("plan-c-newest", "2026-09-12T03:00:00.000Z"),
            )
            .unwrap();
        plans.approve("plan-c-newest").unwrap();
        plans
            .publish_draft(
                "plan-a-oldest",
                &plan_document("plan-a-oldest", "2026-09-12T00:00:00.000Z"),
            )
            .unwrap();
        // Three records likewise; finishedAt — not buildId — decides.
        for (build_id, finished_at, status) in [
            ("build-z-mid", "2026-09-12T02:00:00.000Z", "succeeded"),
            ("build-a-newest", "2026-09-12T04:00:00.000Z", "failed"),
            ("build-m-old", "2026-09-12T00:30:00.000Z", "cancelled"),
        ] {
            records
                .publish(build_id, &record_document(build_id, finished_at, status))
                .unwrap();
        }

        let model = StoreOverlayReadModel::new(store, plans, records);
        let card = model.production_card().unwrap();

        let current = card.current_plan.as_ref().expect("a current plan exists");
        assert_eq!(current.plan_id, "plan-c-newest");
        assert_eq!(current.plan_status, "approved");
        assert_eq!(current.created_at, "2026-09-12T03:00:00.000Z");
        assert_eq!(current.recipe_id, "019e0000-0000-7000-8000-000000000001");

        let latest = card.latest_record.as_ref().expect("a latest record exists");
        assert_eq!(latest.build_id, "build-a-newest");
        assert_eq!(latest.status, "failed");
        assert_eq!(latest.finished_at, "2026-09-12T04:00:00.000Z");

        // Pure function: two unchanged queries observe the same card.
        assert_eq!(model.production_card().unwrap(), card);
    }

    #[test]
    fn production_card_halves_are_independent() {
        let database = unique_database("production-halves");
        let store = std::sync::Arc::new(crate::SqliteTaskStore::open(&database).unwrap());
        let plans = std::sync::Arc::new(crate::PlanDocumentStore::new(unique_root("halves-plans")));
        let records =
            std::sync::Arc::new(crate::RecipeRecordStore::new(unique_root("halves-records")));

        // Planned but never built: the plan half fills, the record half
        // stays honestly empty.
        plans
            .publish_draft(
                "plan-only",
                &plan_document("plan-only", "2026-09-12T05:00:00.000Z"),
            )
            .unwrap();
        let model = StoreOverlayReadModel::new(store, plans, records);
        let card = model.production_card().unwrap();
        assert_eq!(
            card.current_plan.as_ref().expect("plan half").plan_id,
            "plan-only"
        );
        assert_eq!(
            card.latest_record, None,
            "no records is an honest empty half"
        );
    }

    /// Accepts a task row directly on the store (the download face mints
    /// `dl-<downloadId>-a<attempt>` rows through the same primitive).
    fn seed_task(
        store: &crate::SqliteTaskStore,
        task_id: &str,
        correlation_id: &str,
        occurred_at: &str,
    ) {
        store
            .accept_task(&crate::NewTask {
                task_id: task_id.to_owned(),
                correlation_id: correlation_id.to_owned(),
                occurred_at: occurred_at.to_owned(),
            })
            .unwrap();
    }

    #[test]
    fn download_card_empty_authority_is_the_honest_empty_card() {
        let database = unique_database("download-empty");
        let store = std::sync::Arc::new(crate::SqliteTaskStore::open(&database).unwrap());
        let model = StoreOverlayReadModel::new(
            store,
            std::sync::Arc::new(crate::PlanDocumentStore::new(unique_root("dl-empty-plans"))),
            std::sync::Arc::new(crate::RecipeRecordStore::new(unique_root(
                "dl-empty-records",
            ))),
        );

        // 空态即终态: nothing in flight — the honest empty card, not an
        // error; two unchanged queries observe the same card (read-only
        // discipline).
        let card = model.download_card().unwrap();
        assert!(card.active_downloads.is_empty());
        assert_eq!(model.download_card().unwrap(), card);
    }

    #[test]
    fn download_card_holds_only_non_terminal_dl_attempts_with_their_own_fields() {
        let database = unique_database("download-select");
        let store = std::sync::Arc::new(crate::SqliteTaskStore::open(&database).unwrap());

        // Two in-flight download attempts, one terminal download attempt,
        // and two foreign tasks (demo + production prefixes): only the
        // in-flight `dl-` rows may enter the card.
        seed_task(
            &store,
            "dl-019e0000-0000-7000-8000-000000000601-a1",
            "019e0000-0000-7000-8000-000000000601",
            "2026-09-12T01:00:00.000Z",
        );
        seed_task(
            &store,
            "dl-019e0000-0000-7000-8000-000000000602-a1",
            "019e0000-0000-7000-8000-000000000602",
            "2026-09-12T02:00:00.000Z",
        );
        seed_task(
            &store,
            "dl-019e0000-0000-7000-8000-000000000603-a1",
            "019e0000-0000-7000-8000-000000000603",
            "2026-09-12T03:00:00.000Z",
        );
        seed_task(
            &store,
            "demo-019e0000-0000-7000-8000-000000000604",
            "corr-demo",
            "2026-09-12T04:00:00.000Z",
        );
        seed_task(
            &store,
            "prod-019e0000-0000-7000-8000-000000000605",
            "corr-prod",
            "2026-09-12T05:00:00.000Z",
        );
        // Walk one attempt to a terminal state (succeeded) through the
        // real nine-state path: it must leave the card — the stance is
        // "while items are in flight".
        for (state, at) in [
            (crate::TaskState::Preparing, "2026-09-12T03:05:00.000Z"),
            (crate::TaskState::Running, "2026-09-12T03:10:00.000Z"),
        ] {
            let current = store
                .task("dl-019e0000-0000-7000-8000-000000000603-a1")
                .unwrap()
                .unwrap();
            store
                .mutate_task(
                    &current.task_id,
                    current.revision,
                    at,
                    crate::TaskMutation::Transition {
                        state,
                        payload: json!({"receivedBytes": 1, "expectedBytes": 2}),
                    },
                )
                .unwrap();
        }
        let terminal = store
            .task("dl-019e0000-0000-7000-8000-000000000603-a1")
            .unwrap()
            .unwrap();
        store
            .mutate_task(
                &terminal.task_id,
                terminal.revision,
                "2026-09-12T03:30:00.000Z",
                crate::TaskMutation::Complete {
                    state: crate::TaskState::Succeeded,
                    error: None,
                    result: Some(json!({"downloadId": "019e0000-0000-7000-8000-000000000603"})),
                },
            )
            .unwrap();

        let model = StoreOverlayReadModel::new(
            store,
            std::sync::Arc::new(crate::PlanDocumentStore::new(unique_root(
                "dl-select-plans",
            ))),
            std::sync::Arc::new(crate::RecipeRecordStore::new(unique_root(
                "dl-select-records",
            ))),
        );
        let card = model.download_card().unwrap();

        assert_eq!(
            card.active_downloads.len(),
            2,
            "terminal attempt and foreign tasks stay out"
        );
        let first = &card.active_downloads[0];
        assert_eq!(first.download_id, "019e0000-0000-7000-8000-000000000601");
        assert_eq!(
            first.state, "queued",
            "the nine-state word passes through verbatim"
        );
        assert_eq!(first.updated_at, "2026-09-12T01:00:00.000Z");
        // Enqueue order (created_at ascending), never the id alphabet.
        assert_eq!(
            card.active_downloads[1].download_id,
            "019e0000-0000-7000-8000-000000000602"
        );
    }
}
