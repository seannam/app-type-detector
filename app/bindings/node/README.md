# `@snam/app-type-detector` (Node binding)

Placeholder for the `napi-rs` Node binding. The core crate under
`app/crates/app-type-detector` is shipped in v0.1.0; the Node binding is
scheduled for v0.2.

Planned surface:

```ts
import { detectPath, detectFiles, defaultRuleset, renderHumanReadable } from "@snam/app-type-detector";

const report = detectPath("./my-project");
console.log(renderHumanReadable(report));
```

Build strategy: per-triple prebuilt binaries published as optional-dep
subpackages, with a WASM fallback for restricted runtimes.
