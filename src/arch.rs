//! Mermaid architecture diagram for selected use cases.

use crate::{discovery, Discovery};
use serde_json::{json, Value};

fn id(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}

fn label(s: &str) -> String {
    s.replace('"', "'")
}

pub fn mermaid(d: &Discovery, selected: &[String]) -> String {
    let ucs: Vec<_> = d
        .use_cases
        .iter()
        .filter(|u| selected.is_empty() || selected.contains(&u.name))
        .collect();
    let mut sys: Vec<&str> = ucs
        .iter()
        .flat_map(|u| u.systems.iter().map(String::as_str))
        .collect();
    sys.sort();
    sys.dedup();
    let sensitive = sys.iter().any(|s| {
        d.systems
            .iter()
            .any(|x| x.name == *s && ["pii", "phi", "pci"].contains(&x.sensitivity.as_str()))
    });
    let writes = ucs.iter().any(|u| u.writes);
    let mut m = String::from("flowchart LR\n");
    m.push_str("  user([\"Employees / customers\"]) --> sso[\"SSO + agent RBAC\"]\n");
    m.push_str("  sso --> agent[\"Agent service\"]\n");
    if sensitive {
        m.push_str("  agent --> pii[\"PII masking proxy\"]\n  pii --> llm[\"LLM endpoint\"]\n");
    } else {
        m.push_str("  agent --> llm[\"LLM endpoint\"]\n");
    }
    m.push_str("  agent --> mcp[\"MCP tool gateway\"]\n");
    if writes {
        m.push_str("  mcp --> hitl{\"Approval gate\"}\n");
    }
    m.push_str(&format!(
        "  subgraph cust[\"{} systems\"]\n",
        label(&d.customer)
    ));
    for s in &sys {
        let access = d
            .systems
            .iter()
            .find(|x| x.name == *s)
            .map(|x| x.access.as_str())
            .unwrap_or("unknown");
        m.push_str(&format!("    {}[(\"{} · {}\")]\n", id(s), label(s), access));
    }
    m.push_str("  end\n");
    for s in &sys {
        let w = ucs
            .iter()
            .any(|u| u.writes && u.systems.iter().any(|x| x == s));
        if w {
            m.push_str(&format!("  hitl -- write --> {}\n", id(s)));
        }
        m.push_str(&format!("  mcp -- read --> {}\n", id(s)));
    }
    m.push_str("  agent -.-> obs[\"Traces, cost, evals\"]\n");
    m
}

pub fn diagram_op(i: &Value) -> Result<Value, String> {
    let d = discovery(i)?;
    let sel: Vec<String> = i["use_cases"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default();
    Ok(
        json!({"mermaid": mermaid(&d, &sel), "render": "paste into any Mermaid renderer, a GitHub markdown ```mermaid block, or mermaid.live"}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pii_and_writes_add_controls() {
        let d: Discovery = serde_json::from_value(json!({"customer": "Acme \"Co\"", "systems": [{"name": "SAP ECC", "access": "soap", "sensitivity": "pii"}],
            "use_cases": [{"name": "u", "volume_per_month": 1, "minutes_per_task": 1, "systems": ["SAP ECC"], "writes": true}]})).unwrap();
        let m = mermaid(&d, &[]);
        assert!(
            m.contains("PII masking proxy")
                && m.contains("hitl -- write --> SAP_ECC")
                && !m.contains("\"Co\"")
        );
    }
}
