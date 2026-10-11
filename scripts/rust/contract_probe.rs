//! Development-only public result probe, compiled against each compared core.
use std::{error::Error, io::Read};

use normalizer_tr::{AmbiguityPolicy, NormalizeError, NormalizeOptions, Normalizer};
use serde_json::{Value, json};

#[allow(dead_code)]
#[path = "../../tests/support/policy_contract.rs"]
mod policy_fixtures;

fn main() -> Result<(), Box<dyn Error>> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let cases: Vec<Value> = serde_json::from_str(&input)?;
    let normalizer = Normalizer::new()?;
    let mut rows = Vec::new();
    for case in cases {
        let text = case["text"].as_str().ok_or("case has no text")?;
        for (name, policy) in [
            ("preserve", AmbiguityPolicy::Preserve),
            ("reject", AmbiguityPolicy::Reject),
            ("fallback", AmbiguityPolicy::Fallback),
        ] {
            let mut options = NormalizeOptions {
                ambiguity_policy: policy,
                ..Default::default()
            };
            if let Some(hint) = policy_fixtures::hint(&case) {
                options.hints.push(hint);
            }
            let outcome = match normalizer.normalize(text, &options) {
                Ok(result) => json!({"result": result}),
                Err(NormalizeError::Unresolved(issues)) => {
                    json!({"error": "Unresolved", "issues": issues})
                }
                Err(NormalizeError::LimitExceeded(kind)) => {
                    json!({"error": "LimitExceeded", "limit": format!("{kind:?}")})
                }
                Err(error) => json!({"error": format!("{error:?}")}),
            };
            rows.push(json!({"id": case["id"], "policy": name, "outcome": outcome}));
        }
    }
    serde_json::to_writer(std::io::stdout().lock(), &rows)?;
    Ok(())
}
