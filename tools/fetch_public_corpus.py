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
    """The Public Corpus directory: $SBM_PUBLIC_CORPUS, else the app cache (as `AppDirs` in Rust)."""
    if override := os.environ.get("SBM_PUBLIC_CORPUS"):
        return Path(override)
    if sys.platform == "win32":
        return Path(os.environ["LOCALAPPDATA"]) / "search-by-memory/cache/public-corpus"
    return Path(os.environ["HOME"]) / ".cache/search-by-memory/public-corpus"


def fetch_corpus(manifest: Path, corpus: Path, sleep: Callable[[float], None] = time.sleep) -> list[str]:
    """Downloads the files missing from `corpus`; returns those whose site could not be reached."""
    entries = tomllib.loads(manifest.read_text())["file"]
    unreachable = []
    for entry in entries:
        target = corpus / entry["path"]
        if target.is_file() and hashlib.sha256(target.read_bytes()).hexdigest() == entry["sha256"]:
            continue
        try:
            data = download(entry["url"], sleep)
        except urllib.error.URLError as error:
            print(f"{entry['path']}: {error}", file=sys.stderr)
            unreachable.append(entry["path"])
            continue
        if entry.get("convert") == "wikitext":
            verify(entry["path"], data, entry["source_sha256"])
            data = wikitext_to_text(data.decode()).encode()
        verify(entry["path"], data, entry["sha256"])
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_bytes(data)
    return unreachable


def verify(path: str, data: bytes, expected: str) -> None:
    actual = hashlib.sha256(data).hexdigest()
    if actual != expected:
        raise ValueError(f"{path}: sha256 {actual} does not match manifest {expected}")


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
    for link in code.filter_wikilinks():
        if str(link.title).strip().lower().startswith(NON_PROSE_LINKS):
            code.remove(link)
    text = code.strip_code()
    return text.strip() + "\n"


def main() -> None:
    corpus = Path(sys.argv[1]) if len(sys.argv) > 1 else default_corpus_dir()
    if unreachable := fetch_corpus(MANIFEST, corpus):
        sys.exit(f"Public Corpus in {corpus} lacks {len(unreachable)} files; run again later: {', '.join(unreachable)}")
    print(f"Public Corpus ready in {corpus}")


if __name__ == "__main__":
    main()
