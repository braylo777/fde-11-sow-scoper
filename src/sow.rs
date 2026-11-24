//! Draft a 4-week SOW from the ranked gap analysis.
//! Selection: green / yellow use cases in rank order while their effort fits the budget.

use crate::{arch, discovery, gap};
use serde_json::{json, Value};

fn thousands(v: f64) -> String {
    let s = format!("{:.0}", v);
    let mut out = String::new();
    for (k, ch) in s.chars().enumerate() {
        if k > 0 && (s.len() - k) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

pub fn draft_op(i: &Value) -> Result<Value, String> {
    let d = discovery(i)?;
    let budget = i["engineer_days"].as_f64().unwrap_or(20.0);
    let start = i["start_date"].as_str().unwrap_or("TBD");
    let ranked = gap::ranked(&d);
    let mut used = 0.0;
    let mut chosen = Vec::new();
    let mut deferred = Vec::new();
    for r in &ranked {
        let e = r["effort_engineer_days"].as_f64().unwrap_or(0.0);
        if r["readiness"] != "red" && used + e <= budget {
            used += e;
            chosen.push(r.clone());
        } else {
            let why = if r["readiness"] == "red" {
                format!("blocked: {}", r["blockers"][0].as_str().unwrap_or(""))
            } else {
                "does not fit the 4-week budget".into()
            };
            deferred.push(json!({"use_case": r["use_case"], "reason": why}));
        }
    }
    if chosen.is_empty() {
        return Ok(
            json!({"sow_markdown": Value::Null, "deferred": deferred, "note": "nothing is ready: resolve blockers first (run gap_analysis)"}),
        );
    }
    let names: Vec<String> = chosen
        .iter()
        .map(|c| c["use_case"].as_str().unwrap().to_string())
        .collect();
    let metric_of = |n: &str| {
        d.use_cases
            .iter()
            .find(|u| u.name == n)
            .map(|u| u.success_metric.clone())
            .unwrap_or_default()
    };
    let value: f64 = chosen
        .iter()
        .map(|c| c["value_per_month_usd"].as_f64().unwrap_or(0.0))
        .sum();
    let mut md = String::new();
    md += &format!("# Statement of Work: {} AI pilot\n\n", d.customer);
    md += &format!("**Start:** {start} · **Duration:** 4 weeks · **Effort:** {used} of {budget} engineer-days\n\n");
    md += "## 1. Objective\n\n";
    md += &format!("Put {} use case(s) into supervised production and prove value with agreed metrics. Estimated value at full adoption: **${}/month**.\n\n", names.len(), thousands(value));
    md += "## 2. In scope\n\n| Use case | Sponsor | Success metric | Effort (days) | Readiness |\n|---|---|---|---|---|\n";
    for c in &chosen {
        let n = c["use_case"].as_str().unwrap();
        md += &format!(
            "| {n} | {} | {} | {} | {} |\n",
            c["sponsor"].as_str().unwrap_or("-"),
            metric_of(n),
            c["effort_engineer_days"],
            c["readiness"].as_str().unwrap()
        );
    }
    md += "\n## 3. Plan\n\n";
    md += "- **Week 1 — Access and baseline.** SSO and service accounts; read access to each system; golden set of 50–100 real cases per use case with SMEs; baseline metric measured.\n";
    md += "- **Week 2 — Build.** Agent and tools behind the MCP gateway; guardrails for sensitive data; offline eval on the golden set; weekly demo.\n";
    md += "- **Week 3 — Shadow.** Run on live traffic in shadow mode (no actions executed); compare with production; fix regressions; approval gates for writes.\n";
    md += "- **Week 4 — Pilot and handoff.** Supervised pilot for one team; dashboards for cost and quality; runbooks; admin portal for prompts and config; go / no-go review.\n\n";
    md += "## 4. Acceptance criteria\n\n";
    for n in &names {
        let m = metric_of(n);
        md += &format!(
            "- **{n}:** target: {}. Met on the golden set and on 2 weeks of shadow traffic, with no guardrail regression.\n",
            if m.is_empty() { "the metric agreed in week 1".to_string() } else { m }
        );
    }
    md += "- No sensitive data leaves approved boundaries (masking proxy and audit log reviewed by security).\n- Customer IT can change prompts and config and roll back without the vendor.\n\n";
    md += "## 5. Customer dependencies\n\n";
    let mut deps: Vec<String> = chosen
        .iter()
        .flat_map(|c| c["warnings"].as_array().cloned().unwrap_or_default())
        .filter_map(|w| w.as_str().map(String::from))
        .collect();
    deps.sort();
    deps.dedup();
    md += "- Named sponsor and one SME per use case, 3 hours / week.\n- Access requests approved by end of week 1 (delays move the plan day for day).\n";
    for w in deps {
        md += &format!("- {w}.\n");
    }
    md += "\n## 6. Out of scope\n\n";
    for x in &deferred {
        md += &format!(
            "- {} ({})\n",
            x["use_case"].as_str().unwrap_or(""),
            x["reason"].as_str().unwrap_or("")
        );
    }
    md += "- Production rollout beyond the pilot team; model fine-tuning; changes to source systems.\n\n";
    md += "## 7. Architecture\n\n```mermaid\n";
    md += &arch::mermaid(&d, &names);
    md += "```\n\n";
    let threats = crate::threat::model(&d, &names);
    md += &format!("## 8. Threat model (STRIDE)\n\n{} threats, {} high. Each mitigation ships with the test that proves it.\n\n",
        threats.len(), threats.iter().filter(|t| t["level"] == "high").count());
    md += &crate::threat::markdown(&threats);
    Ok(
        json!({"sow_markdown": md, "selected": names, "threats": threats.len(), "engineer_days_used": used, "deferred": deferred}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fits_budget_and_defers_rest() {
        let i = json!({"engineer_days": 12, "discovery": {"customer": "Acme", "systems": [{"name": "Zendesk", "access": "rest", "owner": "it"}],
            "use_cases": [
                {"name": "a", "volume_per_month": 5000, "minutes_per_task": 4, "systems": ["Zendesk"], "has_examples": true, "success_metric": "accuracy ≥ 90%"},
                {"name": "b", "volume_per_month": 3000, "minutes_per_task": 4, "systems": ["Zendesk"], "has_examples": true, "success_metric": "x"},
                {"name": "c", "volume_per_month": 9000, "minutes_per_task": 9, "systems": ["Missing"], "success_metric": "y"}]}});
        let r = draft_op(&i).unwrap();
        assert_eq!(r["selected"].as_array().unwrap().len(), 2);
        assert_eq!(r["deferred"].as_array().unwrap().len(), 1);
        assert!(r["sow_markdown"].as_str().unwrap().contains("```mermaid"));
    }
}
