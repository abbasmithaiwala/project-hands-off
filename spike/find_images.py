import re, sys

import requests

API = "https://commons.wikimedia.org/w/api.php"
UA = {"User-Agent": "hands-off-spike/0.1 (open-source image editor research; https://github.com/hands-off/hands-off)"}
OK = re.compile(r"^(CC0|Public domain|CC BY(-SA)? [\d.]+|PD)", re.I)

def get(params):
    return requests.get(API, params=params, headers=UA, timeout=30).json()

def search(q):
    r = get({"action": "query", "format": "json", "generator": "search", "gsrsearch": f"{q} filetype:bitmap",
             "gsrnamespace": 6, "gsrlimit": 12, "prop": "imageinfo", "iiprop": "url|size|extmetadata|mime",
             "iiurlwidth": 1280})
    out = []
    for p in r.get("query", {}).get("pages", {}).values():
        ii = p["imageinfo"][0]
        md = ii.get("extmetadata", {})
        lic = md.get("LicenseShortName", {}).get("value", "")
        if ii["mime"] != "image/jpeg" or not OK.match(lic) or ii["width"] < 1000:
            continue
        out.append((p["title"], lic, md.get("Artist", {}).get("value", "")[:60], ii["width"], ii["height"], ii.get("thumburl")))
    return out

for q in sys.argv[1:]:
    print("##", q)
    for t in search(q)[:6]:
        print(t[:5], t[5])
