//! Zed extension entry point for R (Ark).
//!
//! This extension contributes the tree-sitter grammar and query files (syntax
//! highlighting, brackets, indents, outline) and launches `ark` in LSP mode so
//! Zed gets completions, hover, diagnostics, and jump-to-definition in `.R`
//! files. Zed speaks LSP to the launched process over its stdin/stdout.

use zed::settings::LspSettings;
use zed_extension_api as zed;

struct ArkRExtension;

impl zed::Extension for ArkRExtension {
    fn new() -> Self {
        Self
    }

    fn language_server_command(
        &mut self,
        _language_server_id: &zed::LanguageServerId,
        worktree: &zed::Worktree,
    ) -> zed::Result<zed::Command> {
        // Inherit the user's shell environment so that `PATH` and `R_HOME`
        // resolve the same R they get in a terminal.
        let mut env = worktree.shell_env();

        // The user can override the binary in Zed settings under
        // `lsp.ark.binary` (`path` / `arguments` / `env`). Pull those out once
        // so we can honor whichever fields are set.
        let binary = LspSettings::for_worktree("ark", worktree)
            .ok()
            .and_then(|settings| settings.binary);
        let (path_override, args_override, env_override) = match binary {
            Some(binary) => (binary.path, binary.arguments, binary.env),
            None => (None, None, None),
        };

        // Resolution order for the binary itself:
        //   1. an explicit `lsp.ark.binary.path` override, else
        //   2. an `ark` found on `$PATH`, else
        //   3. a clear error telling the user exactly what to do.
        let command = match path_override {
            Some(path) => path,
            None => worktree.which("ark").ok_or_else(ark_not_found_message)?,
        };

        // Honor `lsp.ark.binary.arguments` if set, otherwise start LSP mode.
        let args = args_override.unwrap_or_else(|| vec!["lsp".to_string()]);

        // Let any `lsp.ark.binary.env` overrides win over the inherited env by
        // appending them after the shell environment.
        if let Some(env_override) = env_override {
            env.extend(env_override);
        }

        Ok(zed::Command { command, args, env })
    }
}

/// Guidance shown in Zed's language-server logs when no `ark` binary can be
/// found. It tells the user how to build one or point Zed at an existing build.
fn ark_not_found_message() -> String {
    "Could not find an `ark` binary on your PATH.\n\n\
     Build it from the ark repository with `cargo build` (this produces \
     `target/debug/ark`), then either add that `target/debug` directory to your \
     PATH, or point Zed at the binary directly in your settings.json:\n\n\
     {\n  \"lsp\": {\n    \"ark\": {\n      \"binary\": {\n        \
     \"path\": \"/absolute/path/to/ark/target/debug/ark\",\n        \
     \"arguments\": [\"lsp\"]\n      }\n    }\n  }\n}"
        .to_string()
}

zed::register_extension!(ArkRExtension);
