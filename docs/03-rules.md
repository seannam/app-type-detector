# Rule grammar

A rule couples a `when` expression (how to recognize the situation) with a
`payload` (what to conclude). The bundled default ruleset is
`app/crates/app-type-detector/src/default_rules.json`; callers can also supply
their own or extend the defaults.

## Shape

```jsonc
{
  "schema_version": 1,
  "version": "0.1.0",
  "rules": [
    {
      "id": "nextjs-app",
      "description": "Next.js app",
      "when": {
        "kind": "all",
        "of": [
          { "kind": "file_exists", "path": "package.json" },
          { "kind": "content", "file": "package.json", "regex": "\"next\"\\s*:" }
        ]
      },
      "payload": {
        "confidence_weight": 1.0,
        "contributions": [
          { "field": "app_type", "value": "web_app", "delta": 1.0 },
          { "field": "tech_stack.web.backend_frameworks", "value": "nextjs" },
          { "field": "tech_stack.languages", "value": "typescript" }
        ]
      }
    }
  ]
}
```

## `when` expressions

| `kind`         | Fields                                   | Meaning                                        |
|----------------|------------------------------------------|------------------------------------------------|
| `file_exists`  | `path`                                   | Project contains that file.                    |
| `glob`         | `pattern`, `min_count` (default 1)       | At least `min_count` files match the pattern.  |
| `content`      | `file`, `regex`                          | File exists and its contents match the regex. Capture groups are lifted. |
| `all`          | `of: [expr]`                             | Every sub-expression matches.                  |
| `any`          | `of: [expr]`                             | At least one sub-expression matches.           |
| `not`          | `of: expr`                               | The inner expression does NOT match.           |

## `payload`

- `confidence_weight` (default 1.0) — how strongly this rule speaks.
- `contributions` — list of `{ field, value, delta? }` entries. `field` is a
  dotted path into `DetectionReport` (e.g. `tech_stack.web.backend_frameworks`
  or `app_type`). For list fields every contributing value is unioned and
  ranked by weight; for scalars (`tech_stack.game.engine_version`,
  `tech_stack.web.ssr_strategy`, `tech_stack.extension.host`,
  `tech_stack.extension.kind`, `tech_stack.languages.primary`) the highest
  weight wins.
- `captures_into` (optional) — lift the first capture group from a named
  `content` sub-expression into a scalar field.

## Adding a new ecosystem

1. Add a new entry to `default_rules.json`.
2. Add a fixture directory under
   `app/crates/app-type-detector/tests/fixtures/<new-fixture>/`.
3. Add a test in `tests/detect_path.rs` asserting the headline claim.
4. If a new enum variant is needed, add it with `#[non_exhaustive]` so callers
   are minor-version-safe.
