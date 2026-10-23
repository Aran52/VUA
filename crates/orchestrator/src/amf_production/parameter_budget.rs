//! Source-side synced parameter budget evidence and the assembly-plan estimate against
//! VRChat's 256-bit limit. Modular Avatar merges parameters only at build time, so every
//! number here is a source estimate; the built cost stays with the SDK's own check.
use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParameterEntry {
    pub name: String,
    pub bits: u8,
    /// MA renames internal parameters to unique names at build time, so they never merge
    /// with a same-named declaration.
    pub unique: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParameterComponent {
    /// Path of the MA Parameters component inside the candidate; empty names the prefab root.
    pub path: String,
    pub parameters: Vec<ParameterEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ParameterBudget {
    pub expression_parameters_observed: bool,
    pub expression_parameters: Vec<ParameterEntry>,
    pub ma_components: Vec<ParameterComponent>,
    /// MA prefix configs bind parameters that exist only at build time; they add no bits here.
    pub unresolved_prefix_configs: u32,
    pub truncated: bool,
}

fn plain(text: &str, max: usize) -> bool {
    text.chars().count() <= max && !text.chars().any(char::is_control)
}
fn entry_valid(entry: &ParameterEntry) -> bool {
    !entry.name.is_empty() && plain(&entry.name, 256) && (entry.bits == 1 || entry.bits == 8)
}
/// Validate new candidate evidence when present; an older Bridge omits it completely.
pub fn parameter_budget_valid(budget: &ParameterBudget) -> bool {
    budget.expression_parameters.len() <= 256
        && budget.expression_parameters.iter().all(entry_valid)
        && (budget.expression_parameters_observed || budget.expression_parameters.is_empty())
        && budget.ma_components.len() <= 64
        && budget.ma_components.iter().all(|component| {
            plain(&component.path, 1024)
                && component.parameters.len() <= 256
                && component.parameters.iter().all(entry_valid)
        })
        && budget.unresolved_prefix_configs <= 4096
}
/// Every candidate's parameter evidence must stay inside its contract before it is stored.
pub fn validate_prefab_parameter_budgets(prefabs: &[PrefabCandidate]) -> Result<(), AppErrorV1> {
    if prefabs.iter().any(|prefab| prefab.parameter_budget.as_ref().is_some_and(|budget| !parameter_budget_valid(budget))) {
        return Err(production_error(
            "receipt_invalid",
            "Parameter budget evidence exceeds its contract.",
        ));
    }
    Ok(())
}

/// VRChat's synced expression-parameter budget the SDK enforces at upload.
pub const PARAMETER_LIMIT_BITS: u32 = 256;

struct BudgetCount {
    bits: u32,
    seen: std::collections::BTreeSet<String>,
}
impl BudgetCount {
    fn new() -> Self { Self { bits: 0, seen: std::collections::BTreeSet::new() } }
    // MA merges same-named parameters, with the earlier declaration winning: the Avatar's
    // own asset first, then its embedded components, then each selected outfit in order.
    fn add(&mut self, entry: &ParameterEntry) {
        if entry.unique || self.seen.insert(entry.name.clone()) {
            self.bits += entry.bits as u32;
        }
    }
    fn add_budget(&mut self, budget: &ParameterBudget) {
        for entry in &budget.expression_parameters { self.add(entry); }
        for component in &budget.ma_components {
            for entry in &component.parameters { self.add(entry); }
        }
    }
}

/// The plan's estimate of the synced parameter bits the assembled Avatar would use:
/// the Avatar's own Expression Parameters plus every selected candidate's MA Parameters,
/// merged by name the way MA merges at build time, plus one bit for each switch VUA adds.
/// Each source row holds the bits it adds to the running total, so the rows sum to it. Old runs without the evidence, an
/// unreadable Avatar asset or a truncated observation report `unknown` instead of a number.
pub fn plan_parameter_budget(run: &ProductionRun, selection: &AssemblySelection) -> serde_json::Value {
    let avatar = run.prefabs.iter().find(|prefab| prefab.is_avatar && prefab.path == selection.avatar_prefab);
    let avatar_budget = avatar.and_then(|prefab| prefab.parameter_budget.as_ref());
    let outfits: Vec<(&OutfitSelection, &PrefabCandidate)> = selection
        .outfits
        .iter()
        .filter_map(|outfit| {
            run.prefabs
                .iter()
                .find(|prefab| {
                    !prefab.is_avatar && prefab.source_id == outfit.source_id && prefab.path == outfit.prefab_path
                })
                .map(|prefab| (outfit, prefab))
        })
        .collect();
    let unusable = |budget: &Option<&ParameterBudget>| match budget {
        None | Some(&ParameterBudget { truncated: true, .. }) => true,
        Some(_) => false,
    };
    if avatar_budget.map(|budget| !budget.expression_parameters_observed).unwrap_or(true)
        || unusable(&avatar_budget)
        || outfits.iter().any(|(_, outfit)| unusable(&outfit.parameter_budget.as_ref()))
    {
        return serde_json::json!({"schemaVersion":"0.1","limitBits":PARAMETER_LIMIT_BITS,"state":"unknown",
            "estimateBits":0,"evidence":"unity_source_estimate","sources":[],"unresolvedPrefixConfigs":0});
    }
    let avatar_budget = avatar_budget.expect("checked above");
    let avatar_name = avatar.expect("a budget implies its candidate").name.clone();
    let mut count = BudgetCount::new();
    for entry in &avatar_budget.expression_parameters { count.add(entry); }
    let expression_bits = count.bits;
    for component in &avatar_budget.ma_components {
        for entry in &component.parameters { count.add(entry); }
    }
    let avatar_ma_bits = count.bits - expression_bits;
    let mut sources = vec![serde_json::json!({"kind":"expression_parameters","origin":"avatar_base",
        "name":avatar_name,"bits":expression_bits})];
    if avatar_ma_bits > 0 || avatar_budget.unresolved_prefix_configs > 0 {
        sources.push(serde_json::json!({"kind":"ma_parameters","origin":"avatar_base",
            "name":avatar_name,"bits":avatar_ma_bits}));
    }
    let mut unresolved = avatar_budget.unresolved_prefix_configs;
    // An exclusive group's switches share one parameter: a Bool for a group of one, an Int (8
    // bits) otherwise, counted on the group's first outfit.
    let mut group_sizes = std::collections::BTreeMap::<&str, u32>::new();
    for (choice, _) in &outfits {
        if !choice.group.is_empty() && !choice.toggle_name.trim().is_empty() {
            *group_sizes.entry(choice.group.as_str()).or_default() += 1;
        }
    }
    let mut groups_counted = std::collections::BTreeSet::<&str>::new();
    for (choice, outfit) in &outfits {
        // Checked above to be present and untruncated.
        let budget = outfit.parameter_budget.as_ref().expect("checked above");
        let before = count.bits;
        count.add_budget(budget);
        // VUA's own switch for a labelled outfit is an MA Menu Item whose parameter MA creates
        // at build time under a unique name: one synced Bool.
        if !choice.toggle_name.trim().is_empty() {
            if choice.group.is_empty() {
                count.bits += 1;
            } else if groups_counted.insert(choice.group.as_str()) {
                count.bits += if group_sizes[choice.group.as_str()] > 1 { 8 } else { 1 };
            }
        }
        let bits = count.bits - before;
        unresolved += budget.unresolved_prefix_configs;
        if bits > 0 || budget.unresolved_prefix_configs > 0 {
            sources.push(serde_json::json!({"kind":"ma_parameters","origin":"selected_outfit",
                "name":outfit.name,"bits":bits}));
        }
    }
    let estimate = count.bits;
    let state = if estimate > PARAMETER_LIMIT_BITS {
        "over_limit"
    } else if estimate * 10 > PARAMETER_LIMIT_BITS * 9 {
        "near_limit"
    } else {
        "fits"
    };
    serde_json::json!({"schemaVersion":"0.1","limitBits":PARAMETER_LIMIT_BITS,"state":state,
        "estimateBits":estimate,"evidence":"unity_source_estimate","sources":sources,
        "unresolvedPrefixConfigs":unresolved})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, bits: u8) -> ParameterEntry { ParameterEntry { name: name.into(), bits, unique: false } }
    fn component(path: &str, entries: Vec<ParameterEntry>) -> ParameterComponent {
        ParameterComponent { path: path.into(), parameters: entries }
    }
    fn budget(observed: bool, expression: Vec<ParameterEntry>, components: Vec<ParameterComponent>) -> ParameterBudget {
        ParameterBudget { expression_parameters_observed: observed, expression_parameters: expression,
            ma_components: components, unresolved_prefix_configs: 0, truncated: false }
    }
    fn candidate(name: &str, path: &str, avatar: bool, evidence: Option<ParameterBudget>) -> PrefabCandidate {
        PrefabCandidate { source_id: if avatar { "avatar" } else { "outfit" }.into(), path: path.into(),
            name: name.into(), is_avatar: avatar, has_skinned_mesh: true, has_merge_armature: false,
            has_missing_scripts: false, humanoid_bones: vec![], preview: None, parameter_budget: evidence }
    }
    fn run(prefabs: Vec<PrefabCandidate>) -> ProductionRun {
        ProductionRun { run_id: "n3-test".into(), prefabs, ..serde_json::from_value(serde_json::json!({
            "schemaVersion":"0.1","runId":"n3-test","phase":"succeeded","taskId":"t",
            "editor":{"path":"e","version":"2022.3.22f1c1"},
            "projectRoot":"projects/n3-test","choices":[],"sources":[],"prefabs":[],"steps":[],"selection":null,
            "projectFingerprint":"p","unityFingerprint":"u","confirmedDigest":null,
            "error":null,"recovery":null,"result":null})).expect("synthetic run") }
    }
    fn selection_with(outfits: &[(&str, &str)]) -> AssemblySelection {
        AssemblySelection { avatar_prefab: "Assets/Avatar.prefab".into(),
            outfits: outfits.iter().map(|(source, path)| OutfitSelection {
                source_id: (*source).into(), prefab_path: (*path).into(),
                mode: BindingMode::MergeArmature, bone: String::new(), toggle_name: String::new(), active: true,
                saved: true, group: String::new() }).collect() }
    }

    #[test]
    fn estimate_counts_avatar_and_outfits_against_the_limit() {
        let evidence = budget(true, vec![entry("VRCEmote", 8), entry("VRMode", 1)],
            vec![component("Default Clothing", vec![entry("DefaultDress", 1)])]);
        let outfit = budget(true, vec![], vec![component("", vec![entry("WinterCoat", 1)])]);
        let run = run(vec![candidate("Avatar", "Assets/Avatar.prefab", true, Some(evidence)),
            candidate("Coat", "Assets/Outfit.prefab", false, Some(outfit))]);
        let plan = plan_parameter_budget(&run, &selection_with(&[("outfit", "Assets/Outfit.prefab")]));
        assert_eq!(plan["state"], "fits");
        assert_eq!(plan["estimateBits"], 11);
        assert_eq!(plan["sources"].as_array().unwrap().len(), 3);
        assert_eq!(plan["sources"][0]["bits"], 9);
        assert_eq!(plan["sources"][1]["bits"], 1);
        assert_eq!(plan["sources"][2]["origin"], "selected_outfit");
    }

    #[test]
    fn same_named_parameters_merge_like_the_build_would() {
        let evidence = budget(true, vec![entry("OutfitMode", 8)], vec![]);
        let outfit = budget(true, vec![], vec![component("", vec![entry("OutfitMode", 8), entry("Extra", 1),
            ParameterEntry { name: "Inner".into(), bits: 1, unique: true },
            ParameterEntry { name: "Inner".into(), bits: 1, unique: true }])]);
        let run = run(vec![candidate("Avatar", "Assets/Avatar.prefab", true, Some(evidence)),
            candidate("Coat", "Assets/Outfit.prefab", false, Some(outfit))]);
        let plan = plan_parameter_budget(&run, &selection_with(&[("outfit", "Assets/Outfit.prefab")]));
        // The outfit's OutfitMode merges into the Avatar's own; internal parameters keep theirs.
        assert_eq!(plan["estimateBits"], 8 + 1 + 1 + 1);
        // The merged name stays with the Avatar's row, so the rows still sum to the estimate.
        assert_eq!(plan["sources"][1]["bits"], 3);
        assert_eq!(rows_total(&plan), 11);
    }

    fn rows_total(plan: &serde_json::Value) -> u64 {
        plan["sources"].as_array().unwrap().iter().map(|row| row["bits"].as_u64().unwrap()).sum()
    }

    #[test]
    fn an_exclusive_group_shares_one_int_on_its_first_outfit() {
        let evidence = budget(true, vec![entry("VRCEmote", 8)], vec![]);
        let run = run(vec![candidate("Avatar", "Assets/Avatar.prefab", true, Some(evidence)),
            candidate("Coat", "Assets/Coat.prefab", false, Some(budget(true, vec![], vec![]))),
            candidate("Dress", "Assets/Dress.prefab", false, Some(budget(true, vec![], vec![]))),
            candidate("Hat", "Assets/Hat.prefab", false, Some(budget(true, vec![], vec![])))]);
        let mut selection = selection_with(&[("outfit", "Assets/Coat.prefab"), ("outfit", "Assets/Dress.prefab"), ("outfit", "Assets/Hat.prefab")]);
        for (outfit, name) in selection.outfits.iter_mut().zip(["Coat", "Dress", "Hat"]) { outfit.toggle_name = name.into(); }
        selection.outfits[0].group = "main".into();
        selection.outfits[1].group = "main".into();
        let plan = plan_parameter_budget(&run, &selection);
        // VRCEmote 8, the main group's Int 8 on the coat, the hat's own Bool.
        assert_eq!(plan["estimateBits"], 8 + 8 + 1);
        assert_eq!(plan["sources"][1]["name"], "Coat");
        assert_eq!(plan["sources"][1]["bits"], 8);
        assert_eq!(plan["sources"][2]["name"], "Hat");
        assert_eq!(rows_total(&plan), 17);
        // A group of one stays a Bool.
        selection.outfits[1].group.clear();
        assert_eq!(plan_parameter_budget(&run, &selection)["estimateBits"], 8 + 1 + 1 + 1);
    }

    #[test]
    fn each_labelled_outfit_adds_the_bit_of_the_switch_vua_creates() {
        let evidence = budget(true, vec![entry("VRCEmote", 8)], vec![component("", vec![entry("Shared", 1)])]);
        let coat = budget(true, vec![], vec![component("", vec![entry("Shared", 1), entry("Hood", 1)])]);
        let run = run(vec![candidate("Avatar", "Assets/Avatar.prefab", true, Some(evidence)),
            candidate("Coat", "Assets/Outfit.prefab", false, Some(coat)),
            candidate("Hat", "Assets/Hat.prefab", false, Some(budget(true, vec![], vec![])))]);
        let mut selection = selection_with(&[("outfit", "Assets/Outfit.prefab"), ("outfit", "Assets/Hat.prefab")]);
        selection.outfits[0].toggle_name = "Coat".into();
        selection.outfits[1].toggle_name = "Hat".into();
        let plan = plan_parameter_budget(&run, &selection);
        // Avatar 8 + 1, the coat's Hood plus its switch, the hat's switch alone.
        assert_eq!(plan["estimateBits"], 8 + 1 + 2 + 1);
        assert_eq!(plan["sources"][2]["bits"], 2);
        assert_eq!(plan["sources"][3]["name"], "Hat");
        assert_eq!(plan["sources"][3]["bits"], 1);
        assert_eq!(rows_total(&plan), 12);
        // An imported-only outfit has no switch and adds no bit of its own.
        selection.outfits[1].toggle_name = "  ".into();
        assert_eq!(plan_parameter_budget(&run, &selection)["estimateBits"], 11);
    }

    #[test]
    fn the_verdict_marks_near_and_over_the_limit() {
        // 28 distinct synced Int parameters: 224 bits, under the limit on their own.
        let full = (0..28).map(|index| entry(&format!("Param{index}"), 8)).collect();
        let evidence = budget(true, full, vec![]);
        let small = budget(true, vec![], vec![component("", vec![entry("A", 8)])]);
        let big = budget(true, vec![], vec![component("", (0..5).map(|index| entry(&format!("B{index}"), 8)).collect())]);
        let run = run(vec![candidate("Avatar", "Assets/Avatar.prefab", true, Some(evidence)),
            candidate("Small", "Assets/Small.prefab", false, Some(small.clone())),
            candidate("Big", "Assets/Big.prefab", false, Some(big))]);
        assert_eq!(plan_parameter_budget(&run, &selection_with(&[("outfit", "Assets/Small.prefab")]))["state"], "near_limit");
        assert_eq!(plan_parameter_budget(&run, &selection_with(&[("outfit", "Assets/Big.prefab")]))["state"], "over_limit");
    }

    #[test]
    fn missing_or_untrustworthy_evidence_reports_unknown_instead_of_a_number() {
        let unreadable = budget(false, vec![], vec![]);
        let truncated = ParameterBudget { truncated: true, ..budget(true, vec![entry("A", 1)], vec![]) };
        let avatar = candidate("Avatar", "Assets/Avatar.prefab", true, Some(unreadable));
        assert_eq!(plan_parameter_budget(&run(vec![avatar]), &selection_with(&[]))["state"], "unknown");
        let avatar = candidate("Avatar", "Assets/Avatar.prefab", true, None);
        assert_eq!(plan_parameter_budget(&run(vec![avatar]), &selection_with(&[]))["state"], "unknown");
        let avatar = candidate("Avatar", "Assets/Avatar.prefab", true, Some(truncated));
        let outfit = candidate("Coat", "Assets/Outfit.prefab", false,
            Some(ParameterBudget { truncated: true, ..budget(true, vec![], vec![]) }));
        let plan_run = run(vec![avatar, outfit]);
        assert_eq!(plan_parameter_budget(&plan_run, &selection_with(&[("outfit", "Assets/Outfit.prefab")]))["state"], "unknown");
        // A selection naming an unobserved outfit cannot claim a total either.
        let avatar = candidate("Avatar", "Assets/Avatar.prefab", true, Some(budget(true, vec![], vec![])));
        let outfit = candidate("Coat", "Assets/Outfit.prefab", false, None);
        let plan_run = run(vec![avatar, outfit]);
        let plan = plan_parameter_budget(&plan_run, &selection_with(&[("outfit", "Assets/Outfit.prefab")]));
        assert_eq!(plan["state"], "unknown");
        assert_eq!(plan["estimateBits"], 0);
        assert_eq!(plan["sources"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn prefix_configs_report_as_unresolved_without_bits() {
        let evidence = ParameterBudget { unresolved_prefix_configs: 2, ..budget(true, vec![entry("A", 1)], vec![]) };
        let outfit = ParameterBudget { unresolved_prefix_configs: 1,
            ..budget(true, vec![], vec![component("", vec![entry("B", 1)])]) };
        let run = run(vec![candidate("Avatar", "Assets/Avatar.prefab", true, Some(evidence)),
            candidate("Coat", "Assets/Outfit.prefab", false, Some(outfit))]);
        let plan = plan_parameter_budget(&run, &selection_with(&[("outfit", "Assets/Outfit.prefab")]));
        assert_eq!(plan["estimateBits"], 2);
        assert_eq!(plan["unresolvedPrefixConfigs"], 3);
        assert_eq!(plan["sources"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn evidence_stays_inside_its_contract() {
        assert!(parameter_budget_valid(&budget(true, vec![entry("A", 1), entry("B", 8)], vec![])));
        assert!(parameter_budget_valid(&budget(true, vec![], vec![component("", vec![])])));
        assert!(!parameter_budget_valid(&budget(true, vec![entry("A", 7)], vec![])), "bits are 1 or 8");
        assert!(!parameter_budget_valid(&budget(false, vec![entry("A", 1)], vec![])), "unobserved carries no entries");
        assert!(!parameter_budget_valid(&budget(true, vec![], vec![component("a\u{0}b", vec![])])), "no control characters");
        assert!(!parameter_budget_valid(&ParameterBudget {
            expression_parameters: vec![entry("A", 1); 257], ..budget(true, vec![], vec![]) }));
        assert!(!parameter_budget_valid(&ParameterBudget {
            unresolved_prefix_configs: 4097, ..budget(true, vec![], vec![]) }));
        let mut prefabs = vec![candidate("A", "Assets/A.prefab", false,
            Some(ParameterBudget { expression_parameters: vec![entry("", 1)], ..budget(true, vec![], vec![]) }))];
        assert_eq!(validate_prefab_parameter_budgets(&prefabs).unwrap_err().code, "vua.amf_production.receipt_invalid");
        prefabs[0].parameter_budget = None;
        assert!(validate_prefab_parameter_budgets(&prefabs).is_ok());
    }
}
