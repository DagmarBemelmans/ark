//! Zed extension entry point for R (Ark).
//!
//! For now this only contributes the tree-sitter grammar and query files that
//! give `.R` files syntax highlighting, brackets, indents, and an outline. The
//! language server hookup (launching `ark` in LSP mode) lands in task T06; until
//! then `language_server_command` returns an explanatory error so that Zed shows
//! a clear message instead of silently failing.

use zed_extension_api as zed;

struct ArkRExtension;

impl zed::Extension for ArkRExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        _worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        Err("The Ark language server is not wired up yet (task T06). \
             This build of the extension only provides syntax highlighting. \
             To try the server manually, configure `lsp.ark.binary` in your \
             Zed settings to point at the `ark` binary."
            .to_string())
    }
}

zed::register_extension!(ArkRExtension);
