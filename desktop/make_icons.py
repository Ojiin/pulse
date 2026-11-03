"""The app icon of Pulse: renders assets/app-icon.svg with Chrome to
src-tauri/icons/source.png (1024 px, transparent corners); `npm run icons`
then derives the bundle's icon set from it.

Usage: python3 make_icons.py
"""

import subprocess
from pathlib import Path

HERE = Path(__file__).parent
CHROME = "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"
OUT = HERE / "src-tauri/icons/source.png"

subprocess.run([CHROME, "--headless=new", "--disable-gpu", "--hide-scrollbars", "--force-device-scale-factor=1",
                "--default-background-color=00000000", "--window-size=1024,1024", f"--screenshot={OUT}",
                (HERE / "assets/app-icon.svg").as_uri()], check=True, capture_output=True)
print(OUT)
