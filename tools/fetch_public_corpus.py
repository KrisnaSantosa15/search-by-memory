"""Download the Public Corpus listed in benchmark/public/manifest.toml.

Each manifest entry names a file's path inside the corpus, its URL, its
sha256, and its license. Files live outside the repository.

Usage: uv run fetch_public_corpus.py [corpus_dir]
"""

import hashlib
import os
import sys
import time
import tomllib
import urllib.error
import urllib.request
from collections.abc import Callable
from pathlib import Path

import mwparserfromhell


# English and Indonesian namespaces of images and categories, which are not article prose.
NON_PROSE_LINKS = ("file:", "image:", "berkas:", "gambar:", "category:", "kategori:")
USER_AGENT = "search-by-memory-bench/0.1 (offline Benchmark corpus fetcher)"
MANIFEST = Path(__file__).resolve().parent.parent / "benchmark/public/manifest.toml"


def default_corpus_dir() -> Path:
    """The Public Corpus directory: $SBM_PUBLIC_CORPUS, else the user cache."""
    if override := os.environ.get("SBM_PUBLIC_CORPUS"):
        return Path(override)
    return Path.home() / ".cache/search-by-memory/public-corpus"


def fetch_corpus(manifest: Path, corpus: Path, sleep: Callable[[float], None] = time.sleep) -> None:
    entries = tomllib.loads(manifest.read_text())["file"]
    for entry in entries:
        target = corpus / entry["path"]
        if target.is_file() and hashlib.sha256(target.read_bytes()).hexdigest() == entry["sha256"]:
            continue
        data = download(entry["url"], sleep)
        actual = hashlib.sha256(data).hexdigest()
        if actual != entry["sha256"]:
            raise ValueError(f"{entry['path']}: sha256 {actual} does not match manifest {entry['sha256']}")
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)


def download(url: str, sleep: Callable[[float], None], attempts: int = 5) -> bytes:
    """Downloads `url`, waiting as asked (Retry-After) when the server rate-limits."""
    request = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    for attempt in range(1, attempts + 1):
        try:
            with urllib.request.urlopen(request) as response:
                return response.read()
        except urllib.error.HTTPError as error:
            if error.code != 429 or attempt == attempts:
                raise
            sleep(float(error.headers.get("Retry-After") or 30))
    raise AssertionError("unreachable")


def wikitext_to_text(wikitext: str) -> str:
    """Plain text of a Wikipedia article's wikitext, as a reader would save it."""
    code = mwparserfromhell.parse(wikitext)
    for ref in code.filter_tags(matches=lambda tag: tag.tag.lower() == "ref"):
        code.remove(ref)
    for link in code.filter_wikilinks(recursive=False):
        if str(link.title).strip().lower().startswith(NON_PROSE_LINKS):
            code.remove(link)
    text = code.strip_code()
    return text.strip() + "\n"


def main() -> None:
    corpus = Path(sys.argv[1]) if len(sys.argv) > 1 else default_corpus_dir()
    fetch_corpus(MANIFEST, corpus)
    print(f"Public Corpus ready in {corpus}")


if __name__ == "__main__":
    main()
