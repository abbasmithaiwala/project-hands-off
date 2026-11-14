import json
import time

import requests

API = "https://commons.wikimedia.org/w/api.php"
UA = {"User-Agent": "hands-off-spike/0.1 (open-source image editor research; https://github.com/hands-off/hands-off)"}
PICKS = {
    "bottle": "File:Czech Glass Perfume Bottle-2.jpg",
    "couch": "File:ORANGE SOFA.jpg",
    "person_cup": "File:Smiling woman holding a cup of beer.jpg",
    "burger": "File:Big Mac hamburger.jpg",
    "person_pose": "File:Young Bravado.jpg",
}

def download(url, attempts=6):
    for n in range(attempts):
        resp = requests.get(url, headers=UA, timeout=60)
        if resp.status_code != 429:
            resp.raise_for_status()
            return resp
        time.sleep(5 * (n + 1))
    raise RuntimeError(f"rate limited: {url}")


attribution = {}
for key, title in PICKS.items():
    r = requests.get(API, headers=UA, timeout=30, params={
        "action": "query", "format": "json", "titles": title, "prop": "imageinfo",
        "iiprop": "url|extmetadata", "iiurlwidth": 1280}).json()
    page = next(iter(r["query"]["pages"].values()))
    info = page["imageinfo"][0]
    md = info["extmetadata"]
    img = download(info["thumburl"].split("?")[0])
    open(f"images/{key}.jpg", "wb").write(img.content)
    attribution[key] = {
        "title": title,
        "source": info["descriptionurl"],
        "license": md["LicenseShortName"]["value"],
        "license_url": md.get("LicenseUrl", {}).get("value", ""),
        "artist": md.get("Artist", {}).get("value", ""),
    }
    print(key, len(img.content))
    time.sleep(3)
json.dump(attribution, open("images/ATTRIBUTION.json", "w"), indent=2)
