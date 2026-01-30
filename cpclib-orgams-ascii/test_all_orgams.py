#!/usr/bin/env python3
import os
import subprocess
import sys
from pathlib import Path

def run_debug_load_orgams(file_path):
    """Run cargo run --example debug_load_orgams on the given file and return (success, output, error)"""
    cmd = ["cargo", "run", "--example", "debug_load_orgams", str(file_path)]
    try:
        result = subprocess.run(cmd, capture_output=True, text=True, timeout=30)
        success = result.returncode == 0
        return success, result.stdout, result.stderr
    except subprocess.TimeoutExpired:
        return False, "", "Timeout after 30 seconds"
    except Exception as e:
        return False, "", str(e)

def main():
    # Directory to scan
    base_dir = Path("/home/romain/Perso/CPC/rust.cpcdemotools/cpclib-orgams-ascii/tests/orgams-main")
    if not base_dir.exists():
        print(f"Directory {base_dir} does not exist!")
        sys.exit(1)

    # Find all .I and .O files recursively
    files_to_test = []
    for ext in ['.I', '.O']:
        files_to_test.extend(base_dir.rglob(f'*{ext}'))

    if not files_to_test:
        print("No .I or .O files found!")
        sys.exit(1)

    print(f"Found {len(files_to_test)} files to test:")
    for f in files_to_test:
        print(f"  {f}")

    # Run tests
    passed = []
    failed = []

    for file_path in files_to_test:
        print(f"\nTesting {file_path}...")
        success, stdout, stderr = run_debug_load_orgams(file_path)
        
        if success:
            print("  PASSED")
            passed.append(file_path)
        else:
            print("  FAILED")
            # Extract and print error information
            lines = stderr.splitlines()
            start_idx = None
            end_idx = None
            for i, line in enumerate(lines):
                if line.startswith("thread 'main'"):
                    start_idx = i
                if line.startswith("note: run with `RUST_BACKTRACE=1`"):
                    end_idx = i
                    break
            if start_idx is not None and end_idx is not None:
                error_lines = lines[start_idx:end_idx+1]
                for line in error_lines:
                    print(f"    {line}")
            else:
                print("    No specific error info found.")
            failed.append(file_path)

    # Final report
    print(f"\n{'='*50}")
    print("FINAL REPORT")
    print(f"{'='*50}")
    print(f"Total files tested: {len(files_to_test)}")
    print(f"Passed: {len(passed)}")
    print(f"Failed: {len(failed)}")

    if passed:
        print(f"\nPASSED FILES ({len(passed)}):")
        for f in passed:
            print(f"  ✓ {f}")

    if failed:
        print(f"\nFAILED FILES ({len(failed)}):")
        for f in failed:
            print(f"  ✗ {f}")

if __name__ == "__main__":
    main()