---
status: proposed
date: 2026-02-07
---

# Export/import format: plain folder with index.json

## Context and Problem Statement

Dromos needs a portable format for sharing ROM graphs. This includes node metadata (hash, title, version, etc), links between nodes, and the binary diff files for each link. The format should be easy to inspect, create, and consume.

## Decision Drivers

- Must bundle both structured metadata (JSON) and binary diff files.
- Should be directly inspectable without special tools.
- Should be simple to implement and maintain.

## Considered Options

- ZIP archive containing index.json + diffs/
- Plain folder containing index.json + diffs/
- Single JSON file with base64-encoded diffs inline

## Decision Outcome

Chosen option: "Plain folder", because it is the simplest approach and makes contents directly inspectable with standard filesystem tools.

### Folder Structure

```text
my-export/
├── index.json
└── diffs/
    ├── abcdef01_12345678.bsdiff
    └── ...
```

### `index.json` Schema

```json
{
  "dromos_export": { "version": 1, "data_revision": 2, "exported_at": "..." },
  "files": [{ "sha256": "...", "title": "...", "rom_type": "NES", ... }],
  "diffs": [{
    "source_sha256": "...",
    "target_sha256": "...",
    "diff_path": "...",
    "diff_size": 1234,
    "sha256": "..."
  }]
}
```

- `files`: array of ROM node metadata (hash, title, type, version, etc.)
- `diffs`: array of diff edges with SHA-256 checksums for integrity verification
- Each diff entry's `sha256` field contains the hex-encoded SHA-256 hash of the corresponding `.bsdiff` file

### Consequences

- Good, because contents are directly inspectable with any file browser or text editor
- Good, because no dependency on a ZIP library
- Good, because diff file integrity is verified via SHA-256 checksums in index.json
- Neutral, because folder is not a single file (slightly less convenient to transfer)
- Bad, because no compression of index.json (negligible for typical metadata sizes)
