# Graph pool API v1

[中文](../../../../docs_cn/isos/interfaces/http/graph-pool_cn.md) · [Active graph API](graph.md)

The graph pool is the workspace for graph files under the local IsOS configuration directory. A deployment binding records only the selected pool name; the graph bytes exist only in the pool. All routes use the same loopback Host and Origin checks as the graph API. Saving the bound file changes its saved revision, so generated configuration must be refreshed with `PUT /api/isos/v1/graph` before Run. Saving does not start or stop a process.

## GET /api/isos/v1/graph-pool

Returns `data.graphs`, an array of `name`, exact-byte SHA-256 `revision`, and `version`, sorted by name. The pool accepts at most 128 drafts. Unsafe or oversized entries fail closed.

## GET /api/isos/v1/graph-pool/{name}

Returns `data:{name,revision,graph}` for one draft. Names contain 1–48 ASCII letters, digits, hyphens, or underscores. An absent name returns 404.

## PUT /api/isos/v1/graph-pool/{name}

Saves a `0.2.0` graph file. Start from the 16-node demo with separate button and mock-arm monitor chains returned by `GET /api/isos/v1/graph/new`, then send `{"schema_version":"0.1.0","revision":"","graph":<draft>}` to create a new name. To replace an existing file, send its exact revision returned by GET; the editor informs the user that saving to the same name replaces that file. A changed revision returns 409. Files may be incomplete while edited; Apply requires all three cores and validates the complete graph. Graphs are limited to 1 MiB.

The Web editor opens one named file from the pool. It combines the revision-checked selected owner catalog with the global catalog, so owner-only node types keep their settings and ports. Canvas positions and the last opened pool name remain browser-local. If that file disappears, the editor asks for a new name when saving.
