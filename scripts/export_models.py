"""Export RF-DETR-Seg Medium to ONNX and place it in the model directory.

Usage: python scripts/export_models.py [model_dir]   (default: models)
Requires: pip install rfdetr onnx
"""

import shutil
import sys
import tempfile
from pathlib import Path

from rfdetr import RFDETRSegMedium

TARGET_NAME = "rfdetr-seg-medium.onnx"


def main() -> None:
    model_dir = Path(sys.argv[1] if len(sys.argv) > 1 else "models")
    model_dir.mkdir(parents=True, exist_ok=True)
    target = model_dir / TARGET_NAME
    if target.exists():
        print(f"{target} already exists; delete it to export again.")
        return

    with tempfile.TemporaryDirectory() as scratch:
        exported = RFDETRSegMedium().export(output_dir=scratch, verbose=False)
        shutil.move(str(exported), target)
    print(f"Wrote {target} ({target.stat().st_size / 1e6:.0f} MB)")


if __name__ == "__main__":
    main()
