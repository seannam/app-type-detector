/* tslint:disable */
/* eslint-disable */

/**
 * TypeScript surface for `@indiecraft/app-type-detector`.
 *
 * The shapes below mirror `docs/02-output-format.md`. Every field is
 * byte-identical to the JSON produced by the Rust crate; bindings are
 * pass-throughs, never re-mappers.
 */

export type AppTypeValue =
  | "game"
  | "web_app"
  | "mobile_app"
  | "desktop_app"
  | "cli_tool"
  | "library"
  | "mcp_server"
  | "claude_skill"
  | "browser_extension"
  | "editor_extension"
  | "static_site"
  | "unknown"
  | (string & {});

export interface Alternative {
  value: string;
  confidence: number;
}

export interface AppTypeFinding {
  primary: AppTypeValue | null;
  confidence: number;
  alternatives: Alternative[];
}

export interface LanguageUsage {
  language: string;
  role?: string | null;
  file_count: number;
}

export interface LanguagesFinding {
  primary: string | null;
  all: LanguageUsage[];
}

export interface WebStack {
  frameworks: string[];
  rendering_modes: string[];
  css: string[];
  orms: string[];
  cdns: string[];
}

export interface MobileStack {
  platforms: string[];
  ui_toolkits: string[];
  architectures: string[];
  min_targets: Record<string, string>;
}

export interface DesktopStack {
  shells: string[];
  ui_toolkits: string[];
}

export interface GameStack {
  engines: string[];
  engine_version: string | null;
  rendering_pipelines: string[];
  shader_languages: string[];
  physics_engines: string[];
  networking: string[];
}

export interface ExtensionStack {
  targets: string[];
  manifest_version: string | null;
}

export interface TechStack {
  languages: LanguagesFinding;
  build_systems: string[];
  package_managers: string[];
  runtimes: string[];
  platforms: string[];

  databases: string[];
  caches: string[];
  queues: string[];
  storage: string[];
  testing: string[];
  linting: string[];
  formatting: string[];
  ci: string[];
  containerization: string[];
  orchestration: string[];
  iac: string[];
  observability: string[];
  auth_providers: string[];
  payment_processors: string[];

  web: WebStack | null;
  mobile: MobileStack | null;
  desktop: DesktopStack | null;
  game: GameStack | null;
  extension: ExtensionStack | null;
}

export interface Evidence {
  kind: string;
  path?: string | null;
  file?: string | null;
  regex?: string | null;
  captures?: string[];
  matched?: boolean | null;
  count?: number | null;
}

export interface Contribution {
  field: string;
  value?: string | number | null;
  delta?: number | null;
}

export interface Fire {
  rule_id: string;
  weight: number;
  evidence: Evidence[];
  contributes_to: Contribution[];
}

export interface InputSummary {
  files_scanned: number;
  bytes_scanned: number;
}

export interface Scorecard {
  rules_evaluated: number;
  rules_fired: number;
  elapsed_ms: number;
  input_summary: InputSummary;
  ignored_paths: string[];
  fires: Fire[];
  warnings: string[];
}

export interface DetectionReport {
  schema_version: number;
  ruleset_version: string;
  app_type: AppTypeFinding;
  tech_stack: TechStack;
  scorecard: Scorecard;
}

export interface DetectFilesInput {
  /** Map of relative paths to file contents. `null` marks an empty file. */
  files: Record<string, string | null>;
}

/** Detect the app type of a directory on disk. */
export declare function detectPath(path: string): DetectionReport;

/** Detect the app type of an in-memory file map. */
export declare function detectFiles(input: DetectFilesInput): DetectionReport;

/** Return the bundled default ruleset as a JSON-compatible object. */
export declare function defaultRuleset(): unknown;

/** Render a `DetectionReport` as human-readable text. */
export declare function renderHumanReadable(report: DetectionReport): string;
