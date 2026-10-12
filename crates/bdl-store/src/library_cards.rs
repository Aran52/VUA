use super::*;
use std::collections::HashSet;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LibraryCardTarget { pub kind: String, pub id: String }
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct RemoveLibraryCards { pub schema_version: String, pub targets: Vec<LibraryCardTarget> }

impl BdlStore {
    pub fn removed_library_products(&self) -> Result<HashSet<String>, BdlStoreError> {
        let connection = self.connection.lock().expect("SQLite connection poisoned");
        let mut query = connection.prepare("SELECT target_id FROM removed_library_cards WHERE kind='product'")?;
        let rows = query.query_map([], |row| row.get(0))?.collect::<Result<_, _>>()?;
        Ok(rows)
    }

    pub fn remove_library_cards(&self, request: &RemoveLibraryCards, command_id: &str, now: &str) -> Result<RemoveLibraryCards, BdlStoreError> {
        let valid_id = |id: &str| !id.is_empty() && id.len() <= 128 && id.bytes().all(|c| c.is_ascii_alphanumeric() || b"_.:-".contains(&c));
        if request.schema_version != "0.2" || !valid_id(command_id) || request.targets.is_empty() || request.targets.len() > 200
            || request.targets.iter().collect::<HashSet<_>>().len() != request.targets.len()
            || request.targets.iter().any(|target| !valid_id(&target.id) || !["entry", "product"].contains(&target.kind.as_str())
                || target.kind == "product" && (!target.id.starts_with("booth:") || target.id[6..].is_empty() || !target.id[6..].bytes().all(|c| c.is_ascii_digit()))) {
            return Err(BdlStoreError::InvalidEvent("invalid_params"));
        }
        let fingerprint = serde_json::to_string(request)?;
        let mut connection = self.connection.lock().expect("SQLite connection poisoned");
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let prior: Option<(String, String)> = tx.query_row("SELECT fingerprint,result_json FROM removed_library_card_commands WHERE command_id=?1", [command_id], |row| Ok((row.get(0)?,row.get(1)?))).optional()?;
        if let Some((expected, result)) = prior {
            if expected != fingerprint { return Err(BdlStoreError::InvalidEvent("record_conflict")); }
            return Ok(serde_json::from_str(&result)?);
        }
        for target in &request.targets {
            let exists: bool = if target.kind == "entry" {
                tx.query_row("SELECT EXISTS(SELECT 1 FROM warehouse_items WHERE warehouse_item_id=?1)", [&target.id], |row| row.get(0))?
            } else {
                tx.query_row("SELECT EXISTS(SELECT 1 FROM products WHERE product_id=?1)", [&target.id], |row| row.get(0))?
            };
            if !exists { return Err(BdlStoreError::InvalidEvent("card_not_found")); }
        }
        for target in &request.targets {
            tx.execute("INSERT INTO removed_library_cards(kind,target_id,removed_at) VALUES (?1,?2,?3) ON CONFLICT(kind,target_id) DO NOTHING", params![target.kind,target.id,now])?;
        }
        tx.execute("INSERT INTO removed_library_card_commands(command_id,fingerprint,result_json) VALUES (?1,?2,?3)", params![command_id,fingerprint,serde_json::to_string(request)?])?;
        tx.commit()?;
        Ok(request.clone())
    }
}
