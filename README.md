# fde-11-sow-scoper

**Forward Deployed Engineering — Stage 11 of 12: Technical Discovery & SOW Scoping**

Turn a discovery workshop and executive 'AI hype' into a ranked gap analysis, an architecture diagram and a scoped 4-week statement of work with measurable acceptance criteria.

**Live demo:** https://fde-11-sow-scoper.braylo777.workers.dev — the Rust core runs in your browser as WebAssembly. No server, no tracking.

## Why this matters

Most failed AI projects were scoped badly, not built badly. The executive wants 'AI everywhere'; the systems have no APIs; nobody agreed on a metric. An FDE's first deliverable is a plan the customer can sign: one or two use cases with real value, honest effort, named dependencies, and acceptance criteria that can pass or fail. This repo makes that scoping repeatable and explains every number.

## What it does

- Hype translation: exec quotes to themes, measurable 4-week goals, discovery questions and red flags
- Gap analysis: value (volume x minutes x automatable share x rate) vs. effort (integration by access type, writes, PII guardrails, golden-set work); readiness and blockers
- Ranking by value per engineer-day, blocked items last
- Mermaid architecture diagram that adds a PII proxy and approval gate only when the data and writes need them
- SOW in Markdown: objective, scope table, week-by-week plan, acceptance criteria, dependencies, out of scope, diagram

## What it does not do (yet)

- Effort constants are a starting heuristic; calibrate them with your own delivery history
- No interview transcription or note-taking: discovery input is structured JSON
- Pricing and commercial terms are not generated

## Operations

Every operation takes JSON in and gives JSON out. The CLI, the tests and the demo page all use the same entry point.

| Operation | What it does |
|---|---|
| `translate_hype` | Three quotes from the kickoff meeting, turned into scoped goals, the questions to ask next, and red flags to raise early. |
| `gap_analysis` | Four candidate use cases ranked by value per engineer-day. Claims intake has the most volume but is red: the AS/400 has no API. |
| `architecture_diagram` | Diagram for the two chosen use cases. The PII proxy appears because Zendesk holds customer data; the approval gate because invoice disputes write to SAP. |
| `draft_sow` | Fits the best ready use cases into 25 engineer-days and writes the SOW. Lower engineer_days to see items move to out of scope. |

## Run it

You need Rust (stable). No Node, no npm.

```bash
git clone https://github.com/braylo-ai/fde-11-sow-scoper
cd fde-11-sow-scoper
cargo test
cargo run -- ops
```

Try the first demo example from the command line:

```bash
python3 -c 'import json;print(json.dumps(json.load(open("web/examples.json"))["ops"][0]["input"]))' | cargo run -q -- translate_hype
```

## Build and deploy the demo

```bash
./build.sh    # tests, then compiles the core to web/app.wasm
./deploy.sh   # serves web/ as a Cloudflare Workers static-assets site (needs a Cloudflare login)
```

`web/` is a plain static site (`index.html`, `app.js`, `app.wasm`). It has no build step, so any static host works.

## Layout

```
src/lib.rs        operation list and router
src/api.rs        JSON entry point + the 3 WASM exports (alloc, dealloc, run)
src/main.rs       CLI
src/*.rs          one module per capability, each with unit tests
tests/            integration tests (every demo example must pass)
web/              demo page; examples.json drives both the page and this README
```

## Design choices

- **Rust, few dependencies.** Only `serde`, `serde_json`, `sha2` and `hmac`, pinned to exact versions with a committed `Cargo.lock`.
- **Deterministic.** Time, randomness and keys are inputs, so every result can be replayed in a test or an audit.
- **Fail closed.** Unknown input, missing fields or an unmatched policy gives an error or a denial, never a silent pass.

## Part of a series

This is one of 12 repos that work through a public Forward Deployed Engineering roadmap ([source post](https://x.com/suraj_sharma14/status/2124827808535048566)). All 12: [github.com/braylo-ai?q=fde-](https://github.com/braylo-ai?tab=repositories&q=fde-).

## License

MIT — see [LICENSE](LICENSE). Built by [Braylo SI](https://braylo.si).
