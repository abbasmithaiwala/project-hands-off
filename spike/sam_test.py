import json
import time
import warnings

warnings.filterwarnings("ignore")
import numpy as np
from PIL import Image
from ultralytics import SAM

det = json.load(open("out/detections.json"))
PICK = {"bottle": "vase", "couch": "couch", "person_cup": "person", "burger": "sandwich", "person_pose": "person"}
sam = SAM("sam2.1_t.pt")
for name, label in PICK.items():
    box = next(d["box"] for d in det[name]["rfdetr_seg_m"]["dets"] if d["label"] == label)
    img = Image.open(f"images/{name}.jpg").convert("RGB")
    sam.predict(img, bboxes=[box], verbose=False)
    t = time.perf_counter()
    r = sam.predict(img, bboxes=[box], verbose=False)[0]
    ms = round((time.perf_counter() - t) * 1000)
    m = r.masks.data[0].cpu().numpy() > 0.5
    over = np.asarray(img).copy()
    over[m] = (0.55 * over[m] + 0.45 * np.array([255, 120, 0])).astype(np.uint8)
    Image.fromarray(over).save(f"out/sam_{name}.jpg")
    print(name, label, f"{ms}ms", f"mask_area={m.mean():.2f}")
