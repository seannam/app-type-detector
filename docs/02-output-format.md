# Output format

`app-type-detector` always returns a `DetectionReport` — never a bare string.
The JSON wire format is the canonical contract; all renderers (text, TSV,
fires-JSONL) are derived from it.

## Top-level shape

```jsonc
{
  "schema_version": 1,
  "ruleset_version": "0.1.0",

  "app_type": {
    "primary": "game",        // snake_case enum value, or null when ambiguous
    "confidence": 0.97,       // 0..1
    "alternatives": [ { "value": "desktop_app", "confidence": 0.42 } ]
  },

  "tech_stack": {
    "languages": {
      "primary": "csharp",
      "all": [
        { "language": "csharp", "role": "gameplay", "file_count": 142 }
      ]
    },
    "build_systems":    ["unity"],
    "package_managers": ["nuget", "unity_package_manager"],
    "runtimes":         ["mono"],
    "platforms":        ["windows", "macos"],

    "databases": [], "caches": [], "queues": [], "storage": [],
    "testing":   [], "linting": [], "formatting": [],
    "ci":        ["github_actions"],
    "containerization": [], "orchestration": [], "iac": [],
    "observability":    [], "auth_providers": [], "payment_processors": [],

    // Optional sub-records, populated only when relevant.
    "web":       null,
    "mobile":    null,
    "desktop":   null,
    "game": {
      "engines": ["unity"],
      "engine_version": "2022.3.42f1",
      "rendering_pipelines": ["urp"],
      "shader_languages":    ["hlsl", "shaderlab"],
      "physics_engines":     ["physx"],
      "networking":          []
    },
    "extension": null
  },

  "scorecard": {
    "rules_evaluated": 37,
    "rules_fired":     4,
    "elapsed_ms":      3.4,
    "input_summary": { "files_scanned": 412, "bytes_scanned": 1832004 },
    "ignored_paths": [".git", "Library", "Temp", "obj", "bin"],
    "fires": [
      {
        "rule_id": "unity-engine",
        "weight":  1.0,
        "evidence": [
          { "kind": "file_exists", "path": "ProjectSettings/ProjectSettings.asset", "matched": true },
          { "kind": "content", "file": "ProjectSettings/ProjectVersion.txt",
            "regex": "m_EditorVersion:\\s*(\\S+)", "captures": ["2022.3.42f1"] }
        ],
        "contributes_to": [
          { "field": "app_type", "value": "game", "delta": 1.0 },
          { "field": "tech_stack.game.engines", "value": "unity" }
        ]
      }
    ],
    "warnings": []
  }
}
```

## Output formats

| Format          | Producer                          | Consumer                                  |
|-----------------|-----------------------------------|-------------------------------------------|
| `json`          | `report.to_json()`                | services, archives, downstream tooling    |
| `text`          | `render_human_readable(report)`   | terminals, CI logs                        |
| `tsv`           | `report.to_tsv()`                 | shell pipelines                           |
| `fires-jsonl`   | `report.scorecard.fires_jsonl()`  | `jq -c` pipelines, audit logs             |

The `text` renderer reads the JSON shape — not internal types — so it is
identical across every binding.

## Primary rules

1. The library never collapses role and stack into a single string.
2. `app_type.primary` is `null` when no rule dominates by the configured
   margin (default 1.5x over the runner-up). A warning is emitted so consumers
   can fall back to a human or an LLM.
3. `schema_version` will only change for breaking changes to the wire format.
4. Unknown / polyglot / empty codebases are legitimate inputs.

See [`docs/03-rules.md`](03-rules.md) for the rule grammar.
