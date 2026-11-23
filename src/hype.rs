//! Translate executive "AI hype" into scoped, measurable goals.
//! Rule table: pattern → (what it usually means, a measurable 4-week goal, questions to ask).

use serde_json::{json, Value};

struct Rule {
    any: &'static [&'static str],
    theme: &'static str,
    goal: &'static str,
    questions: &'static [&'static str],
    red_flag: Option<&'static str>,
}

const RULES: &[Rule] = &[
    Rule { any: &["replace", "headcount", "fewer people", "cut staff"], theme: "labor substitution",
        goal: "Deflect or fully automate X% of one ticket category, measured on live traffic, with human fallback",
        questions: &["Which single queue or task type has the highest volume?", "What is today's handle time and cost per ticket?", "Who reviews the AI's answers during the pilot?"],
        red_flag: Some("Promising headcount cuts before a pilot sets the project up to fail; commit to a measured deflection rate instead.") },
    Rule { any: &["everywhere", "every department", "transform", "ai-first", "ai first"], theme: "broad transformation",
        goal: "One production use case live in 4 weeks, with a reusable platform (auth, logging, evals) for the next ones",
        questions: &["Which team has a painful, repetitive task and a willing sponsor?", "What would make the next use case 2x faster to ship?"],
        red_flag: Some("'Everywhere' has no owner and no metric; pick one team first.") },
    Rule { any: &["10x", "productivity", "faster", "efficiency"], theme: "productivity",
        goal: "Cut median time per task by N% for a named workflow, measured before vs. after with the same staff",
        questions: &["How is time per task measured today?", "Which step takes longest: finding information, drafting, or approvals?"],
        red_flag: None },
    Rule { any: &["chatbot", "chatgpt", "assistant", "copilot"], theme: "internal assistant",
        goal: "Answer questions from a defined document set with citations; target X% correct on a 100-question eval",
        questions: &["Which documents are the source of truth, and who may see each?", "What questions do people ask today, and where?"],
        red_flag: Some("A general chatbot without a document scope or eval set cannot be accepted or rejected.") },
    Rule { any: &["agent", "autonomous", "on its own", "no human"], theme: "autonomous action",
        goal: "Agent completes one workflow end to end in shadow mode, then with human approval above a risk threshold",
        questions: &["Which actions are reversible, and which are not?", "What limit (amount, record count) needs a human approval?"],
        red_flag: Some("Fully autonomous writes to production systems in week 1 are a security and audit risk; start in shadow mode.") },
    Rule { any: &["data", "insight", "dashboard", "analytics"], theme: "insight from data",
        goal: "Natural-language answers over N approved tables, matching analyst answers on a 50-question eval",
        questions: &["Which tables, and is there a semantic layer or data dictionary?", "Which numbers must never be wrong (finance, compliance)?"],
        red_flag: None },
    Rule { any: &["customer experience", "cx", "nps", "csat"], theme: "customer experience",
        goal: "Raise first-contact resolution or CSAT on one channel by N points vs. a control group",
        questions: &["Which channel and which contact reasons are in scope?", "How is CSAT collected today and at what response rate?"],
        red_flag: None },
];

pub fn translate_op(i: &Value) -> Result<Value, String> {
    let statements = i["statements"]
        .as_array()
        .ok_or("missing `statements` (list of exec quotes)")?;
    let mut out = Vec::new();
    for s in statements {
        let text = s.as_str().unwrap_or("");
        let low = text.to_lowercase();
        let hits: Vec<&Rule> = RULES
            .iter()
            .filter(|r| r.any.iter().any(|k| low.contains(k)))
            .collect();
        if hits.is_empty() {
            out.push(json!({"statement": text, "themes": [], "scoped_goals": [], "questions": ["What outcome would make this a success in 30 days, and how would we measure it?"], "red_flags": []}));
            continue;
        }
        out.push(json!({
            "statement": text,
            "themes": hits.iter().map(|r| r.theme).collect::<Vec<_>>(),
            "scoped_goals": hits.iter().map(|r| r.goal).collect::<Vec<_>>(),
            "questions": hits.iter().flat_map(|r| r.questions.iter().copied()).collect::<Vec<_>>(),
            "red_flags": hits.iter().filter_map(|r| r.red_flag).collect::<Vec<_>>()
        }));
    }
    Ok(json!({"translations": out}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn maps_hype_to_goals() {
        let r = translate_op(&json!({"statements": ["We want AI agents everywhere so we can replace half the call center", "Make the weather nice"]})).unwrap();
        let t = &r["translations"][0];
        let themes: Vec<&str> = t["themes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x.as_str().unwrap())
            .collect();
        assert!(
            themes.contains(&"labor substitution")
                && themes.contains(&"autonomous action")
                && themes.contains(&"broad transformation")
        );
        assert_eq!(t["red_flags"].as_array().unwrap().len(), 3);
        assert_eq!(r["translations"][1]["themes"], json!([]));
    }
}
