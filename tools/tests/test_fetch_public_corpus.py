import hashlib
import threading
from functools import partial
from http.server import HTTPServer, SimpleHTTPRequestHandler
from pathlib import Path

import pytest

from fetch_public_corpus import fetch_corpus


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
