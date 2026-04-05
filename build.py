#!/usr/bin/env python3
"""Build script for im-select-ssh.nvim.

Builds the Rust server, publishes the C# client, and bundles the Lua plugin
into a dist/ directory.
"""

import argparse
import platform
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent
DIST = ROOT / "dist"
SERVER_PLUGIN = DIST / "server-plugin"


def run(cmd: list[str], cwd: Path) -> None:
    print(f"  > {' '.join(cmd)}")
    subprocess.run(cmd, cwd=cwd, check=True)


def check_tool(name: str) -> None:
    if shutil.which(name) is None:
        sys.exit(f"Error: '{name}' not found on PATH")


def build_server(profile: str) -> None:
    print("\n[1/3] Building Rust server...")
    check_tool("cargo")

    cargo_args = ["cargo", "build"]
    if profile == "release":
        cargo_args.append("--release")
    run(cargo_args, cwd=ROOT / "server")

    binary_name = "im-select-server"
    if platform.system() == "Windows":
        binary_name += ".exe"

    src = ROOT / "server" / "target" / profile / binary_name
    dst = SERVER_PLUGIN / "server"
    dst.mkdir(parents=True, exist_ok=True)
    shutil.copy2(src, dst / binary_name)
    print(f"  OK Copied to {dst / binary_name}")


def build_client(profile: str) -> None:
    print("\n[2/3] Publishing C# client...")
    check_tool("dotnet")

    config = "Release" if profile == "release" else "Debug"
    run(["dotnet", "publish", "-c", config], cwd=ROOT / "client")

    publish_dir = ROOT / "client" / "bin" / config / "net10.0" / "publish"
    dst = DIST / "client"
    if dst.exists():
        shutil.rmtree(dst)
    shutil.copytree(publish_dir, dst)
    print(f"  OK Copied to {dst}")


def bundle_lua() -> None:
    print("\n[3/3] Bundling Lua plugin...")
    for dirname in ("lua", "plugin"):
        src = ROOT / dirname
        dst = SERVER_PLUGIN / dirname
        if dst.exists():
            shutil.rmtree(dst)
        shutil.copytree(src, dst)
        print(f"  OK Copied {dirname}/ to {dst}")


def main() -> None:
    parser = argparse.ArgumentParser(description="Build im-select-ssh.nvim")
    parser.add_argument(
        "--debug", action="store_true",
        help="Build in debug mode (default: release)",
    )
    parser.add_argument("--skip-server", action="store_true", help="Skip Rust server build")
    parser.add_argument("--skip-client", action="store_true", help="Skip C# client publish")
    parser.add_argument("--skip-lua", action="store_true", help="Skip Lua plugin bundling")
    parser.add_argument("--clean", action="store_true", help="Remove dist/ before building")
    args = parser.parse_args()

    profile = "debug" if args.debug else "release"

    if args.clean and DIST.exists():
        shutil.rmtree(DIST)
        print("Cleaned dist/")

    DIST.mkdir(parents=True, exist_ok=True)

    if not args.skip_server:
        build_server(profile)
    if not args.skip_client:
        build_client(profile)
    if not args.skip_lua:
        bundle_lua()

    print(f"\nDone -- artifacts in {DIST}")


if __name__ == "__main__":
    main()
