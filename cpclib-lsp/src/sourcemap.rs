use minijinja::Environment;

const MARKER_PREFIX: &str = "#_SRCL:";
const MARKER_SUFFIX: &str = "_#";

/// Mapping from lines in the Jinja-expanded text back to lines in the original source.
/// Index is the expanded line number (0-based); value is the original line number (0-based),
/// or `None` for lines that were synthesised by Jinja with no direct source counterpart.
pub struct SourceMap {
    pub expanded_to_original: Vec<Option<u32>>,
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
            expanded_to_original: (0..line_count as u32).map(Some).collect(),
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
    file_dir: Option<&std::path::Path>,
) -> Result<(String, SourceMap), minijinja::Error> {
    // Step 1 — annotate every line with its original line number.
    let annotated = annotate(source);

    // Step 2 — render through minijinja.
    let mut env = Environment::new();
    if let Some(dir) = file_dir {
        let dir = dir.to_path_buf();
        env.set_loader(move |name| {
            let path = dir.join(name);
            match std::fs::read_to_string(&path) {
                Ok(s) => Ok(Some(s)),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                Err(e) => Err(minijinja::Error::new(
                    minijinja::ErrorKind::InvalidOperation,
                    "could not read include",
                )
                .with_source(e)),
            }
        });
    }

    // No variable context — we only need structural expansion for LSP purposes.
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
    } else {
        clean_text
    };

    Ok((
        clean_text,
        SourceMap {
            expanded_to_original: map,
        },
    ))
}

// ─── internal helpers ────────────────────────────────────────────────────────

fn annotate(source: &str) -> String {
    let mut out = String::with_capacity(source.len() + source.lines().count() * 12);
    for (i, line) in source.lines().enumerate() {
        // Strip any existing trailing whitespace so we can cleanly append.
        let trimmed_end = line.trim_end();
        // Append the marker as a YAML comment.  Jinja passes comments through,
        // so the marker survives template expansion for non-control lines.
        out.push_str(trimmed_end);
        out.push(' ');
        out.push_str(MARKER_PREFIX);
        out.push_str(&i.to_string());
        out.push_str(MARKER_SUFFIX);
        out.push('\n');
    }
    out
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
