use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AppType {
    WebApp,
    WebApi,
    StaticSite,
    MobileApp,
    DesktopApp,
    Game,
    CliTool,
    Library,
    Daemon,
    BrowserExtension,
    EditorExtension,
    CmsPlugin,
    McpServer,
    ClaudeSkill,
    Unknown,
}

impl AppType {
    pub fn as_str(&self) -> &'static str {
        match self {
            AppType::WebApp => "web_app",
            AppType::WebApi => "web_api",
            AppType::StaticSite => "static_site",
            AppType::MobileApp => "mobile_app",
            AppType::DesktopApp => "desktop_app",
            AppType::Game => "game",
            AppType::CliTool => "cli_tool",
            AppType::Library => "library",
            AppType::Daemon => "daemon",
            AppType::BrowserExtension => "browser_extension",
            AppType::EditorExtension => "editor_extension",
            AppType::CmsPlugin => "cms_plugin",
            AppType::McpServer => "mcp_server",
            AppType::ClaudeSkill => "claude_skill",
            AppType::Unknown => "unknown",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "web_app" => AppType::WebApp,
            "web_api" => AppType::WebApi,
            "static_site" => AppType::StaticSite,
            "mobile_app" => AppType::MobileApp,
            "desktop_app" => AppType::DesktopApp,
            "game" => AppType::Game,
            "cli_tool" => AppType::CliTool,
            "library" => AppType::Library,
            "daemon" => AppType::Daemon,
            "browser_extension" => AppType::BrowserExtension,
            "editor_extension" => AppType::EditorExtension,
            "cms_plugin" => AppType::CmsPlugin,
            "mcp_server" => AppType::McpServer,
            "claude_skill" => AppType::ClaudeSkill,
            "unknown" => AppType::Unknown,
            _ => return None,
        })
    }
}
