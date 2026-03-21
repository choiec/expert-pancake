# Quickstart: Memory Ingest with Canonical Source Identity

**Status**: IMPLEMENT-READY

## Purpose

This quickstart documents the local validation flow for the merged `001-memory-ingest` slice after canonical source identity governance was folded in from `002`.

## Prerequisites

- Rust stable toolchain with edition 2024 support
- Docker and Docker Compose or equivalent local containers
- `curl` for HTTP checks

## Environment

```bash
export APP_LISTEN_ADDR=127.0.0.1:3000
export SURREALDB_URL=ws://127.0.0.1:8000/rpc
export SURREALDB_NAMESPACE=memory
export SURREALDB_DATABASE=memory
export SURREALDB_USERNAME=root
export SURREALDB_PASSWORD=root
export MEILI_HTTP_ADDR=http://127.0.0.1:7700
export MEILI_MASTER_KEY=local-dev-key
```

## Run the Service

```bash
cargo run -p app_server
```

## Smoke Test: Canonical Manual Ingest

```bash
curl -i http://127.0.0.1:3000/sources/register \
  -H 'content-type: application/json' \
  --data '{
    "title": "Axum Plan",
    "summary": "Planning notes",
    "external-id": "https://api.cherry-pick.net/qti/v3p0/kice.re.kr:20240621",
    "document-type": "markdown",
    "content": "# Intro\n\nHello world\n\n# Next\n\nMore text",
    "metadata": {"topic": "planning"}
  }'
```

Expected behavior:

- `201 Created` for first registration
- `200 OK` for semantic replay
- `409 Conflict` for the same canonical identity with semantically different content

## Smoke Test: Direct Standard Ingest

```bash
curl -i http://127.0.0.1:3000/sources/register \
  -H 'content-type: application/json' \
  --data '{
    "@context": ["https://www.w3.org/ns/credentials/v2"],
    "type": ["VerifiableCredential", "OpenBadgeCredential"],
    "id": "urn:example:badge:001",
    "name": "Rust Badge",
    "issuer": {"id": "https://issuer.example.org"}
  }'
```

Expected behavior:

- response returns canonical `external_id`
- `source_metadata.system.ingest_kind = direct_standard`
- `source_metadata.system.original_standard_id` preserves the upstream `id`
- one derived `json_document` memory item is returned
- formatting-only JSON replay resolves to the same authoritative source

## Retrieval Checks

```bash
curl -i http://127.0.0.1:3000/sources/{source_id}
curl -i http://127.0.0.1:3000/memory-items/{urn}
```

Expected behavior:

- source retrieval returns canonical identity plus provenance metadata
- memory-item retrieval returns authoritative stored content exactly as committed

## Search Check

```bash
curl -i 'http://127.0.0.1:3000/search/memory-items?q=rust&limit=10'
```

Search returns projection hits only. Search degradation may return `503` without changing authoritative write or retrieval behavior.
