#!/usr/bin/env python3
"""
Copy the wasm-bindgen output together with assets and index.html into
web-out/, then optionally zip everything for upload to itch.io.

Usage:
    python scripts/package_wasm.py          # populate web-out/
    python scripts/package_wasm.py --zip    # populate web-out/ and zip it
"""

import argparse
import shutil
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "web-out"
ZIP = ROOT / "robo_cleanup_game_web.zip"


def package() -> None:
    OUT.mkdir(exist_ok=True)

    # Copy the assets directory (overwrite previous run if present)
    assets_src = ROOT / "assets"
    assets_dst = OUT / "assets"
    if assets_src.exists():
        if assets_dst.exists():
            shutil.rmtree(assets_dst)
        shutil.copytree(assets_src, assets_dst)
        print(f"  copied  assets/  →  web-out/assets/")

    # Copy index.html
    html_src = ROOT / "index.html"
    if html_src.exists():
        shutil.copy2(html_src, OUT / "index.html")
        print(f"  copied  index.html  →  web-out/index.html")

    print(f"Ready:  {OUT}/")


def zip_output() -> None:
    if ZIP.exists():
        ZIP.unlink()

    with zipfile.ZipFile(ZIP, "w", compression=zipfile.ZIP_DEFLATED) as zf:
        for f in sorted(OUT.rglob("*")):
            if f.is_file():
                zf.write(f, f.relative_to(OUT))

    size_mb = ZIP.stat().st_size / 1_048_576
    print(f"Zipped: {ZIP.name}  ({size_mb:.1f} MB)")
    print("Upload to itch.io → choose 'This file is for HTML'.")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Package WASM build for web / itch.io")
    parser.add_argument("--zip", action="store_true", help="Also create a .zip archive")
    args = parser.parse_args()

    package()
    if args.zip:
        zip_output()
