//! Stage 11 — Technical Discovery & SOW Scoping.
//!
//! * `hype`  — translate executive statements ("AI everywhere", "replace the call center")
//!   into measurable, scoped goals plus the discovery questions that make them concrete.
//! * `gap`   — score each candidate use case: value (volume x minutes), integration effort per
//!   system, data sensitivity, write access, missing data; readiness and blockers.
//! * `arch`  — a Mermaid architecture diagram for the selected use cases.
//! * `sow`   — pick what fits a 4-week, N-engineer-day budget and write the SOW in Markdown:
//!   weekly plan, deliverables, measurable acceptance criteria, out of scope, dependencies.

mod api;
pub mod arch;
pub mod gap;
pub mod hype;
pub mod sow;

pub use api::dispatch;
use serde::Deserialize;
use serde_json::Value;

pub const OPS: &[&str] = &["translate_hype", "gap_analysis", "architecture_diagram", "draft_sow"];

#[derive(Debug, Clone, Deserialize)]
pub struct System {
    pub name: String,
    /// rest | graphql | soap | db | file_export | none
    pub access: String,
    #[serde(default)]
    pub sensitivity: String, // public | internal | pii | phi | pci
    #[serde(default)]
    pub owner: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UseCase {
    pub name: String,
    #[serde(default)]
    pub sponsor: String,
    pub volume_per_month: f64,
    pub minutes_per_task: f64,
    #[serde(default = "auto")]
    pub automatable_share: f64,
    pub systems: Vec<String>,
    #[serde(default)]
    pub writes: bool,
    #[serde(default)]
    pub has_examples: bool,
    #[serde(default)]
    pub success_metric: String,
}
fn auto() -> f64 {
    0.6
}

#[derive(Debug, Clone, Deserialize)]
pub struct Discovery {
    pub customer: String,
    pub systems: Vec<System>,
    pub use_cases: Vec<UseCase>,
    #[serde(default = "rate")]
    pub loaded_hourly_rate: f64,
}
fn rate() -> f64 {
    55.0
}

pub(crate) fn discovery(i: &Value) -> Result<Discovery, String> {
    serde_json::from_value(i.get("discovery").cloned().ok_or("missing `discovery`")?).map_err(|e| format!("discovery: {e}"))
}

pub fn handle(op: &str, i: &Value) -> Result<Value, String> {
    match op {
        "translate_hype" => hype::translate_op(i),
        "gap_analysis" => gap::analyze_op(i),
        "architecture_diagram" => arch::diagram_op(i),
        "draft_sow" => sow::draft_op(i),
        _ => Err(format!("unknown op `{op}`; known: {}", OPS.join(", "))),
    }
}
