//! CLI: `fde-11-sow-scoper <op> [file.json]` (reads JSON input from the file or stdin).
//! `fde-11-sow-scoper ops` lists the operations.
use std::io::Read;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let op = args.get(1).map(String::as_str).unwrap_or("ops");
    if op == "ops" || op == "--help" || op == "-h" {
        println!("fde-11-sow-scoper: Turn discovery notes and executive AI hype into a gap analysis, an architecture diagram and a scoped 4-week statement of work.");
        println!("usage: fde-11-sow-scoper <op> [input.json]   (input JSON from file or stdin)");
        for o in fde_11_sow_scoper::OPS {
            println!("  {o}");
        }
        return;
    }
    let mut raw = String::new();
    match args.get(2) {
        Some(p) => raw = std::fs::read_to_string(p).expect("read input file"),
        None => {
            std::io::stdin().read_to_string(&mut raw).expect("read stdin");
        }
    }
    let input: serde_json::Value = serde_json::from_str(if raw.trim().is_empty() { "null" } else { &raw })
        .expect("input must be JSON");
    let req = serde_json::json!({"op": op, "input": input}).to_string();
    let out: serde_json::Value = serde_json::from_str(&fde_11_sow_scoper::dispatch(&req)).unwrap();
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
    if out["ok"] != true {
        std::process::exit(1);
    }
}
