#!/usr/bin/env python3
"""Render the illustration SVGs to PNGs with headless Chrome, for checking by eye.

    python3 website/illustrations/render.py    # or: make site-render

Writes website/illustrations/renders/<name>.png (2x) and sheet.png (every scene
in a 3-column contact sheet). Look for props that don't meet the surface they
rest on, hands that miss what they hold, and anything poking past the card edge.
Set CHROME=/path/to/chrome if it isn't found.
"""

import os
import pathlib
import shutil
import subprocess
import sys
import tempfile

HERE = pathlib.Path(__file__).resolve().parent
SVGS = HERE.parent / "static" / "illustrations"
OUT = HERE / "renders"
W, H, SCALE, COLS = 480, 300, 2, 3

CANDIDATES = [
    "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome",
    "/Applications/Chromium.app/Contents/MacOS/Chromium",
    "google-chrome", "google-chrome-stable", "chromium", "chromium-browser",
]


def find_chrome():
    for c in ([os.environ["CHROME"]] if os.environ.get("CHROME") else []) + CANDIDATES:
        found = c if os.path.isfile(c) else shutil.which(c)
        if found:
            return found
    sys.exit("render.py: no Chrome/Chromium found; set CHROME=/path/to/chrome")


def screenshot(chrome, html, png, width, height, tmp):
    page = tmp / (png.stem + ".html")
    page.write_text(f"<html><body style='margin:0;overflow:hidden'>{html}</body></html>")
    subprocess.run([chrome, "--headless=new", "--disable-gpu", "--hide-scrollbars",
                    f"--window-size={width},{height}", f"--screenshot={png}", page.as_uri()],
                   check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)


def main():
    chrome = find_chrome()
    svgs = sorted(SVGS.glob("*.svg"))
    if not svgs:
        sys.exit(f"render.py: no SVGs in {SVGS}; run `make site-illustrations` first")
    OUT.mkdir(exist_ok=True)
    with tempfile.TemporaryDirectory() as t:
        tmp = pathlib.Path(t)
        for svg in svgs:
            img = f"<img src='{svg.as_uri()}' width={W * SCALE} height={H * SCALE}>"
            screenshot(chrome, img, OUT / f"{svg.stem}.png", W * SCALE, H * SCALE, tmp)
        rows = -(-len(svgs) // COLS)
        grid = "".join(f"<img src='{s.as_uri()}' width={W} height={H}>" for s in svgs)
        screenshot(chrome, f"<div style='display:grid;grid-template-columns:repeat({COLS},{W}px)'>{grid}</div>",
                   OUT / "sheet.png", W * COLS, H * rows, tmp)
    print(f"rendered {len(svgs)} illustrations and sheet.png to {OUT}")


if __name__ == "__main__":
    main()
