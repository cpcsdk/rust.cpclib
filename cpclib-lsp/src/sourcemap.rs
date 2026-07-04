use minijinja::{Environment, UndefinedBehavior};

const MARKER_PREFIX: &str = "#_SRCL:";
const MARKER_SUFFIX: &str = "_#";

/// Mapping from lines in the Jinja-expanded text back to lines in the original source.
/// Index is the expanded line number (0-based); value is the original line number (0-based),
/// or `None` for lines that were synthesised by Jinja with no direct source counterpart.
pub struct SourceMap {
    pub expanded_to_original: Vec<Option<u32>>
}

impl SourceMap {
    /// Convert an expanded-file line number to the original-file line number.
    pub fn to_original(&self, expanded_line: u32) -> Option<u32> {
        self.expanded_to_original
            .get(expanded_line as usize)
            .copied()
            .flatten()
    }

    /// Identity map for when we're working directly on the raw file.
    pub fn identity(line_count: usize) -> Self {
        Self {
            expanded_to_original: (0..line_count as u32).map(Some).collect()
        }
    }
}

/// Expand a Jinja template, returning the rendered text and a source map.
///
/// The source map maps each line in the output back to the line in `source`
/// that generated it.  Lines that come from Jinja control structures
/// (`{% for %}`, `{% if %}`, etc.) with no data payload map to `None`.
///
/// `file_dir` is the directory used as base path for `{% include %}` directives.
/// Pass `None` when no file-system includes are expected.
pub fn expand_with_source_map(
    source: &str,
    file_dir: Option<&std::path::Path>
) -> Result<(String, SourceMap), minijinja::Error> {
    // Step 1 — annotate every line with its original line number.
    let annotated = annotate(source);

    // Step 2 — render through minijinja.
    let mut env = Environment::new();
    // Lenient: undefined variables return Undefined instead of aborting.
    // This lets the expansion proceed even when external build-time variables
    // (e.g. definitions passed via `bndbuild -D`) are absent.
    env.set_undefined_behavior(UndefinedBehavior::Chainable);

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

    // No variable context — variables defined via {% set %} in the template
    // are sufficient; external definitions resolve to Undefined (lenient).
    let rendered = env.render_str(&annotated, minijinja::context!())?;

    // Step 3 — build the source map and strip markers.
    let mut map = Vec::new();
    let mut clean_lines = Vec::new();

    for line in rendered.lines() {
        let (clean, original) = strip_marker(line);
        clean_lines.push(clean);
        map.push(original);
    }

    let clean_text = clean_lines.join("\n");
    // Preserve trailing newline if the rendered output had one.
    let clean_text = if rendered.ends_with('\n') {
        format!("{}\n", clean_text)
    }
    else {
        clean_text
    };

    Ok((
        clean_text,
        SourceMap {
            expanded_to_original: map
        }
    ))
}

// ─── internal helpers ────────────────────────────────────────────────────────

fn annotate(source: &str) -> String {
    let mut out = String::with_capacity(source.len() + source.lines().count() * 16);
    let mut in_block_tag = false; // inside a {%…%} that hasn't closed yet

    for (i, line) in source.lines().enumerate() {
        let trimmed = line.trim_end();
        // Decide before annotating: does this line end inside an open block?
        let ends_in_block = line_ends_in_block_tag(trimmed, in_block_tag);

        if in_block_tag || ends_in_block {
            // Line is inside or starts a multi-line {% %} tag —
            // injecting the marker here would corrupt the Jinja expression.
            out.push_str(trimmed);
        }
        else {
            out.push_str(trimmed);
            out.push(' ');
            out.push_str(MARKER_PREFIX);
            out.push_str(&i.to_string());
            out.push_str(MARKER_SUFFIX);
        }
        out.push('\n');
        in_block_tag = ends_in_block;
    }
    out
}

/// Returns `true` if `line` ends while still inside an open `{%…%}` block tag.
/// `starts_inside` carries the state from the previous line.
fn line_ends_in_block_tag(line: &str, starts_inside: bool) -> bool {
    let bytes = line.as_bytes();
    let mut inside = starts_inside;
    let mut i = 0;
    while i + 1 < bytes.len() {
        if !inside && bytes[i] == b'{' && bytes[i + 1] == b'%' {
            inside = true;
            i += 2;
        }
        else if inside && bytes[i] == b'%' && bytes[i + 1] == b'}' {
            inside = false;
            i += 2;
        }
        else {
            i += 1;
        }
    }
    inside
}

fn strip_marker(line: &str) -> (String, Option<u32>) {
    if let Some(marker_start) = line.rfind(MARKER_PREFIX) {
        let after = &line[marker_start + MARKER_PREFIX.len()..];
        if let Some(end) = after.find(MARKER_SUFFIX) {
            if let Ok(n) = after[..end].parse::<u32>() {
                // Remove the marker (and any whitespace before it).
                let clean = line[..marker_start].trim_end().to_string();
                return (clean, Some(n));
            }
        }
    }
    (line.to_string(), None)
}
