#!/usr/bin/env python3
"""
Build script for cpclib-vscode extension.
Replaces the Makefile with a fully cross-platform Python solution.

Usage:
    python build_extension.py build              - Auto-detect OS and build for current platform
    python build_extension.py build-linux        - Build extension with Linux LSP binary
    python build_extension.py build-windows      - Build extension with Windows LSP binary
    python build_extension.py build-macosx       - Build extension with macOS LSP binary
    python build_extension.py package-all        - Build LSP for all platforms and package
    python build_extension.py clean              - Remove build artifacts
    python build_extension.py clean-bins         - Remove only platform binaries
    python build_extension.py install-deps       - Install npm dependencies
    python build_extension.py compile            - Compile TypeScript
"""

import argparse
import platform
import subprocess
import sys
from pathlib import Path
import shutil


class Color:
    """ANSI color codes for terminal output"""
    BLUE = '\033[94m'
    GREEN = '\033[92m'
    YELLOW = '\033[93m'
    RED = '\033[91m'
    BOLD = '\033[1m'
    END = '\033[0m'


class ExtensionBuilder:
    """Builder for cpclib-vscode extension"""

    # Platform target triples
    LINUX_TARGET = "x86_64-unknown-linux-gnu"
    WINDOWS_TARGET = "x86_64-pc-windows-msvc"
    MACOSX_TARGET = "x86_64-apple-darwin"

    # Binary names
    LINUX_BINARY = "cpclib-lsp"
    WINDOWS_BINARY = "cpclib-lsp.exe"
    MACOSX_BINARY = "cpclib-lsp"

    def __init__(self):
        # Determine paths
        self.project_root = Path(__file__).parent.resolve()
        self.workspace_root = self.project_root.parent
        self.bin_dir = self.project_root / "bin"
        self.out_dir = self.project_root / "out"
        
        # Platform-specific directories
        self.linux_bin_dir = self.bin_dir / "linux"
        self.windows_bin_dir = self.bin_dir / "windows"
        self.macosx_bin_dir = self.bin_dir / "macos"
        
        # Detect current OS
        self.detected_os = self._detect_os()
        
        # Check Node.js version
        self._check_node_version()

    def _detect_os(self) -> str:
        """Detect the current operating system"""
        system = platform.system().lower()
        if system == "linux":
            return "linux"
        elif system == "windows":
            return "windows"
        elif system == "darwin":
            return "macosx"
        else:
            self.error(f"Unsupported OS: {system}")
            sys.exit(1)

    def _check_node_version(self):
        """Check if Node.js version is compatible with @vscode/vsce"""
        try:
            result = subprocess.run(
                ["node", "--version"],
                capture_output=True,
                text=True,
                check=True
            )
            version_str = result.stdout.strip().lstrip('v')
            major_version = int(version_str.split('.')[0])
            
            if major_version < 20:
                self.error(f"Node.js v{version_str} is too old!")
                print(f"\n{Color.YELLOW}@vscode/vsce requires Node.js v20 or later.{Color.END}")
                print(f"\n{Color.BOLD}To upgrade Node.js on Ubuntu:{Color.END}")
                print(f"  1. Using NodeSource repository:")
                print(f"     curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -")
                print(f"     sudo apt-get install -y nodejs")
                print(f"\n  2. Using snap:")
                print(f"     sudo snap install node --classic --channel=20")
                print(f"\n  3. Using nvm (Node Version Manager):")
                print(f"     curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash")
                print(f"     nvm install 20")
                sys.exit(1)
                
        except (subprocess.CalledProcessError, FileNotFoundError, ValueError) as e:
            self.error(f"Could not check Node.js version: {e}")
            sys.exit(1)

    def info(self, message: str):
        """Print info message"""
        print(f"{Color.BLUE}===> {message}{Color.END}")

    def success(self, message: str):
        """Print success message"""
        print(f"{Color.GREEN}[OK] {message}{Color.END}")

    def error(self, message: str):
        """Print error message"""
        print(f"{Color.RED}[ERROR] {message}{Color.END}", file=sys.stderr)

    def run_command(self, cmd: list, cwd: Path = None, check: bool = True, shell: bool = False) -> subprocess.CompletedProcess:
        """Run a shell command"""
        cwd = cwd or self.project_root
        self.info(f"Running: {' '.join(cmd)}")
        
        # On Windows, use shell=True for npm/npx commands to find .cmd files
        if platform.system() == "Windows" and cmd[0] in ["npm", "npx"]:
            shell = True
        
        try:
            return subprocess.run(cmd, cwd=cwd, check=check, shell=shell)
        except subprocess.CalledProcessError as e:
            self.error(f"Command failed with exit code {e.returncode}")
            raise

    def build_lsp(self, target: str, binary_name: str, bin_output_dir: Path, native: bool = False):
        """
        Build the LSP binary using Cargo
        
        Args:
            target: Target triple (e.g., x86_64-pc-windows-msvc) or "release" for native
            binary_name: Name of the output binary (e.g., cpclib-lsp.exe)
            bin_output_dir: Directory to copy the binary to
            native: If True, build without explicit --target flag (native build)
        """
        self.info(f"Building cpclib-lsp for target {target}...")
        
        # Build cargo command
        cargo_cmd = ["cargo", "build", "--release", "-p", "cpclib-lsp"]
        if not native:
            cargo_cmd.extend(["--target", target])
        
        # Run cargo build
        self.run_command(cargo_cmd, cwd=self.workspace_root)
        
        # Determine source path
        if native:
            source_path = self.workspace_root / "target" / "release" / binary_name
        else:
            source_path = self.workspace_root / "target" / target / "release" / binary_name
        
        # Create output directory
        bin_output_dir.mkdir(parents=True, exist_ok=True)
        
        # Copy binary
        dest_path = bin_output_dir / binary_name
        self.info(f"Copying binary to {dest_path}...")
        shutil.copy2(source_path, dest_path)
        
        # Set executable permissions on Unix-like systems
        if binary_name != self.WINDOWS_BINARY and platform.system() != "Windows":
            dest_path.chmod(0o755)
        
        self.success(f"cpclib-lsp for {target} built successfully")

    def prepare_extension(self):
        """Install npm dependencies and compile TypeScript"""
        self.info("Installing npm dependencies...")
        self.run_command(["npm", "ci"], cwd=self.project_root)
        
        self.info("Compiling TypeScript...")
        self.run_command(["npm", "run", "compile"], cwd=self.project_root)
        
        self.success("Extension prepared successfully")

    def clean_bins(self):
        """Remove platform binaries"""
        self.info("Cleaning platform binaries...")
        if self.bin_dir.exists():
            shutil.rmtree(self.bin_dir)
            self.info(f"Removed {self.bin_dir}")

    def clean_all(self):
        """Remove all build artifacts"""
        self.clean_bins()
        
        self.info("Cleaning build artifacts...")
        
        # Remove out directory
        if self.out_dir.exists():
            shutil.rmtree(self.out_dir)
            self.info(f"Removed {self.out_dir}")
        
        # Remove node_modules
        node_modules = self.project_root / "node_modules"
        if node_modules.exists():
            shutil.rmtree(node_modules)
            self.info(f"Removed {node_modules}")
        
        # Remove .vsix files
        for vsix_file in self.project_root.glob("*.vsix"):
            vsix_file.unlink()
            self.info(f"Removed {vsix_file}")
        
        self.success("Clean complete!")

    def rename_vsix_with_platform(self, platform_suffix: str):
        """Rename the generated .vsix file to include platform suffix"""
        # Find the generated .vsix file
        vsix_files = list(self.project_root.glob("*.vsix"))
        if not vsix_files:
            self.error("No .vsix file found to rename")
            return None
        
        original_vsix = vsix_files[0]
        # Extract name and version from original filename
        # Expected format: cpclib-vscode-0.0.1.vsix
        name_parts = original_vsix.stem.rsplit('-', 1)  # Split from the right to get version
        if len(name_parts) == 2:
            base_name, version = name_parts
            new_name = f"{base_name}-{platform_suffix}-{version}.vsix"
        else:
            # Fallback if format is unexpected
            new_name = f"{original_vsix.stem}-{platform_suffix}.vsix"
        
        new_path = self.project_root / new_name
        original_vsix.rename(new_path)
        return new_path

    def build_for_platform(self, platform_name: str):
        """Build extension for a specific platform"""
        self.clean_bins()
        
        # Determine if this is a native build
        native_build = (self.detected_os == platform_name)
        
        if platform_name == "linux":
            target = self.LINUX_TARGET
            binary = self.LINUX_BINARY
            bin_dir = self.linux_bin_dir
        elif platform_name == "windows":
            target = self.WINDOWS_TARGET
            binary = self.WINDOWS_BINARY
            bin_dir = self.windows_bin_dir
        elif platform_name == "macosx":
            target = self.MACOSX_TARGET
            binary = self.MACOSX_BINARY
            bin_dir = self.macosx_bin_dir
        else:
            self.error(f"Unknown platform: {platform_name}")
            sys.exit(1)
        
        # Build LSP binary
        self.build_lsp(target, binary, bin_dir, native=native_build)
        
        # Prepare extension
        self.prepare_extension()
        
        # Package extension
        self.info("Packaging extension...")
        self.run_command(["npx", "@vscode/vsce", "package", "--allow-missing-repository"], cwd=self.project_root)
        
        # Rename with platform suffix
        renamed_vsix = self.rename_vsix_with_platform(platform_name)
        
        # Success message
        self.success(f"{platform_name.capitalize()} extension build complete!")
        print(f"  Binary location: {bin_dir / binary}")
        if renamed_vsix:
            print(f"  Extension package: {renamed_vsix}")

    def build_auto(self):
        """Auto-detect OS and build for current platform"""
        self.info(f"Detected OS: {self.detected_os}")
        self.build_for_platform(self.detected_os)

    def package_all(self):
        """Build LSP for all platforms and package extension"""
        self.clean_bins()
        
        self.info("Building cpclib-lsp for all platforms...")
        
        # Build for all platforms (always cross-compile for package-all)
        self.build_lsp(self.LINUX_TARGET, self.LINUX_BINARY, self.linux_bin_dir, native=False)
        self.build_lsp(self.WINDOWS_TARGET, self.WINDOWS_BINARY, self.windows_bin_dir, native=False)
        self.build_lsp(self.MACOSX_TARGET, self.MACOSX_BINARY, self.macosx_bin_dir, native=False)
        
        # Prepare extension
        self.prepare_extension()
        
        # Package extension
        self.info("Packaging extension with all platform binaries...")
        self.run_command(["npx", "@vscode/vsce", "package", "--allow-missing-repository"], cwd=self.project_root)
        
        # Rename with 'all' suffix
        renamed_vsix = self.rename_vsix_with_platform("all")
        
        self.success("Multi-platform extension package complete!")
        if renamed_vsix:
            print(f"  Package: {renamed_vsix}")

    def install_deps(self):
        """Install npm dependencies"""
        self.info("Installing npm dependencies...")
        self.run_command(["npm", "ci"], cwd=self.project_root)
        self.success("Dependencies installed")

    def compile_typescript(self):
        """Compile TypeScript"""
        self.info("Compiling TypeScript...")
        self.run_command(["npm", "run", "compile"], cwd=self.project_root)
        self.success("TypeScript compiled")

    def watch_typescript(self):
        """Start TypeScript watch mode"""
        self.info("Starting TypeScript watch mode...")
        self.run_command(["npm", "run", "watch"], cwd=self.project_root)

    def show_help(self):
        """Show help information"""
        print(f"{Color.BOLD}CPClib VSCode Extension Build System{Color.END}")
        print("\nAvailable commands:")
        print("  build              - Auto-detect OS and build for current platform")
        print("  build-linux        - Build extension with Linux LSP binary")
        print("  build-windows      - Build extension with Windows LSP binary")
        print("  build-macosx       - Build extension with macOS LSP binary")
        print("  package-all        - Build LSP for all platforms and package extension")
        print("  clean              - Remove build artifacts")
        print("  clean-bins         - Remove only platform binaries")
        print("  install-deps       - Install npm dependencies")
        print("  compile            - Compile TypeScript")
        print("  watch              - Start TypeScript watch mode")
        print(f"\nDetected OS: {Color.BOLD}{self.detected_os}{Color.END}")


def main():
    parser = argparse.ArgumentParser(
        description="Build script for cpclib-vscode extension",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__
    )
    parser.add_argument(
        "command",
        nargs="?",
        default="help",
        choices=[
            "build", "build-linux", "build-windows", "build-macosx",
            "package-all", "clean", "clean-bins",
            "install-deps", "compile", "watch", "help"
        ],
        help="Command to execute"
    )
    
    args = parser.parse_args()
    builder = ExtensionBuilder()
    
    try:
        if args.command == "help":
            builder.show_help()
        elif args.command == "build":
            builder.build_auto()
        elif args.command == "build-linux":
            builder.build_for_platform("linux")
        elif args.command == "build-windows":
            builder.build_for_platform("windows")
        elif args.command == "build-macosx":
            builder.build_for_platform("macosx")
        elif args.command == "package-all":
            builder.package_all()
        elif args.command == "clean":
            builder.clean_all()
        elif args.command == "clean-bins":
            builder.clean_bins()
        elif args.command == "install-deps":
            builder.install_deps()
        elif args.command == "compile":
            builder.compile_typescript()
        elif args.command == "watch":
            builder.watch_typescript()
    except KeyboardInterrupt:
        print("\n\nInterrupted by user")
        sys.exit(130)
    except Exception as e:
        builder.error(str(e))
        sys.exit(1)


if __name__ == "__main__":
    main()
