# `app-type-detector` (Python binding)

Placeholder for the `pyo3` + `maturin` Python binding. The core crate under
`app/crates/app-type-detector` is shipped in v0.1.0; the Python binding is
scheduled for v0.2.

Planned surface:

```python
from app_type_detector import detect_path, detect_files, default_ruleset, render_human_readable

report = detect_path("./my-project")
print(render_human_readable(report))
```

Build strategy: ABI3 wheels so Python 3.10+ shares one binary per triple.
