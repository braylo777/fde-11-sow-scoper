//! STRIDE threat model for the selected use cases.
//!
//! Threats come from the shape of the deployment, not from a fixed list: which systems the
//! agent touches, how (REST, DB, SOAP, no API), how sensitive the data is, and whether the
//! agent writes. Each threat has a STRIDE category, a 1-3 likelihood and impact, a risk
//! score, a mitigation, and a test that proves the mitigation works. Highest risk first.

use crate::{discovery, Discovery};
use serde_json::{json, Value};

struct T {
    stride: &'static str,
    component: String,
    threat: String,
    likelihood: u8,
    impact: u8,
    mitigation: &'static str,
    test: &'static str,
}

fn sev(s: &str) -> u8 {
    match s {
        "phi" | "pci" => 3,
        "pii" => 3,
        "internal" => 2,
        _ => 1,
    }
}

pub fn model(d: &Discovery, selected: &[String]) -> Vec<Value> {
    let ucs: Vec<_> = d
        .use_cases
        .iter()
        .filter(|u| selected.is_empty() || selected.contains(&u.name))
        .collect();
    let mut sys: Vec<&str> = ucs.iter().flat_map(|u| u.systems.iter().map(String::as_str)).collect();
    sys.sort();
    sys.dedup();
    let mut t: Vec<T> = vec![
        T { stride: "Spoofing", component: "Agent identity".into(),
            threat: "Agent calls tools with a long-lived shared service account, so actions cannot be tied to a human and a leaked key works for months".into(),
            likelihood: 2, impact: 3,
            mitigation: "Short-lived (5 min) delegated tokens per human and agent; SCIM deactivation revokes them",
            test: "Deactivate a test user; their agent's next tool call is refused" },
        T { stride: "Tampering", component: "Agent service".into(),
            threat: "Prompt injection in customer-supplied text (tickets, emails, PDFs) makes the agent call tools it should not".into(),
            likelihood: 3, impact: 2,
            mitigation: "Tool allow-list per use case; untrusted text never widens scopes; schema-validated tool args",
            test: "Red-team set of 25 injected tickets: zero out-of-scope tool calls" },
        T { stride: "Repudiation", component: "Audit trail".into(),
            threat: "No tamper-evident record of who (human + agent) did what, so incidents cannot be reconstructed".into(),
            likelihood: 2, impact: 2,
            mitigation: "Hash-chained audit log of every tool call with human sub and agent act claims",
            test: "Edit one audit row; the chain check fails" },
        T { stride: "Denial of service", component: "LLM endpoint".into(),
            threat: "Runaway agent loop or traffic spike burns budget and hits provider rate limits".into(),
            likelihood: 2, impact: 2,
            mitigation: "Per-team token budgets, step caps, circuit breaker and kill switch",
            test: "Replay a looping trajectory; it stops at the step cap and alerts" },
    ];
    let max_sens = sys
        .iter()
        .filter_map(|s| d.systems.iter().find(|x| x.name == *s))
        .map(|x| sev(&x.sensitivity))
        .max()
        .unwrap_or(1);
    if max_sens >= 3 {
        t.push(T { stride: "Information disclosure", component: "LLM endpoint".into(),
            threat: "Regulated data (PII / PHI / PCI) is sent to a third-party model provider in prompts".into(),
            likelihood: 3, impact: 3,
            mitigation: "Masking proxy swaps identifiers for tokens before inference and restores after; or private / on-prem model",
            test: "Golden set with seeded SSNs and cards: zero raw values in outbound prompts" });
        t.push(T { stride: "Information disclosure", component: "Retrieval".into(),
            threat: "Retrieval returns documents the asking user may not see (cross-team or executive data)".into(),
            likelihood: 2, impact: 3,
            mitigation: "Permission filter before retrieval (tenant + ACL), not after generation",
            test: "Leak test: low-privilege user queries for restricted docs; zero returned" });
    }
    for s in &sys {
        let Some(x) = d.systems.iter().find(|x| x.name == *s) else { continue };
        let name = x.name.clone();
        match x.access.as_str() {
            "db" => t.push(T { stride: "Elevation of privilege", component: name.clone(),
                threat: format!("Direct DB access to {name} with broad credentials lets one bad query read or change any table"),
                likelihood: 2, impact: sev(&x.sensitivity).max(2),
                mitigation: "Read replica, least-privilege DB role, parameterized queries only, row limits",
                test: "Run a forbidden query with the agent's role; the DB refuses it" }),
            "none" => t.push(T { stride: "Tampering", component: name.clone(),
                threat: format!("{name} has no API; screen scraping or RPA breaks silently and may enter wrong data"),
                likelihood: 3, impact: 2,
                mitigation: "Read-only first; verify each scrape against a checksum or second source; defer writes",
                test: "Change one screen label in staging; the bot stops and alerts instead of guessing" }),
            "file_export" => t.push(T { stride: "Information disclosure", component: name.clone(),
                threat: format!("Exported files from {name} sit unencrypted on shared drives and go stale"),
                likelihood: 2, impact: sev(&x.sensitivity),
                mitigation: "Encrypted drop zone with expiry; freshness check on every load",
                test: "Load a file older than the freshness limit; the pipeline refuses it" }),
            _ => {}
        }
        if x.owner.trim().is_empty() {
            t.push(T { stride: "Repudiation", component: name.clone(),
                threat: format!("{name} has no named owner, so nobody approves access or answers for incidents"),
                likelihood: 2, impact: 2,
                mitigation: "Name an owner before week 1 ends; owner signs the access request",
                test: "Access request for the system shows a named approver" });
        }
    }
    for u in ucs.iter().filter(|u| u.writes) {
        t.push(T { stride: "Elevation of privilege", component: u.name.clone(),
            threat: format!("Agent writes to systems of record for '{}' with no human check; one bad step is a real transaction", u.name),
            likelihood: 2, impact: 3,
            mitigation: "Preview-then-commit writes, approval gate above a value limit, idempotency keys, optimistic locking",
            test: "Over-limit write in shadow pauses for approval; a retried commit writes once" });
    }
    let mut rows: Vec<Value> = t
        .into_iter()
        .map(|t| {
            let risk = t.likelihood * t.impact;
            json!({"stride": t.stride, "component": t.component, "threat": t.threat,
                   "likelihood": t.likelihood, "impact": t.impact, "risk": risk,
                   "level": if risk >= 6 { "high" } else if risk >= 4 { "medium" } else { "low" },
                   "mitigation": t.mitigation, "verification": t.test})
        })
        .collect();
    rows.sort_by(|a, b| b["risk"].as_u64().cmp(&a["risk"].as_u64()));
    for (n, r) in rows.iter_mut().enumerate() {
        r["id"] = json!(format!("T-{:02}", n + 1));
    }
    rows
}

pub fn markdown(rows: &[Value]) -> String {
    let mut md = String::from("| ID | STRIDE | Component | Threat | Risk | Mitigation | Verification |\n|---|---|---|---|---|---|---|\n");
    for r in rows {
        md += &format!(
            "| {} | {} | {} | {} | {} ({}) | {} | {} |\n",
            r["id"].as_str().unwrap_or(""),
            r["stride"].as_str().unwrap_or(""),
            r["component"].as_str().unwrap_or("").replace('|', "/"),
            r["threat"].as_str().unwrap_or("").replace('|', "/"),
            r["level"].as_str().unwrap_or(""),
            r["risk"],
            r["mitigation"].as_str().unwrap_or(""),
            r["verification"].as_str().unwrap_or("")
        );
    }
    md
}

pub fn threat_op(i: &Value) -> Result<Value, String> {
    let d = discovery(i)?;
    let selected: Vec<String> = i["selected"]
        .as_array()
        .map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let rows = model(&d, &selected);
    let high = rows.iter().filter(|r| r["level"] == "high").count();
    Ok(json!({"threats": rows, "high": high, "markdown": markdown(&rows)}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(sens: &str, access: &str, writes: bool) -> Discovery {
        serde_json::from_value(json!({"customer": "X",
            "systems": [{"name": "S", "access": access, "sensitivity": sens, "owner": "IT"}],
            "use_cases": [{"name": "U", "volume_per_month": 10, "minutes_per_task": 1, "systems": ["S"], "writes": writes}]}))
        .unwrap()
    }

    #[test]
    fn baseline_threats_always_present_and_sorted() {
        let r = model(&d("internal", "rest", false), &[]);
        assert_eq!(r.len(), 4);
        assert!(r.windows(2).all(|w| w[0]["risk"].as_u64() >= w[1]["risk"].as_u64()));
        assert_eq!(r[0]["id"], "T-01");
    }

    #[test]
    fn pii_db_and_writes_add_threats() {
        let r = model(&d("pii", "db", true), &[]);
        let has = |s: &str| r.iter().any(|t| t["threat"].as_str().unwrap().contains(s));
        assert!(has("third-party model provider"));
        assert!(has("Direct DB access"));
        assert!(has("no human check"));
        assert_eq!(r[0]["level"], "high");
    }

    #[test]
    fn unowned_and_no_api_flagged() {
        let mut x = d("internal", "none", false);
        x.systems[0].owner = String::new();
        let r = model(&x, &[]);
        assert!(r.iter().any(|t| t["threat"].as_str().unwrap().contains("no API")));
        assert!(r.iter().any(|t| t["threat"].as_str().unwrap().contains("no named owner")));
    }
}
