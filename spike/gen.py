import base64
import io
import json
import sys
import time
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

import requests
from PIL import Image

MODELS = ["google/gemini-3.1-flash-image", "google/gemini-3-pro-image", "openai/gpt-5-image-mini"]
VARIANTS = ["text_overlay", "preview_ref"]
OUT = Path("out/gen")
URL = "https://openrouter.ai/api/v1/chat/completions"


def api_key():
    for line in open("../local.env"):
        if line.startswith("OPENROUTER_API_KEY="):
            return line.split("=", 1)[1].strip().strip('"')
    raise SystemExit("OPENROUTER_API_KEY missing")


KEY = api_key()
meta = json.load(open("out/scen/meta.json"))

TAIL = (" Everything outside the edited object must stay pixel-for-pixel identical to the original. "
        "Output only the full edited image with the same framing and aspect ratio as the original.")


def data_url(path, max_side=1280):
    im = Image.open(path).convert("RGB")
    im.thumbnail((max_side, max_side))
    buf = io.BytesIO()
    im.save(buf, "JPEG", quality=92)
    return "data:image/jpeg;base64," + base64.b64encode(buf.getvalue()).decode()


def build(sid, variant):
    m = meta[sid]
    orig = data_url(m["original"])
    if variant == "text_overlay":
        text = ("Edit the first image. The target object is outlined in orange in the second image (remove the outline in your output). "
                + m["instruction"] + TAIL)
        imgs = [orig, data_url(f"out/scen/{sid}_overlay.jpg")]
    else:
        text = ("Edit the first image. The second image is a rough, low-quality preview showing the desired new geometry "
                "(shape, size and position) of the target object; it contains artifacts, ignore them. " + m["instruction"]
                + " Reproduce the geometry shown in the second image, with photorealistic texture, lighting and detail matching the first image." + TAIL)
        imgs = [orig, data_url(f"out/scen/{sid}_preview.jpg")]
    content = [{"type": "text", "text": text}] + [{"type": "image_url", "image_url": {"url": u}} for u in imgs]
    return content


def run(job):
    model, variant, sid = job
    stem = f"{OUT}/{model.replace('/', '_')}__{variant}__{sid}"
    if Path(stem + ".png").exists():
        return job, "cached"
    body = {"model": model, "modalities": ["image", "text"], "usage": {"include": True},
            "messages": [{"role": "user", "content": build(sid, variant)}]}
    t = time.perf_counter()
    try:
        r = requests.post(URL, json=body, timeout=300, headers={"Authorization": f"Bearer {KEY}"})
        dt = time.perf_counter() - t
        data = r.json()
        if r.status_code != 200 or "choices" not in data:
            Path(stem + ".err.json").write_text(json.dumps(data)[:2000])
            return job, f"ERROR {r.status_code} {json.dumps(data)[:200]}"
        msg = data["choices"][0]["message"]
        images = msg.get("images") or []
        if not images:
            Path(stem + ".err.json").write_text(json.dumps(data)[:2000])
            return job, f"NO_IMAGE text={str(msg.get('content'))[:150]}"
        b64 = images[0]["image_url"]["url"].split(",", 1)[1]
        Path(stem + ".png").write_bytes(base64.b64decode(b64))
        usage = data.get("usage", {})
        Path(stem + ".json").write_text(json.dumps({"seconds": round(dt, 1), "cost": usage.get("cost"), "usage": usage}))
        return job, f"ok {dt:.0f}s cost={usage.get('cost')}"
    except Exception as e:
        return job, f"EXC {e}"


if __name__ == "__main__":
    only_model = sys.argv[1] if len(sys.argv) > 1 else None
    limit = int(sys.argv[2]) if len(sys.argv) > 2 else None
    jobs = [(m, v, s) for m in MODELS for v in VARIANTS for s in meta if only_model in (None, m)]
    if limit:
        jobs = jobs[:limit]
    with ThreadPoolExecutor(4) as ex:
        for job, res in ex.map(run, jobs):
            print(job, res, flush=True)
