# Phase 1 spike (throwaway)

Scripts used to answer the two Phase 1 gate questions. Findings live in `docs/adr/0003` and `docs/adr/0004`; results and golden images in `eval/`. This code is not part of the product.

Run from this directory with a venv that has the packages listed in `requirements.txt`; the OpenRouter key is read from `../local.env`.

Order: `fetch_images.py`, `detect.py`, `export_bench.py`, `sam_test.py`, `prepare.py`, `gen.py`, `evaluate.py`.
