import shutil
import statistics
import time
import warnings
from pathlib import Path

warnings.filterwarnings("ignore")
import numpy as np
import onnxruntime as ort
from rfdetr import RFDETRMedium, RFDETRSegMedium
from ultralytics import YOLO

MODELS = Path("models")
MODELS.mkdir(exist_ok=True)


def export_all():
    for name, cls in [("rfdetr_m", RFDETRMedium), ("rfdetr_seg_m", RFDETRSegMedium)]:
        target = MODELS / f"{name}.onnx"
        if target.exists():
            continue
        tmp = MODELS / f"_{name}"
        path = cls().export(output_dir=str(tmp), verbose=False)
        shutil.move(str(path), target)
        shutil.rmtree(tmp, ignore_errors=True)
    target = MODELS / "yolo11m_seg.onnx"
    if not target.exists():
        out = YOLO("yolo11m-seg.pt").export(format="onnx", opset=17, simplify=False)
        shutil.move(out, target)


def bench(path, providers, runs=10):
    so = ort.SessionOptions()
    sess = ort.InferenceSession(str(path), so, providers=providers)
    inp = sess.get_inputs()[0]
    shape = [d if isinstance(d, int) else 1 for d in inp.shape]
    x = np.random.rand(*shape).astype(np.float32)
    sess.run(None, {inp.name: x})
    times = []
    for _ in range(runs):
        t = time.perf_counter()
        sess.run(None, {inp.name: x})
        times.append((time.perf_counter() - t) * 1000)
    return sess.get_providers()[0], shape, statistics.median(times), min(times)


export_all()
for p in sorted(MODELS.glob("*.onnx")):
    print(p.name, f"{p.stat().st_size / 1e6:.0f}MB")
    for providers in (["CPUExecutionProvider"], ["CoreMLExecutionProvider", "CPUExecutionProvider"]):
        try:
            prov, shape, med, best = bench(p, providers)
            print(f"  {providers[0]:26s} used={prov:24s} input={shape} median={med:.0f}ms best={best:.0f}ms", flush=True)
        except Exception as e:
            print(f"  {providers[0]:26s} FAILED: {str(e)[:200]}", flush=True)
