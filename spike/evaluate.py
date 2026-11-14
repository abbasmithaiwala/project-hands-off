import json
import warnings
from pathlib import Path

warnings.filterwarnings("ignore")
import numpy as np
from PIL import Image
from rfdetr import RFDETRSegMedium
from rfdetr.assets.coco_classes import COCO_CLASSES

meta = json.load(open("out/scen/meta.json"))
MODELS = ["google/gemini-3.1-flash-image", "google/gemini-3-pro-image", "openai/gpt-5-image-mini"]
VARIANTS = ["text_overlay", "preview_ref"]
model = RFDETRSegMedium()


def iou(a, b):
    ix0, iy0, ix1, iy1 = max(a[0], b[0]), max(a[1], b[1]), min(a[2], b[2]), min(a[3], b[3])
    inter = max(0, ix1 - ix0) * max(0, iy1 - iy0)
    ua = (a[2] - a[0]) * (a[3] - a[1]) + (b[2] - b[0]) * (b[3] - b[1]) - inter
    return inter / ua if ua else 0.0


rows = []
for sid, m in meta.items():
    W, H = m["size"]
    orig = Image.open(m["original"]).convert("RGB")
    oa = np.asarray(orig).astype(np.float32)
    sx0, sy0, sx1, sy1 = m["source_box"]
    tx0, ty0, tx1, ty1 = m["target_box"]
    ex0, ey0, ex1, ey1 = min(sx0, tx0), min(sy0, ty0), max(sx1, tx1), max(sy1, ty1)
    pad = int(0.05 * max(W, H))
    keep = np.ones((H, W), bool)
    keep[max(ey0 - pad, 0):ey1 + pad, max(ex0 - pad, 0):ex1 + pad] = False
    for mdl in MODELS:
        for var in VARIANTS:
            f = Path(f"out/gen/{mdl.replace('/', '_')}__{var}__{sid}.png")
            if not f.exists():
                rows.append((sid, mdl, var, None)); continue
            out = Image.open(f).convert("RGB")
            ar_out, ar_orig = out.width / out.height, W / H
            res = out.resize((W, H), Image.LANCZOS)
            diff = float(np.abs(np.asarray(res).astype(np.float32) - oa)[keep].mean())
            det = model.predict(res, threshold=0.25)
            cands = [i for i, c in enumerate(det.class_id) if COCO_CLASSES[int(c)] == m["label"]]
            if cands:
                best = max(cands, key=lambda i: iou([float(v) for v in det.xyxy[i]], m["target_box"]))
                b = [float(v) for v in det.xyxy[best]]
                i_t, i_s = iou(b, m["target_box"]), iou(b, m["source_box"])
            else:
                i_t = i_s = None
            meta_f = json.load(open(str(f)[:-4] + ".json"))
            rows.append((sid, mdl, var, dict(ar_drift=round(abs(ar_out / ar_orig - 1), 3), bg_diff=round(diff, 1),
                         iou_target=None if i_t is None else round(i_t, 2), iou_source=None if i_s is None else round(i_s, 2),
                         sec=meta_f["seconds"], cost=round(meta_f["cost"], 3))))
json.dump(rows, open("out/eval.json", "w"), indent=1)
print(f"{'scenario':16s} {'model':32s} {'variant':13s} ar_drift bg_diff iou_tgt iou_src  sec  cost")
for sid, mdl, var, r in rows:
    if r is None:
        print(sid, mdl, var, "MISSING"); continue
    print(f"{sid:16s} {mdl:32s} {var:13s} {r['ar_drift']:7} {r['bg_diff']:7} {str(r['iou_target']):7} {str(r['iou_source']):7} {r['sec']:5} {r['cost']}")
