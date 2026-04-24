#![allow(missing_docs)]

//! Default, cached ruleset loaded from the embedded JSON.

use once_cell::sync::OnceCell;

use crate::rules::Ruleset;

const DEFAULT_RULES_JSON: &str = include_str!("default_rules.json");

static CACHED: OnceCell<Ruleset> = OnceCell::new();

pub fn default_ruleset() -> &'static Ruleset {
    CACHED.get_or_init(|| {
        Ruleset::from_json(DEFAULT_RULES_JSON)
            .expect("bundled default_rules.json failed to parse; this is a bug")
    })
}
