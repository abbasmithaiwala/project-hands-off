import json
import warnings

warnings.filterwarnings("ignore")
import cv2
import numpy as np
from PIL import Image
from rfdetr import RFDETRSegMedium
from rfdetr.assets.coco_classes import COCO_CLASSES

SCENARIOS = {
    "couch_shorter": dict(image="couch", label="couch", warp=("scale_x", 0.55),
        instruction="Shorten the orange sofa to about 55% of its current length so it becomes a compact loveseat, keeping its left end where it is. Keep the sofa's depth, height, colour and style, and keep the rest of the room unchanged."),
    "bottle_diamond": dict(image="bottle", label="vase", warp=("diamond", 0.3),
        instruction="Reshape the left glass bottle so its body becomes a diamond shape: narrow at the neck, widest in the upper middle, tapering to a narrow pointed base. Keep realistic glass material, colour and reflections. Do not change the object on the right or the background."),
    "burger_shorter": dict(image="burger", label="sandwich", warp=("scale_y", 0.7),
        instruction="Make the burger about 30% shorter in height (a thinner stack) while keeping its width, ingredients and the plain white background."),
    "cup_bigger": dict(image="person_cup", label="bowl", warp=("scale", 1.6),
        instruction="Make the drinking glass the woman is holding about 1.6 times larger, with her hand still holding it naturally. Leave everything else unchanged."),
    "man_broader": dict(image="person_pose", label="person", warp=("scale_x", 1.25),
        instruction="Make the man's torso and flexed arms about 25% broader and more muscular. Keep his face, pose, the bicycle and the background unchanged."),
}

model = RFDETRSegMedium()


def warp_crop(crop_rgba, kind, amount):
    h, w = crop_rgba.shape[:2]
    if kind == "scale_x":
        nw = max(1, round(w * amount)); return cv2.resize(crop_rgba, (nw, h), interpolation=cv2.INTER_AREA if amount < 1 else cv2.INTER_CUBIC), "x"
    if kind == "scale_y":
        nh = max(1, round(h * amount)); return cv2.resize(crop_rgba, (w, nh), interpolation=cv2.INTER_AREA if amount < 1 else cv2.INTER_CUBIC), "bottom"
    if kind == "scale":
        return cv2.resize(crop_rgba, (round(w * amount), round(h * amount)), interpolation=cv2.INTER_CUBIC), "center"
    if kind == "diamond":
        out = np.zeros_like(crop_rgba)
        for y in range(h):
            t = y / max(h - 1, 1)
            s = (0.55 + 0.45 * (t / 0.4)) if t < 0.4 else 1 - (1 - amount) * ((t - 0.4) / 0.6)
            nw = max(1, round(w * s))
            row = cv2.resize(crop_rgba[y:y + 1], (nw, 1), interpolation=cv2.INTER_AREA)
            x0 = (w - nw) // 2
            out[y, x0:x0 + nw] = row[0]
        return out, "center"
    raise ValueError(kind)


meta = {}
for sid, sc in SCENARIOS.items():
    img = Image.open(f"images/{sc['image']}.jpg").convert("RGB")
    arr = np.asarray(img)
    H, W = arr.shape[:2]
    det = model.predict(img, threshold=0.3)
    cands = [i for i, c in enumerate(det.class_id) if COCO_CLASSES[int(c)] == sc["label"]]
    i = max(cands, key=lambda k: det.confidence[k])
    mask = det.mask[i].astype(np.uint8)
    x0, y0, x1, y1 = [int(round(v)) for v in det.xyxy[i]]
    x0, y0, x1, y1 = max(x0, 0), max(y0, 0), min(x1, W), min(y1, H)

    kernel = np.ones((15, 15), np.uint8)
    bg = cv2.inpaint(arr, cv2.dilate(mask, kernel) * 255, 5, cv2.INPAINT_TELEA)

    crop = np.dstack([arr[y0:y1, x0:x1], mask[y0:y1, x0:x1] * 255])
    warped, anchor = warp_crop(crop, *sc["warp"])
    wh, ww = warped.shape[:2]
    h, w = y1 - y0, x1 - x0
    if anchor == "x":
        nx0, ny0 = x0, y0 if sc["warp"][1] < 1 else y0
        if sc["warp"][1] > 1:
            nx0 = x0 - (ww - w) // 2
    elif anchor == "bottom":
        nx0, ny0 = x0, y1 - wh
    else:
        nx0, ny0 = x0 + (w - ww) // 2, y0 + (h - wh) // 2
    preview = bg.copy()
    tgt = np.zeros((H, W), np.uint8)
    ys, xs = max(ny0, 0), max(nx0, 0)
    ye, xe = min(ny0 + wh, H), min(nx0 + ww, W)
    sub = warped[ys - ny0:ye - ny0, xs - nx0:xe - nx0]
    a = (sub[..., 3:4] / 255.0)
    preview[ys:ye, xs:xe] = (a * sub[..., :3] + (1 - a) * preview[ys:ye, xs:xe]).astype(np.uint8)
    tgt[ys:ye, xs:xe] = (sub[..., 3] > 127)
    tys, txs = np.where(tgt > 0)
    target_box = [int(txs.min()), int(tys.min()), int(txs.max()), int(tys.max())]

    over = arr.copy()
    edge = cv2.morphologyEx(mask, cv2.MORPH_GRADIENT, np.ones((5, 5), np.uint8)) > 0
    over[edge] = (255, 140, 0)

    Image.fromarray(preview).save(f"out/scen/{sid}_preview.jpg", quality=95)
    Image.fromarray(over).save(f"out/scen/{sid}_overlay.jpg", quality=95)
    Image.fromarray(mask * 255).save(f"out/scen/{sid}_mask.png")
    meta[sid] = dict(sc, original=f"images/{sc['image']}.jpg", source_box=[x0, y0, x1, y1], target_box=target_box, size=[W, H])
    print(sid, "src", [x0, y0, x1, y1], "target", target_box)
json.dump(meta, open("out/scen/meta.json", "w"), indent=2)
