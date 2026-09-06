# Publishing the CPClib Zed Extension

This guide explains how to publish and update the CPClib extension for Zed.

## Overview

Zed extensions are distributed differently than VS Code extensions:
- **No central marketplace binary distribution** — Extensions are Rust code compiled on the user's machine
- **GitHub-based** — Extensions are referenced by their Git repository URL
- **Listed in registry** — The [Zed Extensions Repository](https://github.com/zed-industries/extensions) contains a registry of approved extensions
- **Automatic updates** — Zed checks for updates automatically when users restart

---

## Prerequisites

1. **Zed extension API knowledge:**
   - Read: https://zed.dev/docs/extensions/developing-extensions
   - Understand the `extension.toml` format
   - Familiarity with `zed_extension_api` crate

2. **Repository structure:**
   - Extension code must be in a dedicated directory (e.g., `cpclib-lsp-zed/`)
   - Must contain: `extension.toml`, `Cargo.toml`, `src/lib.rs`
   - Optional: language configs in `languages/`, snippets in `snippets/`

3. **cpclib-lsp binary distribution:**
   - Users must install `cpclib-lsp` separately (via `cargo install` or downloading binaries)
   - Extension detects the binary automatically from `~/.cargo/bin` or `$PATH`

---

## Publishing Process

### 1. Prepare the Extension

Ensure all required files are up to date:

```bash
cd cpclib-lsp-zed/

# Verify extension.toml metadata
cat extension.toml
# Check: id, name, version, description, repository

# Verify Cargo.toml
cat Cargo.toml
# Check: version matches extension.toml

# Test locally
cargo build --release
```

### 2. Version Numbering

Update version in **both** files:

**extension.toml:**
```toml
version = "0.1.0"  # Use semantic versioning
```

**Cargo.toml:**
```toml
version = "0.1.0"  # Must match extension.toml
```

Version format: `MAJOR.MINOR.PATCH`
- **MAJOR:** Breaking changes (e.g., requires new cpclib-lsp version)
- **MINOR:** New features (e.g., added language support)
- **PATCH:** Bug fixes

### 3. Tag and Push

```bash
# Commit all changes
git add .
git commit -m "Release cpclib-lsp-zed v0.1.0"

# Create a git tag (optional but recommended)
git tag zed-v0.1.0
git push origin main --tags
```

### 4. Submit to Zed Extensions Registry

To make your extension discoverable in Zed's extension list:

1. **Fork the Zed extensions repository:**
   ```bash
   git clone https://github.com/zed-industries/extensions.git
   cd extensions
   ```

2. **Add your extension to the registry:**
   
   Create a new directory: `extensions/cpclib-lsp-zed/`
   
   Add a `extension.toml` pointing to your repository:
   ```toml
   id = "cpclib-lsp-zed"
   name = "CPClib - Amstrad CPC Development"
   version = "0.1.0"
   schema_version = 1
   authors = ["Krusty Benediction <krusty.benediction@gmail.com>"]
   description = "Language support for Amstrad CPC development: Z80 assembly (basm), bndbuild, Locomotive BASIC"
   repository = "https://github.com/cpcsdk/rust.cpclib"
   
   # Path to extension within the repository
   path = "cpclib-lsp-zed"
   ```

3. **Submit a pull request:**
   ```bash
   git checkout -b add-cpclib-extension
   git add extensions/cpclib-lsp-zed/
   git commit -m "Add CPClib extension for Amstrad CPC development"
   git push origin add-cpclib-extension
   ```

4. **Create PR on GitHub:**
   - Go to https://github.com/zed-industries/extensions
   - Click "New Pull Request"
   - Provide description of the extension
   - Wait for review by Zed maintainers

---

## Testing Before Publishing

### Local Installation Test

Users can test your extension before it's in the registry:

1. **Via Git URL:**
   ```bash
   # Users can install directly from GitHub
   zed: extensions
   # Then manually add your repo URL
   ```

2. **Manual installation:**
   ```bash
   # Clone and link locally
   cd ~/.config/zed/extensions/
   ln -s /path/to/rust.cpclib/cpclib-lsp-zed cpclib-lsp-zed
   ```

3. **Test in Zed:**
   - Open a `.asm` file
   - Verify LSP features activate
   - Check language server logs for errors

### Automated Checks

Run these before every release:

```bash
# Verify extension compiles
cd cpclib-lsp-zed
cargo check
cargo clippy

# Test that cpclib-lsp can be found
which cpclib-lsp || echo "Warning: cpclib-lsp not in PATH"

# Verify extension.toml syntax
cat extension.toml | grep -E "^(id|name|version|schema_version|repository)"
```

---

## Update Process

When releasing a new version:

1. **Update version numbers** in both `extension.toml` and `Cargo.toml`
2. **Update CHANGELOG** (if you maintain one)
3. **Commit and tag:**
   ```bash
   git commit -am "Bump version to 0.2.0"
   git tag zed-v0.2.0
   git push origin main --tags
   ```
4. **Notify users** (optional):
   - GitHub release notes
   - Update README with new features

Zed will automatically detect updates when users restart.

---

## Troubleshooting

### Extension doesn't appear in Zed

**Possible causes:**
- Extension not yet approved in Zed registry
- Schema version incompatibility
- Build errors in extension code

**Solution:**
- Check Zed's extension logs
- Verify `extension.toml` schema version matches your Zed version
- Test local installation first

### Users report "LSP server not found"

**Cause:** `cpclib-lsp` binary not installed

**Solution:**
- Update README with clear installation instructions
- Provide pre-built binaries for all platforms
- Consider adding a check in the extension that shows a helpful error

### Build fails on user's machine

**Cause:** Missing Rust toolchain or dependencies

**Solution:**
- Document system requirements in README
- Test on clean systems (Linux, macOS, Windows)
- Check `zed_extension_api` version compatibility

---

## Distribution Channels

### Official Channels

1. **Zed Extensions Registry** (recommended)
   - Listed in Zed's built-in extension browser
   - Automatic updates
   - Vetted by Zed team

2. **GitHub Releases**
   - Not applicable for Zed (extensions are source-distributed)
   - But you can publish `cpclib-lsp` binaries as GitHub releases

### Alternative Distribution

Users can always install directly from your repository:
```bash
# Direct installation from Git
zed: extensions → Install from Git
# Enter: https://github.com/cpcsdk/rust.cpclib
# Path: cpclib-lsp-zed
```

---

## Checklist

Before publishing a new version:

- [ ] Version bumped in `extension.toml`
- [ ] Version bumped in `Cargo.toml` (must match)
- [ ] `cargo check` passes
- [ ] `cargo clippy` shows no warnings
- [ ] README updated with new features
- [ ] Language configs verified for all 4 languages
- [ ] Snippets tested in Zed
- [ ] LSP server path resolution tested
- [ ] Committed and tagged in Git
- [ ] PR submitted to Zed extensions registry (if first release)

---

## Support and Maintenance

- **Issues:** https://github.com/cpcsdk/rust.cpclib/issues
- **Discussions:** GitHub Discussions or Zed community channels
- **Updates:** Monitor Zed's `zed_extension_api` for breaking changes

---

## References

- [Zed Extension Docs](https://zed.dev/docs/extensions/developing-extensions)
- [Zed Extension API](https://docs.rs/zed_extension_api/)
- [Zed Extensions Repository](https://github.com/zed-industries/extensions)
- [CPClib Documentation](https://cpcsdk.github.io/rust.cpclib/)
