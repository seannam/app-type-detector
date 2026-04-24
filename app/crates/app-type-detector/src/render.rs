#![allow(missing_docs)]

//! Human-readable renderer. Consumes only the JSON-shaped report so the same
//! output is reachable from any binding.

use serde_json::Value;

use crate::types::report::DetectionReport;

pub fn render(report: &DetectionReport) -> String {
    let json = serde_json::to_value(report).unwrap_or(Value::Null);
    render_json(&json)
}

pub fn render_json(value: &Value) -> String {
    let mut out = String::new();

    // Header line: use the project name if supplied via scorecard.ignored_paths
    // (we do not attempt to infer a name; callers can prepend it).
    out.push_str("(unnamed)\n\n");

    // App Type block.
    let app_type = value.get("app_type").cloned().unwrap_or(Value::Null);
    out.push_str("App Type\n");
    let primary = app_type
        .get("primary")
        .and_then(|v| v.as_str())
        .map(String::from);
    let confidence = app_type
        .get("confidence")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    match primary.as_deref() {
        Some(p) => out.push_str(&format!(
            "  {} ({}%)\n",
            p,
            (confidence * 100.0).round() as i64
        )),
        None => out.push_str("  unable to determine a single app type (no rule dominated)\n"),
    }
    if let Some(alts) = app_type.get("alternatives").and_then(|v| v.as_array()) {
        if !alts.is_empty() {
            if primary.is_none() {
                out.push_str("  Candidates:\n");
            }
            for alt in alts {
                let v = alt.get("value").and_then(|v| v.as_str()).unwrap_or("");
                let c = alt
                    .get("confidence")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0);
                out.push_str(&format!("    · {} ({}%)\n", v, (c * 100.0).round() as i64));
            }
        }
    }

    out.push('\n');

    // Tech Stack block.
    out.push_str("Tech Stack\n");
    let ts = value.get("tech_stack").cloned().unwrap_or(Value::Null);

    if let Some(langs) = ts.get("languages") {
        let primary_lang = langs.get("primary").and_then(|v| v.as_str()).unwrap_or("");
        let all = langs
            .get("all")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        let rest: Vec<String> = all
            .iter()
            .filter_map(|u| u.get("language").and_then(|v| v.as_str()).map(String::from))
            .filter(|l| l != primary_lang)
            .collect();
        if !primary_lang.is_empty() || !rest.is_empty() {
            let mut parts = Vec::new();
            if !primary_lang.is_empty() {
                parts.push(format!("{} (primary)", pretty_value(primary_lang)));
            }
            parts.extend(rest.into_iter().map(|l| pretty_value(&l)));
            push_row(&mut out, "Languages", &parts.join(", "));
        }
    }

    push_list(&mut out, "Build System", ts.get("build_systems"));
    push_list(&mut out, "Package Mgr", ts.get("package_managers"));
    push_list(&mut out, "Runtime", ts.get("runtimes"));
    push_list(&mut out, "Platforms", ts.get("platforms"));
    push_list(&mut out, "Databases", ts.get("databases"));
    push_list(&mut out, "Caches", ts.get("caches"));
    push_list(&mut out, "Queues", ts.get("queues"));
    push_list(&mut out, "Storage", ts.get("storage"));
    push_list(&mut out, "Testing", ts.get("testing"));
    push_list(&mut out, "Linting", ts.get("linting"));
    push_list(&mut out, "Formatting", ts.get("formatting"));
    push_list(&mut out, "CI", ts.get("ci"));
    push_list(&mut out, "Containers", ts.get("containerization"));
    push_list(&mut out, "Orchestration", ts.get("orchestration"));
    push_list(&mut out, "IaC", ts.get("iac"));
    push_list(&mut out, "Observability", ts.get("observability"));
    push_list(&mut out, "Auth", ts.get("auth_providers"));
    push_list(&mut out, "Payments", ts.get("payment_processors"));

    if let Some(web) = ts.get("web").filter(|v| !v.is_null()) {
        out.push('\n');
        out.push_str("Web\n");
        push_list(&mut out, "Backend", web.get("backend_frameworks"));
        push_list(&mut out, "Frontend", web.get("frontend_frameworks"));
        push_list(&mut out, "Styling", web.get("css_frameworks"));
        push_list(&mut out, "Bundler", web.get("bundlers"));
        if let Some(ssr) = web.get("ssr_strategy").and_then(|v| v.as_str()) {
            push_row(&mut out, "SSR", ssr);
        }
        push_list(&mut out, "API style", web.get("api_styles"));
        push_list(&mut out, "ORM", web.get("orms"));
    }

    if let Some(mobile) = ts.get("mobile").filter(|v| !v.is_null()) {
        out.push('\n');
        out.push_str("Mobile\n");
        push_list(&mut out, "UI", mobile.get("ui_frameworks"));
        push_list(&mut out, "Notable SDKs", mobile.get("notable_sdks"));
    }

    if let Some(desktop) = ts.get("desktop").filter(|v| !v.is_null()) {
        out.push('\n');
        out.push_str("Desktop\n");
        push_list(&mut out, "Shell", desktop.get("shells"));
        push_list(&mut out, "Installer", desktop.get("installer_formats"));
    }

    if let Some(game) = ts.get("game").filter(|v| !v.is_null()) {
        out.push('\n');
        out.push_str("Game\n");
        if let Some(engines) = game.get("engines").and_then(|v| v.as_array()) {
            let engine = engines
                .first()
                .and_then(|v| v.as_str())
                .map(String::from)
                .unwrap_or_default();
            let version = game
                .get("engine_version")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let label = if version.is_empty() {
                pretty_value(&engine)
            } else {
                format!("{} {}", pretty_value(&engine), version)
            };
            push_row(&mut out, "Engine", &label);
        }
        push_list(&mut out, "Rendering", game.get("rendering_pipelines"));
        push_list(&mut out, "Shaders", game.get("shader_languages"));
        push_list(&mut out, "Physics", game.get("physics_engines"));
        push_list(&mut out, "Networking", game.get("networking"));
    }

    if let Some(ext) = ts.get("extension").filter(|v| !v.is_null()) {
        out.push('\n');
        out.push_str("Extension\n");
        if let Some(host) = ext.get("host").and_then(|v| v.as_str()) {
            push_row(&mut out, "Host", &pretty_value(host));
        }
        if let Some(kind) = ext.get("kind").and_then(|v| v.as_str()) {
            push_row(&mut out, "Kind", &pretty_value(kind));
        }
    }

    // Scorecard footer.
    if let Some(sc) = value.get("scorecard") {
        let evaluated = sc
            .get("rules_evaluated")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        let fired = sc.get("rules_fired").and_then(|v| v.as_u64()).unwrap_or(0);
        let elapsed = sc.get("elapsed_ms").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let rsv = value
            .get("ruleset_version")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        out.push('\n');
        out.push_str(&format!(
            "Scorecard ({}/{} rules fired in {:.1} ms · ruleset v{})\n",
            fired, evaluated, elapsed, rsv
        ));
        if let Some(fires) = sc.get("fires").and_then(|v| v.as_array()) {
            for fire in fires {
                let id = fire.get("rule_id").and_then(|v| v.as_str()).unwrap_or("");
                let w = fire.get("weight").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let summary = summarize_contributions(fire);
                out.push_str(&format!("  ✓ {:<25}  w={:.2}   →  {}\n", id, w, summary));
            }
        }
        if let Some(warnings) = sc.get("warnings").and_then(|v| v.as_array()) {
            for w in warnings {
                if let Some(s) = w.as_str() {
                    out.push_str(&format!("  ! {}\n", s));
                }
            }
        }
    }

    out
}

fn push_row(out: &mut String, label: &str, value: &str) {
    out.push_str(&format!("  {:<15} {}\n", label, value));
}

fn push_list(out: &mut String, label: &str, value: Option<&Value>) {
    let Some(arr) = value.and_then(|v| v.as_array()) else {
        return;
    };
    if arr.is_empty() {
        return;
    }
    let items: Vec<String> = arr
        .iter()
        .filter_map(|v| v.as_str().map(pretty_value))
        .collect();
    if items.is_empty() {
        return;
    }
    push_row(out, label, &items.join(", "));
}

fn summarize_contributions(fire: &Value) -> String {
    let Some(list) = fire.get("contributes_to").and_then(|v| v.as_array()) else {
        return String::new();
    };
    let mut parts = Vec::new();
    for c in list {
        let field = c.get("field").and_then(|v| v.as_str()).unwrap_or("");
        let value = match c.get("value") {
            Some(Value::String(s)) => s.clone(),
            Some(v) => v.to_string(),
            None => String::new(),
        };
        let leaf = field.rsplit('.').next().unwrap_or(field);
        parts.push(format!("{}={}", leaf, value));
    }
    parts.join(", ")
}

fn pretty_value(s: &str) -> String {
    match s {
        "csharp" => "C#".into(),
        "cpp" => "C++".into(),
        "typescript" => "TypeScript".into(),
        "javascript" => "JavaScript".into(),
        "hlsl" => "HLSL".into(),
        "glsl" => "GLSL".into(),
        "shaderlab" => "ShaderLab".into(),
        "wgsl" => "WGSL".into(),
        "metal" => "Metal".into(),
        "gdscript" => "GDScript".into(),
        "godot_shader" => "Godot Shader".into(),
        "unity" => "Unity".into(),
        "godot" => "Godot".into(),
        "bevy" => "Bevy".into(),
        "unreal" => "Unreal".into(),
        "nextjs" => "Next.js".into(),
        "astro" => "Astro".into(),
        "fastapi" => "FastAPI".into(),
        "swiftui" => "SwiftUI".into(),
        "jetpack_compose" => "Jetpack Compose".into(),
        "react" => "React".into(),
        "tailwindcss" => "Tailwind CSS".into(),
        "turbopack" => "Turbopack".into(),
        "postgres" => "PostgreSQL".into(),
        "mysql" => "MySQL".into(),
        "sqlite" => "SQLite".into(),
        "mongodb" => "MongoDB".into(),
        "redis" => "Redis".into(),
        "node" => "Node".into(),
        "deno" => "Deno".into(),
        "bun" => "Bun".into(),
        "dotnet" => ".NET".into(),
        "mono" => "Mono".into(),
        "ios" => "iOS".into(),
        "android" => "Android".into(),
        "macos" => "macOS".into(),
        "windows" => "Windows".into(),
        "linux" => "Linux".into(),
        "web" => "Web".into(),
        "visionos" => "visionOS".into(),
        "tvos" => "tvOS".into(),
        "watchos" => "watchOS".into(),
        "steamdeck" => "Steam Deck".into(),
        "github_actions" => "GitHub Actions".into(),
        "gitlab_ci" => "GitLab CI".into(),
        "circleci" => "CircleCI".into(),
        "docker" => "Docker".into(),
        "podman" => "Podman".into(),
        "vitest" => "Vitest".into(),
        "jest" => "Jest".into(),
        "pytest" => "pytest".into(),
        "eslint" => "ESLint".into(),
        "prisma" => "Prisma".into(),
        "stripe" => "Stripe".into(),
        "paddle" => "Paddle".into(),
        "npm" => "npm".into(),
        "pnpm" => "pnpm".into(),
        "yarn" => "yarn".into(),
        "cargo" => "Cargo".into(),
        "uv" => "uv".into(),
        "nuget" => "NuGet".into(),
        "unity_package_manager" => "Unity Package Manager".into(),
        "physx" => "PhysX".into(),
        "urp" => "URP (Universal Render Pipeline)".into(),
        "hdrp" => "HDRP (High Definition Render Pipeline)".into(),
        "builtin" => "Built-in".into(),
        "tauri" => "Tauri".into(),
        "electron" => "Electron".into(),
        other => {
            let mut chars = other.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        }
    }
}
