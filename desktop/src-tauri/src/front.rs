// Whether the Claude app is the one in front: an answer that comes while it
// is not has no reader, whatever session the app has on show.

use objc2_app_kit::NSWorkspace;

const CLAUDE: &str = "com.anthropic.claudefordesktop";

pub fn claude_in_front() -> bool {
    let front = NSWorkspace::sharedWorkspace().frontmostApplication();
    front.and_then(|app| app.bundleIdentifier()).is_some_and(|id| id.to_string() == CLAUDE)
}
