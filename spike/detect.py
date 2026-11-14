import json
import time
import warnings

warnings.filterwarnings("ignore")
import numpy as np
import supervision as sv
from PIL import Image
from rfdetr import RFDETRMedium, RFDETRSegMedium
from rfdetr.assets.coco_classes import COCO_CLASSES
from ultralytics import YOLO

NAMES = ["bottle", "couch", "person_cup", "burger", "person_pose"]
THRESH = 0.35
results = {}

def summarize(det, class_names):
    return [
        {"label": class_names.get(int(c), str(int(c))), "conf": round(float(s), 2), "box": [round(float(v)) for v in b]}
        for b, c, s in zip(det.xyxy, det.class_id, det.confidence)
    ]

rf = RFDETRMedium()
rfseg = RFDETRSegMedium()
yolo = YOLO("yolo11m-seg.pt")

for name in NAMES:
    img = Image.open(f"images/{name}.jpg").convert("RGB")
    entry = {}

    rf.predict(img, threshold=THRESH)
    t = time.perf_counter()
    det = rf.predict(img, threshold=THRESH)
    entry["rfdetr_m"] = {"ms": round((time.perf_counter() - t) * 1000), "dets": summarize(det, COCO_CLASSES)}

    rfseg.predict(img, threshold=THRESH)
    t = time.perf_counter()
    det = rfseg.predict(img, threshold=THRESH)
    entry["rfdetr_seg_m"] = {
        "ms": round((time.perf_counter() - t) * 1000),
        "dets": summarize(det, COCO_CLASSES),
        "has_masks": det.mask is not None,
    }
    if det.mask is not None:
        ann = sv.MaskAnnotator().annotate(img.copy(), det)
        ann = sv.BoxAnnotator().annotate(ann, det)
        Image.fromarray(np.asarray(ann)).save(f"out/det_rfseg_{name}.jpg")

    t = time.perf_counter()
    r = yolo.predict(img, conf=THRESH, verbose=False)[0]
    entry["yolo11m_seg"] = {
        "ms": round((time.perf_counter() - t) * 1000),
        "dets": [
            {"label": r.names[int(c)], "conf": round(float(s), 2), "box": [round(float(v)) for v in b]}
            for b, c, s in zip(r.boxes.xyxy, r.boxes.cls, r.boxes.conf)
        ],
    }
    Image.fromarray(r.plot()[..., ::-1]).save(f"out/det_yolo_{name}.jpg")
    results[name] = entry
    print(name, json.dumps(entry)[:1200], flush=True)

json.dump(results, open("out/detections.json", "w"), indent=2)
