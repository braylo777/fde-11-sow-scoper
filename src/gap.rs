//! Gap analysis: value vs. effort vs. risk for each candidate use case.

use crate::{discovery, Discovery, UseCase};
use serde_json::{json, Value};

/// Engineer-days to integrate one system by access type (read). Writes add 50%.
pub fn integration_days(access: &str) -> Option<f64> {
    match access {
        "rest" | "graphql" => Some(1.5),
        "db" => Some(2.5),
        "soap" => Some(3.0),
        "file_export" => Some(2.0),
        "none" => None, // no API: needs RPA / UI automation or a manual export, a blocker for week 1
        _ => Some(3.0),
    }
}

pub fn assess(d: &Discovery, u: &UseCase) -> Value {
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    let mut days = 4.0; // baseline: eval set, prompt / agent build, deployment
    let mut sensitivity = "internal".to_string();
    for sname in &u.systems {
        match d.systems.iter().find(|s| &s.name == sname) {
            None => blockers.push(format!(
                "system `{sname}` not in the discovery inventory: confirm owner and access"
            )),
            Some(s) => {
                match integration_days(&s.access) {
                    Some(x) => days += if u.writes { x * 1.5 } else { x },
                    None => blockers.push(format!(
                        "{} has no API or export: needs RPA or a manual export",
                        s.name
                    )),
                }
                if ["pii", "phi", "pci"].contains(&s.sensitivity.as_str()) {
                    sensitivity = s.sensitivity.clone();
                }
                if s.owner.is_empty() {
                    warnings.push(format!("{} has no named owner for access requests", s.name));
                }
            }
        }
    }
    if sensitivity != "internal" {
        days += 2.0; // PII masking / guardrails (stage 6)
        warnings.push(format!(
            "{} data: needs masking proxy and audit log before go-live",
            sensitivity.to_uppercase()
        ));
    }
    if u.writes {
        days += 1.5; // approval gate / HITL (stage 7)
        warnings
            .push("writes to a system of record: start in shadow mode, then human approval".into());
    }
    if !u.has_examples {
        days += 1.5;
        warnings
            .push("no labelled examples: budget time to build a golden set with the SMEs".into());
    }
    if u.success_metric.is_empty() {
        blockers.push("no success metric agreed with the sponsor".into());
    }
    let hours_per_month = u.volume_per_month * u.minutes_per_task / 60.0 * u.automatable_share;
    let value_month = hours_per_month * d.loaded_hourly_rate;
    let score = value_month / days;
    let readiness = if !blockers.is_empty() {
        "red"
    } else if warnings.len() >= 3 {
        "yellow"
    } else {
        "green"
    };
    json!({
        "use_case": u.name, "sponsor": u.sponsor, "readiness": readiness,
        "hours_saved_per_month": (hours_per_month * 10.0).round() / 10.0,
        "value_per_month_usd": value_month.round(), "effort_engineer_days": days,
        "value_per_effort_day": score.round(), "data_sensitivity": sensitivity,
        "blockers": blockers, "warnings": warnings
    })
}

pub fn ranked(d: &Discovery) -> Vec<Value> {
    let mut rows: Vec<Value> = d.use_cases.iter().map(|u| assess(d, u)).collect();
    rows.sort_by(|a, b| {
        let ra = (a["readiness"] == "red") as u8;
        let rb = (b["readiness"] == "red") as u8;
        ra.cmp(&rb).then(
            b["value_per_effort_day"]
                .as_f64()
                .partial_cmp(&a["value_per_effort_day"].as_f64())
                .unwrap(),
        )
    });
    rows
}

pub fn analyze_op(i: &Value) -> Result<Value, String> {
    let d = discovery(i)?;
    Ok(
        json!({"customer": d.customer, "ranked_use_cases": ranked(&d),
        "method": "value = volume x minutes x automatable share x loaded rate; effort = base 4 days + per-system integration + guardrails + HITL + golden set"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    fn d() -> Discovery {
        serde_json::from_value(json!({"customer": "x", "systems": [
            {"name": "Zendesk", "access": "rest", "sensitivity": "pii", "owner": "it"},
            {"name": "AS400", "access": "none", "owner": "ops"}],
            "use_cases": [
                {"name": "triage", "volume_per_month": 4000, "minutes_per_task": 3, "systems": ["Zendesk"], "has_examples": true, "success_metric": "routing accuracy"},
                {"name": "billing fix", "volume_per_month": 9000, "minutes_per_task": 10, "systems": ["AS400"], "writes": true, "success_metric": "x"}]})).unwrap()
    }
    #[test]
    fn red_items_rank_last_and_pii_adds_effort() {
        let r = ranked(&d());
        assert_eq!(r[0]["use_case"], "triage");
        assert_eq!(r[0]["effort_engineer_days"], 4.0 + 1.5 + 2.0);
        assert_eq!(r[1]["readiness"], "red");
    }
    #[test]
    fn no_api_is_blocker() {
        assert_eq!(integration_days("none"), None);
    }
}
