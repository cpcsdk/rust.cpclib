//! The minijinja environment a `bndbuild` file is expanded through.
//!
//! Build files carry `{% set %}` variables and `{% include %}`s, so anything
//! that wants to read what a rule *actually* says has to render them first.
//! The environment is the part with real decisions in it - lenient undefined
//! handling, trailing-newline preservation, a loader rooted at the build
//! file's directory, and stubs for bndbuild's own template functions - so it
//! lives here rather than in whichever tool happened to need it first.

use minijinja::{Environment, UndefinedBehavior};

pub fn build_environment(file_dir: Option<&std::path::Path>) -> Environment<'static> {
    let mut env = Environment::new();
    // Lenient: undefined variables return Undefined instead of aborting.
    // This lets the expansion proceed even when external build-time variables
    // (e.g. definitions passed via `bndbuild -D`) are absent.
    env.set_undefined_behavior(UndefinedBehavior::Chainable);
    // Without this, minijinja strips exactly one trailing newline when
    // parsing *each* template - including an `{% include %}`d file's own
    // content, compiled as its own template. That strip makes the
    // included content's last line merge onto the same output line as
    // whatever immediately follows the `{% include %}` tag in the
    // includer (here, this module's own line-number marker), so that
    // marker ends up wrongly attached to the included file's content
    // instead of landing on its own line. Keeping the trailing newline
    // keeps included content and includer markers on separate lines, so
    // `to_original` correctly reports `None` for every included line.
    env.set_keep_trailing_newline(true);

    if let Some(dir) = file_dir {
        let dir = dir.to_path_buf();
        env.set_loader(move |name| {
            let path = dir.join(name);
            match std::fs::read_to_string(&path) {
                Ok(s) => Ok(Some(s)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => {
                    Err(minijinja::Error::new(
                        minijinja::ErrorKind::InvalidOperation,
                        "could not read include"
                    )
                    .with_source(e))
                },
            }
        });
    }

    // Register the same custom functions as bndbuild's create_template_env so
    // templates that call them don't abort the LSP expansion.
    fn lsp_fail(_msg: String) -> Result<String, minijinja::Error> {
        Ok(String::new())
    }
    fn lsp_assert(_ok: bool, _msg: String) -> Result<(), minijinja::Error> {
        Ok(())
    }
    fn lsp_basename(path: String) -> Result<String, minijinja::Error> {
        Ok(std::path::Path::new(&path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&path)
            .to_string())
    }
    fn lsp_escape(path: String) -> Result<String, minijinja::Error> {
        Ok(path)
    }

    env.add_function("fail", lsp_fail);
    env.add_function("assert", lsp_assert);
    env.add_function("basename", lsp_basename);
    env.add_function("basm_escape_path", lsp_escape);
    env.add_filter("basm_escape_path", |path: String| path);

    env
}

/// Render `source` through [`build_environment`].
///
/// For callers that want the expanded text and nothing else. The LSP's own
/// expansion adds line markers around this so it can map a rendered line back
/// to the line the user wrote; a caller that only reads values needs none of
/// that machinery.
pub fn expand(
    source: &str,
    file_dir: Option<&std::path::Path>
) -> Result<String, minijinja::Error> {
    build_environment(file_dir).render_str(source, minijinja::context!())
}
