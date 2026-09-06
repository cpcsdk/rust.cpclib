# VSCode Extension CI/CD and Publishing Guide

## Overview

The VSCode extension (`cpclib-vscode`) has been set up with comprehensive CI/CD automation and documentation. This guide explains the binary distribution strategy and publishing workflow.

## Binary Distribution Strategy: **Embedded Platform-Specific Binaries**

The chosen approach is to **embed all three platform binaries** (Linux, Windows, macOS) in a single extension package. This provides the best user experience:

### ✅ Advantages
- **Zero user setup**: Works immediately after install - no manual binary installation required
- **Version consistency**: Extension and LSP server versions are always in sync
- **Reliable**: No network downloads, no PATH issues, no cargo install required
- **Fallback support**: Still works if users have their own `cpclib-lsp` in PATH

### 📦 How It Works

1. **Build Phase** (GitHub Actions):
   - Three parallel jobs build `cpclib-lsp` for Linux, Windows, and macOS
   - Each job uploads its platform-specific binary as an artifact

2. **Package Phase**:
   - Downloads all three binaries into `bin/linux/`, `bin/windows/`, `bin/macos/`
   - Compiles TypeScript extension code
   - Packages everything into a single `.vsix` file with `vsce package`

3. **Runtime** (in VS Code):
   - `resolveServerPath()` first checks `extension/bin/<platform>/cpclib-lsp[.exe]`
   - Falls back to `$PATH` if user has a custom install
   - Falls back to `~/.cargo/bin` for development setups
   - Reports clear error if binary not found anywhere

### 📊 Package Size Impact

Typical binary sizes (release build, stripped):
- Linux: ~8-12 MB
- Windows: ~6-10 MB  
- macOS: ~8-12 MB
- **Total: ~25-35 MB** for the complete extension

This is acceptable for modern VSCode extensions (e.g., Rust Analyzer is ~40 MB).

## Files Modified

### 1. **README.md** (NEW)
Comprehensive documentation covering:
- Feature overview (assembly, bndbuild, BASIC support)
- LSP capabilities (hover, completion, diagnostics, etc.)
- Installation and configuration
- Troubleshooting
- **Explicit statement**: "This extension is completely biased toward the Benediction toolchain and focuses on basm assembler and bndbuild project management tool."

### 2. **.github/workflows/build_vscode_extension.yml** (NEW)
Three-job workflow:
- `build-lsp-binaries`: Cross-platform binary builds (Linux/Windows/macOS)
- `package-extension`: Downloads binaries, compiles TS, creates .vsix
- `publish-extension`: Publishes to marketplace on version tags

### 3. **src/extension.ts** (MODIFIED)
Updated `resolveServerPath()`:
- Accepts `extensionPath` parameter
- Checks bundled binaries first: `bin/<platform>/cpclib-lsp[.exe]`
- Falls back to PATH and ~/.cargo/bin
- Platform detection: `linux`, `macos`, `windows`

### 4. **.vscodeignore** (MODIFIED)
Added explicit comment documenting that `bin/` must be packaged (NOT ignored).

### 5. **package.json** (MODIFIED)
Removed unused `cpclib-lsp.bndbuildPath` setting - the LSP server uses bndbuild as a library, not an external binary.

## Publishing Workflow

### Initial Setup (One-Time)

1. **Create a VSCode Marketplace Publisher Account**:
   - Go to https://marketplace.visualstudio.com/manage
   - Sign in with Microsoft account
   - Create a publisher (or use existing "krusty-benediction" from package.json)

2. **Set up Azure AD Authentication** (required for automated publishing):
   
   **⚠️ Note**: Personal Access Tokens (PAT) are deprecated. Modern automated publishing requires Azure AD service principal or managed identity.
   
   **Option A: Azure AD Service Principal** (recommended for GitHub Actions):
   ```bash
   # Create an Azure AD app registration:
   # 1. Go to Azure Portal → Azure Active Directory → App registrations → New registration
   # 2. Name: "VSCode Extension Publisher" 
   # 3. Copy the Application (client) ID and Tenant ID
   # 4. Certificates & secrets → New client secret → Copy the secret value
   # 5. Grant the app permission to publish extensions in Marketplace (see Microsoft docs)
   ```
   - Store as GitHub secrets:
     - `AZURE_CLIENT_ID` (Application/client ID)
     - `AZURE_TENANT_ID` (Directory/tenant ID)  
     - `AZURE_CLIENT_SECRET` (Client secret value)
   
   **Option B: Legacy PAT** (deprecated, may stop working):
   - Azure DevOps → User Settings → Personal Access Tokens
   - Scope: Marketplace (Manage)
   - Store as GitHub secret: `VSCE_PAT`
   - See: https://code.visualstudio.com/api/working-with-extensions/publishing-extension#secure-automated-publishing-to-visual-studio-marketplace

3. **(Optional) Open VSX Registry Token**:
   - For VS Codium and open-source marketplaces
   - Get token from https://open-vsx.org/
   - Store as GitHub secret: `OVSX_PAT`

### Automated Publishing (Recommended)

Trigger a release by pushing a version tag:

```bash
# Update version in package.json
cd cpclib-vscode
npm version patch  # or minor, major

# Push tag to GitHub
git push origin v0.0.2  # triggers CI/CD
```

The workflow will:
1. Build LSP binaries for all platforms
2. Package the extension
3. Publish to VS Code Marketplace (if `VSCE_PAT` is set)
4. Publish to Open VSX (if `OVSX_PAT` is set)
5. Create GitHub Release with `.vsix` attachment

### Manual Publishing

For testing or one-off publishes:

```bash
# Install vsce if not already installed
npm install -g @vscode/vsce

# Build binaries first (or download from GitHub Actions artifacts)
# Place them in:
#   bin/linux/cpclib-lsp
#   bin/windows/cpclib-lsp.exe
#   bin/macos/cpclib-lsp

# Package extension
cd cpclib-vscode
npm install
npm run compile
vsce package

# Publish to marketplace
vsce publish

# Or install locally for testing
code --install-extension cpclib-vscode-0.0.1.vsix
```

## Testing the Extension

### Local Development
```bash
# Build LSP server
cargo build --release -p cpclib-lsp

# Copy to extension bin directory
mkdir -p cpclib-vscode/bin/linux
cp target/release/cpclib-lsp cpclib-vscode/bin/linux/

# Open extension in VS Code
cd cpclib-vscode
code .

# Press F5 to launch Extension Development Host
```

### Testing Packaged Extension
```bash
# Package with local binaries
vsce package

# Install in your main VS Code
code --install-extension cpclib-vscode-0.0.1.vsix

# Test with a real .asm file
```

## Continuous Updates

When you want to release a new version:

1. **Make changes** to extension or LSP server
2. **Update version** in `cpclib-vscode/package.json`
3. **Update CHANGELOG** (recommended)
4. **Push a tag**:
   ```bash
   git tag v0.0.2
   git push origin v0.0.2
   ```
5. **GitHub Actions** handles the rest automatically

## Troubleshooting CI

### Build Fails on macOS/Windows
- Check Rust toolchain compatibility (currently using `nightly`)
- Verify all crates compile on all platforms: `cargo check --workspace`

### Binary Not Found in Extension
- Check GitHub Actions artifacts were downloaded correctly
- Verify bin/ paths: `bin/linux/`, `bin/windows/`, `bin/macos/`
- Ensure `.vscodeignore` doesn't exclude `bin/`

### Extension Won't Publish
- **Using Azure AD**: Verify `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, and `AZURE_CLIENT_SECRET` secrets are set correctly
- **Using legacy PAT**: Verify `VSCE_PAT` secret is set (but note PATs are deprecated)
- Check publisher name matches `package.json` (should be your marketplace publisher ID)
- Ensure version number is unique (can't republish same version)
- Verify the service principal has permission to publish for your publisher account

### Package Size Too Large
- GitHub has a 2GB artifact limit (we're nowhere near this)
- Marketplace has ~100MB soft limit, ~1GB hard limit
- If needed, use separate platform-specific extensions (more complex)

## Alternative Approaches (Not Chosen)

### ❌ Manual Install (User runs `cargo install`)
- Poor UX: requires Rust toolchain
- Version mismatches between extension and binary
- PATH configuration issues

### ❌ Download on Demand (Extension fetches binary at runtime)
- Requires hosting binaries somewhere
- Network dependency, potential failures
- Security concerns (signature verification needed)
- Still need to package binaries as fallback

### ❌ Separate Platform-Specific Extensions
- More complex CI/CD (three separate packages)
- Marketplace shows three separate entries (confusing)
- More maintenance overhead
- Only needed if package size becomes an issue (not currently)

## Next Steps

1. ✅ **Configure GitHub Secrets**:
   
   **Modern approach (Azure AD service principal)**:
   - Add `AZURE_CLIENT_ID`, `AZURE_TENANT_ID`, `AZURE_CLIENT_SECRET` for VS Code Marketplace
   - See "Initial Setup" section above for creating the service principal
   
   **Legacy approach (PAT - deprecated)**:
   - Add `VSCE_PAT` for VS Code Marketplace (will work as fallback if Azure AD not configured)
   
   **Optional**:
   - Add `OVSX_PAT` for Open VSX Registry

2. ✅ **Test Locally**:
   - Build binaries for your platform
   - Test extension in development mode (F5)
   - Package and install .vsix locally: `vsce package && code --install-extension *.vsix`

3. ✅ **Push First Tag**:
   - Update version in package.json: `npm version patch`
   - Push the tag: `git push origin v0.0.2`
   - Verify workflow completes successfully in GitHub Actions
   - Check artifacts are created
   - Test the published extension from marketplace

4. ✅ **Iterate**:
   - Monitor user feedback
   - Fix bugs and add features
   - Release updates via tags

## Documentation Links

- VSCode Extension Publishing: https://code.visualstudio.com/api/working-with-extensions/publishing-extension
- vsce CLI: https://github.com/microsoft/vscode-vsce
- Open VSX: https://github.com/eclipse/openvsx/wiki/Publishing-Extensions
- GitHub Actions: https://docs.github.com/en/actions
