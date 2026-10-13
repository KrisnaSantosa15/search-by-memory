"""Download the Public Corpus listed in benchmark/public/manifest.toml.

Each manifest entry names a file's path inside the corpus, its URL, its
sha256, and its license. Files live outside the repository.

Usage: uv run fetch_public_corpus.py [corpus_dir]
"""

import tomllib
import urllib.request
from pathlib import Path


def fetch_corpus(manifest: Path, corpus: Path) -> None:
    entries = tomllib.loads(manifest.read_text())["file"]
    for entry in entries:
        target = corpus / entry["path"]
        target.parent.mkdir(parents=True, exist_ok=True)
        with urllib.request.urlopen(entry["url"]) as response:
            target.write_bytes(response.read())
