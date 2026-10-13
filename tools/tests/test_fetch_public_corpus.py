import hashlib
import threading
from functools import partial
from http.server import HTTPServer, SimpleHTTPRequestHandler
from pathlib import Path

import pytest

from fetch_public_corpus import default_corpus_dir, fetch_corpus


@pytest.fixture
def server(tmp_path):
    """Serves files from a temporary directory over HTTP, standing in for the internet."""
    served = tmp_path / "served"
    served.mkdir()
    handler = partial(SimpleHTTPRequestHandler, directory=served)
    httpd = HTTPServer(("127.0.0.1", 0), handler)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    yield served, f"http://127.0.0.1:{httpd.server_port}"
    httpd.shutdown()


def write_manifest(path: Path, entries: list[dict]) -> Path:
    blocks = [
        "[[file]]\n" + "".join(f'{key} = "{value}"\n' for key, value in entry.items())
        for entry in entries
    ]
    path.write_text("\n".join(blocks))
    return path


def sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def test_missing_file_is_downloaded_into_the_corpus(tmp_path, server):
    served, base_url = server
    (served / "paper.pdf").write_bytes(b"%PDF paper")
    manifest = write_manifest(
        tmp_path / "manifest.toml",
        [{"path": "papers/paper.pdf", "url": f"{base_url}/paper.pdf", "sha256": sha256(b"%PDF paper"), "license": "CC-BY-4.0"}],
    )
    corpus = tmp_path / "corpus"

    fetch_corpus(manifest, corpus)

    assert (corpus / "papers/paper.pdf").read_bytes() == b"%PDF paper"


def test_hash_mismatch_fails_and_leaves_no_file_behind(tmp_path, server):
    served, base_url = server
    (served / "paper.pdf").write_bytes(b"tampered")
    manifest = write_manifest(
        tmp_path / "manifest.toml",
        [{"path": "paper.pdf", "url": f"{base_url}/paper.pdf", "sha256": sha256(b"original"), "license": "CC-BY-4.0"}],
    )
    corpus = tmp_path / "corpus"

    with pytest.raises(ValueError, match="sha256"):
        fetch_corpus(manifest, corpus)

    assert not (corpus / "paper.pdf").exists()


def test_file_already_in_the_corpus_is_not_downloaded_again(tmp_path, server):
    _, base_url = server  # serves nothing: any download attempt fails with 404
    manifest = write_manifest(
        tmp_path / "manifest.toml",
        [{"path": "paper.pdf", "url": f"{base_url}/paper.pdf", "sha256": sha256(b"cached"), "license": "CC-BY-4.0"}],
    )
    corpus = tmp_path / "corpus"
    corpus.mkdir()
    (corpus / "paper.pdf").write_bytes(b"cached")

    fetch_corpus(manifest, corpus)

    assert (corpus / "paper.pdf").read_bytes() == b"cached"


def test_corpus_lives_in_the_user_cache_unless_overridden(monkeypatch, tmp_path):
    monkeypatch.delenv("SBM_PUBLIC_CORPUS", raising=False)
    monkeypatch.setenv("HOME", str(tmp_path))
    assert default_corpus_dir() == tmp_path / ".cache/search-by-memory/public-corpus"

    monkeypatch.setenv("SBM_PUBLIC_CORPUS", str(tmp_path / "elsewhere"))
    assert default_corpus_dir() == tmp_path / "elsewhere"


def test_downloads_identify_the_tool_with_a_user_agent(tmp_path):
    # Wikimedia and arXiv reject anonymous default agents such as "Python-urllib".
    seen = []

    class RecordingHandler(SimpleHTTPRequestHandler):
        def do_GET(self):
            seen.append(self.headers["User-Agent"])
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b"data")

    httpd = HTTPServer(("127.0.0.1", 0), RecordingHandler)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    manifest = write_manifest(
        tmp_path / "manifest.toml",
        [{"path": "f", "url": f"http://127.0.0.1:{httpd.server_port}/f", "sha256": sha256(b"data"), "license": "CC0-1.0"}],
    )

    fetch_corpus(manifest, tmp_path / "corpus")
    httpd.shutdown()

    assert seen[0].startswith("search-by-memory-bench/")


def test_rate_limited_download_is_retried_after_the_server_asked_delay(tmp_path):
    # Wikimedia answers bursts of downloads with 429 Too Many Requests.
    attempts = []

    class RateLimitedOnceHandler(SimpleHTTPRequestHandler):
        def do_GET(self):
            attempts.append(self.path)
            if len(attempts) == 1:
                self.send_response(429)
                self.send_header("Retry-After", "7")
                self.end_headers()
                return
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b"data")

    httpd = HTTPServer(("127.0.0.1", 0), RateLimitedOnceHandler)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()
    manifest = write_manifest(
        tmp_path / "manifest.toml",
        [{"path": "f", "url": f"http://127.0.0.1:{httpd.server_port}/f", "sha256": sha256(b"data"), "license": "CC0-1.0"}],
    )
    slept = []

    fetch_corpus(manifest, tmp_path / "corpus", sleep=slept.append)
    httpd.shutdown()

    assert slept == [7]
    assert (tmp_path / "corpus/f").read_bytes() == b"data"
