"""Write reference embeddings from the official sentence-transformers pipeline.

The Rust parity test (crates/embed-onnx/tests/embedding.rs) checks that the
ONNX embedder reproduces these vectors (ADR 0001 go/no-go gate).

Usage: uv run make_parity_fixtures.py
"""

import json
from pathlib import Path

from sentence_transformers import SentenceTransformer

MODEL_ID = "google/embeddinggemma-2"
OUT = Path(__file__).resolve().parent.parent / "crates/embed-onnx/tests/fixtures/parity.json"

QUERIES = [
    "tutorial explaining attention mechanisms: query, key and value",
    "auth service code that refreshes JWT tokens",
    "meeting notes about the third-quarter budget",
]
CHUNKS = [
    "Transformers compute attention by comparing query vectors against key vectors.",
    "pub async fn refresh_jwt_token(token: &str) -> Result<Token, AuthError>",
    "The meeting discussed the third-quarter budget and next year's sales targets.",
    "Preheat the oven to 180 degrees and bake for 25 minutes.",
]


def main() -> None:
    model = SentenceTransformer(MODEL_ID, device="cpu")
    print("prompts:", model.prompts)
    items = [
        {"kind": "query", "text": text, "embedding": vector.tolist()}
        for text, vector in zip(QUERIES, model.encode_query(QUERIES))
    ] + [
        {"kind": "chunk", "text": text, "embedding": vector.tolist()}
        for text, vector in zip(CHUNKS, model.encode_document(CHUNKS))
    ]
    OUT.write_text(json.dumps({"model": MODEL_ID, "dimensions": 768, "items": items}))
    print(f"wrote {len(items)} reference embeddings to {OUT}")


if __name__ == "__main__":
    main()
