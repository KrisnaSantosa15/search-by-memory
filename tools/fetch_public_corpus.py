"""Download the Public Corpus listed in benchmark/public/manifest.toml.

Each manifest entry names a file's path inside the corpus, its URL, its
sha256, and its license. Files live outside the repository.

Usage: uv run fetch_public_corpus.py [corpus_dir]
"""

import hashlib
import os
import sys
import tomllib
import urllib.request
from pathlib import Path


MANIFEST = Path(__file__).resolve().parent.parent / "benchmark/public/manifest.toml"


def default_corpus_dir() -> Path:
    """The Public Corpus directory: $SBM_PUBLIC_CORPUS, else the user cache."""
    if override := os.environ.get("SBM_PUBLIC_CORPUS"):
        return Path(override)
    return Path.home() / ".cache/search-by-memory/public-corpus"


def fetch_corpus(manifest: Path, corpus: Path) -> None:
    entries = tomllib.loads(manifest.read_text())["file"]
    for entry in entries:
        target = corpus / entry["path"]
        if target.is_file() and hashlib.sha256(target.read_bytes()).hexdigest() == entry["sha256"]:
            continue
        with urllib.request.urlopen(entry["url"]) as response:
            data = response.read()
        actual = hashlib.sha256(data).hexdigest()
        if actual != entry["sha256"]:
            raise ValueError(f"{entry['path']}: sha256 {actual} does not match manifest {entry['sha256']}")
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)


def main() -> None:
    corpus = Path(sys.argv[1]) if len(sys.argv) > 1 else default_corpus_dir()
    fetch_corpus(MANIFEST, corpus)
    print(f"Public Corpus ready in {corpus}")


if __name__ == "__main__":
    main()
