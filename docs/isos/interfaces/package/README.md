# Generic package contributions

<!-- doc-navigation:start -->
<details>
<summary>Folder: isos/interfaces/package</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)

</details>
<!-- doc-navigation:end -->

## Table of contents

- [Package admission and publication rules](#package-admission-and-publication-rules)
- [Package release contract](#release--package-release-contract)
- [Generic package resources](#resources--generic-package-resources)
- [Generic process contribution](#process--generic-process-contribution)
- [Verified package admission](#verification--verified-package-admission)
- [Sign in for package publication](#authentication--sign-in-for-package-publication)


This page is for package publishers and core authors. IsOS currently treats every package by the same exact-pin and verified-byte path; `kind` is descriptive metadata only. A package contributes registered node types, resources, processes, data types, and optional node sets through `node_contributions`. The owning core validates any payload-specific semantics before readiness.

- [Release](README.md#release) defines the package identity, version, provenance, assets, and contribution envelope; it is the entry point for a publisher.
- [Package-owned configuration form](README.md#release--package-owned-configuration-form) defines bilingual field metadata, validation bounds, and template values that the generic Web editor displays.
- [Resources](README.md#resources) defines stable IDs, kinds, display metadata, contained files, and digests that instances can select without path guessing.
- [Process](README.md#process) defines entrypoint, lifecycle adapter, host inputs, and resource references for an owner-run process.
- [Verification](README.md#verification) explains exact-pin, file, catalog, and owner capability checks applied before graph publication or launch.

The [registered node contract](../node/README.md) defines the same type and port rules for package and built-in nodes. The [core handoff](../core.md#core_handoff) binds a verified process to one run and revision.

## Package admission and publication rules

IsOS installs one exact `{id,version}` release only after the selected asset and every declared file match their digests. Installation makes its types and resources selectable; it does not change a saved graph, Apply state, or active run. Apply checks the selected release, owner claims, and graph revision; Run rechecks the saved bytes and launch inputs. Stop requires owner quiescence and direct-child exit before the run is retired.

`pkg update` reports approved patch releases for locally verified package IDs. `pkg upgrade` rechecks the selected release and requires asset-bound approval before installing it beside the old pin. Neither operation changes a graph pin. A published release requires a passing exact-version qualification, immutable matching source tag, verified archive bytes, and successful managed replay; draft or partial uploads are not published releases. See [package operations](../../core/pkg.md) for commands and recovery steps.

`pkg refresh-sources` checks only `local_source` packages pinned by the bound graph. It reuses verified installed bytes when the recorded checkout is clean and HEAD still matches the receipt; otherwise it rebuilds from the clean checkout. An explicit `pkg install --source` forces a build. Catalog-installed release bytes remain unchanged. `install_dev.sh` calls this check by default; `--no-source-refresh` skips it.

<a id="release"></a>
<a id="release--package-release-contract"></a>
## Package release contract

[中文](../../../../docs_cn/isos/interfaces/package/README_cn.md#release) · [Verification](README.md#verification) · [Resources](README.md#resources)

`isos.package@v1` is the active IsOS contribution identity. New releases use `"schema_version":"0.2.1"`; existing exact pins using `0.2.0` and `0.1.0` remain readable. `node_contributions` declares registered node types, Linux `.so` libraries, external interfaces, routes, processes, resources, data types, and node sets. IsOS reads a release only through an exact `{id,version}` pin. A selected package type must be declared in that verified release before Apply, Run, or Restart; a runtime service list alone does not register a type. Historical releases with general EOS hardware names remain readable, but Apply requires an actual verified type declaration.

| Field | Type / unit | Purpose and validation | Absence/default |
| --- | --- | --- | --- |
| `contract` | string / identity | Must equal `isos.package@v1`. | Required. |
| `schema_version` | string / version | New releases use `0.2.1`; `0.2.0` and `0.1.0` are read for existing pins. | Required. |
| `id`, `version` | strings / exact package pin | Canonical package ID and exact release version; never infer latest. | Required. |
| `kind` | string / catalog class | Bounded stable descriptor for discovery; it is metadata and does not select behavior or a parser. | Required. |
| `source_commit` | 40-hex string / Git commit | Publisher provenance; independent of payload digest. | Required. |
| `description`, `license`, `source_url`, `release_url` | strings / display or URL | Catalog metadata only; cannot grant runtime capability. | Optional, absent. |
| `contracts` | string array / identities | Declared external capabilities; owner admission still verifies them. | Optional, empty. |
| `compatibility` | object / qualification bounds | `isos_min_inclusive` and `isos_max_exclusive` are SemVer bounds; `required_owner_capabilities[]` contains unique `{owner,capability}` pairs for `isos`, `ei`, or `eos`. IsOS rejects an incompatible installed build; the selected owner must advertise required capabilities before launch. | Optional for existing releases. |
| `node_contributions` | object / declarations | Types, libraries, processes, data types, routes, bridges, and node sets; IsOS validates structure and ownership. | Required for Apply/Run. |
| `assets` | array / candidate artifacts | Host-matched, bounded download candidates with URL, byte length, and SHA-256. | Required for archive install. |

Schema `0.2.1` adds `node_contributions.runtime_services[]` and `package_config`. Each service declares an ID, package-relative entrypoint, bounded arguments (only `${socket}` expands), configuration service type, and contributed node type IDs. `package_config` binds a package-relative JSON descriptor and TOML template with SHA-256 digests. The descriptor contract is `isos.package-config@v1`, schema `0.1.0`; its `package`, `host`, and `services.<instance>` sections carry typed fields and bilingual labels and descriptions. Local TOML is stored per deployment and exact package pin outside the verified archive. Saving an unchanged configuration does not create a new revision; a rejected edit preserves the saved revision. The HTTP `packages/config` endpoint returns descriptor, TOML, parsed values, and revision; revision-checked `PUT` saves, and `packages/config/validate` checks without saving.


A release may contain owner-defined opaque metadata only inside a bounded owner extension. IsOS never interprets that metadata as scene, device, library, or adapter authority. The owning core validates its semantics before declaring readiness. The [process contract](README.md#process) defines the generic contribution fields.

The package ID is a stable `namespace/name`; the version is an exact pin. A new version gets its own receipt and payload, so installing it does not replace older pins or rewrite saved graphs. Package kind and publisher name do not select a runtime parser. A contributed node type's core prefix and declared owner must agree; a process may run only under its own core's `process` layer. Historical receipts remain readable, while a cross-owner process placement is rejected at admission.

```json
{
  "contract": "isos.package@v1",
  "schema_version": "0.2.1",
  "id": "vendor/sample-a",
  "kind": "component",
  "version": "1.2.3",
  "source_commit": "0000000000000000000000000000000000000000",
  "node_contributions": {
    "node_types": [],
    "libraries": [],
    "port_routes": [],
    "bridge_adapters": [],
    "processes": [],
    "node_sets": []
  },
  "assets": []
}
```

The empty example is structurally illustrative; admission requires the selected registered type and declared owner capabilities. The [CLI publication procedure](../../core/pkg.md) is separate from package admission.

<a id="release--package-owned-configuration-form"></a>
### Package-owned configuration form

A publisher can expose package settings in the Web package inspector by including `package_config` in the verified release. Its `descriptor` and `template` are package-relative files, each bound by its SHA-256 in the release manifest. IsOS returns the verified descriptor through `GET /api/isos/v1/packages/config?id=<id>&version=<version>`; the Web client renders its fields without package-specific UI code. The descriptor supplies the English and Chinese section and field labels, descriptions, and optional notes. Put package-specific wording and limits here, then publish a new exact version when they change. An already installed version retains its original verified descriptor.

The JSON descriptor has `contract:"isos.package-config@v1"`, `schema_version:"0.1.0"`, and `sections[]`. Each section has a unique `id` (`package`, `host`, or `services.<instance>`), bilingual `label` and `description`, and `fields[]`. Each field has a unique slug `name`, `type` (`string`, `integer`, `boolean`, `directory`, `file`, or `enum`), `required`, bilingual `label` and `description`, and optional bilingual `note`. An integer field may declare `min` and `max`; the form uses them as input bounds and IsOS validates them when saving. A `default` can fill an initially empty form input, but the package's TOML template must contain the actual configured value: the default alone is not written to TOML or passed to an owner. File and directory values must be absolute; selected services require existing paths of the declared kind. Runtime services and their environment templates may refer only to declared configuration fields.

An optional `setup` object may name a verified package-relative executable for interactive CLI configuration. It declares a `service` matching a `services.<instance>` section, `entrypoint`, bilingual `prompt`, and `outputs` naming declared host file or directory fields. When any output path is missing, `isos pkg configure <pin> --service <instance>` asks before running the helper. The helper receives a deployment-local data directory and an output JSON path, then writes only the declared absolute paths; IsOS checks their kinds and presents the populated TOML in the editor before saving. Declining keeps manual configuration available. Noninteractive configuration and the Web form do not execute helpers.

For a bounded string selector, declare `"type":"enum"` and a nonempty `options` array of `{ "value": "stable-id", "label": { "en": "Display name", "cn": "显示名称" } }` objects. Values must be distinct, nonempty strings; labels must be nonempty in both languages. An enum cannot use numeric bounds, and its optional `default` must match an option. IsOS rejects any TOML value outside the declared options, including edits through the raw TOML view or CLI. The default only initializes the form; include the desired value in the TOML template to make it active.

For example, a package can declare a `services.camera` integer field with `"name":"width"`, `"min":64`, `"max":2048`, bilingual labels and descriptions, and put `width = 1280` under `[services.camera]` in its template. The package owner still validates that value against its own camera implementation. The UI and IsOS do not assign camera semantics to the field. The [package configuration API](../api/http.md#http_packages) supports reading, validating, and revision-checked saving of this same generic form; configuration is stored per deployment and exact pin, separate from graph drafts. Saving changed configuration requires Apply before the next Run.

This small descriptor and template illustrate the file pair; the release manifest must bind their exact paths and digests:

```json
{
  "contract": "isos.package-config@v1",
  "schema_version": "0.1.0",
  "sections": [
    {
      "id": "package",
      "label": {
        "en": "Package",
        "cn": "软件包"
      },
      "description": {
        "en": "Exact pin identity.",
        "cn": "精确版本标识。"
      },
      "fields": [
        {
          "name": "id",
          "type": "string",
          "required": true,
          "label": {
            "en": "Package ID",
            "cn": "软件包标识"
          },
          "description": {
            "en": "Must match the release ID.",
            "cn": "须与发行包标识一致。"
          }
        }
      ]
    },
    {
      "id": "services.camera",
      "label": {
        "en": "Camera service",
        "cn": "相机服务"
      },
      "description": {
        "en": "Camera settings.",
        "cn": "相机设置。"
      },
      "fields": [
        {
          "name": "width",
          "type": "integer",
          "required": true,
          "min": 64,
          "max": 2048,
          "label": {
            "en": "Width (px)",
            "cn": "宽度（像素）"
          },
          "description": {
            "en": "Frame width.",
            "cn": "画面宽度。"
          }
        }
      ]
    }
  ]
}
```

```toml
[package]
id = "vendor/camera"

[services.camera]
width = 1280
```

<a id="release--runtime-service-fields-021"></a>
### Runtime service fields (`0.2.1`)

A service may declare `resources[]` with the same `{setting,kind,descriptor}` verified resource catalog reference used by a process. Its arguments may contain `${socket}` or a matching `${resource.<setting>.path}`. `environment[]` contains bounded `{name,value}` entries. Values may interpolate `${config.host.<field>}`, `${config.service.<field>}`, `${generation_id}`, and `${selected_nodes.settings_json}`. The last token expands to a JSON object keyed by selected device ID, containing only settings listed in `required_node_settings`. Unknown templates and reserved environment names are rejected. A service starts only when selected EOS hardware nodes reference its exact package pin and `service_instance`.

`runtime_services[].required_node_settings` is an optional map from a service node type ID to a nonempty list of setting names declared by that node type. Each selected node of that type must provide those settings at Apply and Run. The node type setting schema validates value types and bounds at Apply. Settings stay on the graph node, so an edited value takes effect after Apply and the next Run.

Apply records the selected service instances and SHA-256 of their deployment-local TOML. Run and Restart compare those values with the selected generation and reject a changed configuration until Apply is repeated. IsOS starts each selected service once per exact pin and instance before EOS readiness, and passes a private socket and service generation to EOS in the Run request. The socket and executable path are never graph-authored. EOS Stop proof precedes IsOS termination and reaping of these service children; an unconfirmed child exit is reported as unknown.

A `runtime_services[].requirements[]` entry has `{path,kind}`. `kind` is `file` or `directory`; `path` begins with `${config.host.<field>}` or `${config.service.<field>}` and may have a relative suffix. IsOS canonicalizes each selected path. For a file it records the content SHA-256 in the Apply generation and compares it at Run and Restart, so a local library replacement requires Apply again. This is generic package metadata; the package chooses which local files must be pinned.



<a id="resources"></a>
<a id="resources--generic-package-resources"></a>
## Generic package resources

[中文](../../../../docs_cn/isos/interfaces/package/README_cn.md#resources) · [Release](README.md#release) · [Process](README.md#process)

A verified release may declare resources for selectors and process settings. The descriptor contract is `isos.package-resources@v1` with `schema_version:"0.1.0"`; it contains at most 256 resource records and is bounded to 256 KiB. IsOS bounds the descriptor, verifies declared file containment and digest, and exposes stable IDs. It does not parse resource bytes or interpret relationships in owner metadata. The selected core must validate semantics before readiness. A resource file is never a graph setting or executable authority.

| Field | Type / unit | Purpose and validation | Absence/default |
| --- | --- | --- | --- |
| `kind` | bounded string / family | Names the registered resource family; not a hard-coded package class. | Required. |
| `id` | bounded string / stable selector ID | Unique with `kind` in one release; path-free and suitable for graph storage. | Required. |
| `label` | bounded string / display text | Human-readable choice name; cannot affect admission. | Required. |
| `description` | bounded string / display text | Selection guidance; cannot affect admission. | Required. |
| `path` | safe relative string or null / payload path | Optional regular file within the verified payload. | Null for logical resources. |
| `sha256` | lowercase 64-hex string or null / file bytes | Exact digest, rechecked at admission; present iff `path` is present. | Null for logical resources. |
| `metadata` | bounded JSON value / owner-defined | At most 16 KiB serialized; opaque to IsOS, with relationships validated by owner. | Optional, null. |

Unknown IDs, duplicate `(kind,id)`, changed bytes, escaping paths, and symlinks fail with bounded package diagnostics. The canonical graph pointer is retained in errors. A stale revision fails before mutation. The same checks run before Apply, Run, and Restart.

```json
{
  "kind": "configuration",
  "id": "balanced",
  "label": "Balanced",
  "description": "General profile",
  "path": "profiles/balanced.json",
  "sha256": "0000000000000000000000000000000000000000000000000000000000000000"
}
```

The hash is format-only. A publisher must use the real file digest.

<a id="process"></a>
<a id="process--generic-process-contribution"></a>
## Generic process contribution

[中文](../../../../docs_cn/isos/interfaces/package/README_cn.md#process) · [Resources](README.md#resources) · [Owner handoff](../core.md#core_handoff)

A `node_contributions.processes[]` entry describes a package process without granting launch authority. IsOS resolves the exact verified pin, matches the registered node type and placement, checks declared resource IDs and digests, and selects the owning core. The owner verifies its lifecycle adapter. IsOS supervises only IsOS-owned direct children; another core supervises its own children.

An owner package process uses a registered `<core>.process.<group>.<node>@v0.2` type at `<core>/process/<group>/<instance>`, where `<core>` is `isos`, `ei`, or `eos`. The process instance carries an exact `node.package` pin and stable resource IDs in settings. IsOS checks the selected package during preview and Apply; the selected owner must accept its lifecycle adapter before a run-bound handoff is admitted. The owner launches and stops its own direct child.

Current process contributions use `implementation.kind: "managed_process"`, `lifecycle: "process"`, and the declared owner core and `process` layer. Historical `0.1.x` external process records remain readable under their original names but are not executable through this contract.

The current EOS owner accepts one package process per graph. A second pinned EOS process rejects before selecting a backend for launch or Stop. Historical cross-owner graph placements remain readable as records but are rejected at execution admission.

The graph also selects the built-in IsOS owner node `isos.process.component@v0.2` at `isos/process/component/eos`. A package lifecycle family is admitted when the EOS catalog reports it as compiled or loadable from a verified package adapter library. A family with neither capability is rejected.

The optional `node_contributions.processes[].args` array contains at most 16 command arguments, each at most 256 bytes. An argument may be a literal, `${socket}`, or `${resource.<setting>.path}`. The owner binds `${socket}` to a private run socket and a resource placeholder to the verified selected file; unknown or missing placeholders reject the launch. The owner checks entrypoint and selected resource digests again before spawn. The declaration has no shell interpretation.

| Field | Type / unit | Purpose and validation | Absence/default |
| --- | --- | --- | --- |
| `node_type` | string / registered `@v0.2` identity | Links this process to one registered type. | Required. |
| `entrypoint` | safe relative string / payload path | Verified executable regular file; no fixed filename or package-ID rule. | Required for direct-child process. |
| `lifecycle_adapter` | string / owner capability | Must be declared and accepted by selected owner; unknown adapter fails. | Required. |
| `resources` | array / setting references | Each setting names a registered resource family and descriptor; selected ID is resolved against verified release. | Empty if none. |
| `host_inputs` | array / declared host requirements | Named inputs verified by the owner; values never enter release or saved graph. | Empty if none. |

The bound `isos.package-process@v1` handoff carries the verified executable and resources, owning core, lifecycle adapter, run ID, and saved revision. Dependency order and readiness/stop conditions remain in the IsOS runtime plan and owner policy; they are not fields in this handoff. Owner rejection, failed readiness, or uncertain stop never becomes a successful replacement. The [handoff contract](../core.md#core_handoff) defines wire fields.

For example, `eos.process.vendor.worker@v0.2` at `eos/process/vendor/main` selects a declared lifecycle adapter and verified resources. A missing declaration, changed package byte, unavailable adapter, unknown resource ID, or wrong process placement rejects admission before the owner receives a launch.

<a id="process--generic-skill-selection"></a>
### Generic skill selection

A skill library is selected only by exact package pin and a declared regular file with SHA-256. IsOS verifies the release and selected bytes; the owning runtime validates ABI and callable semantics. Recursive file discovery never grants execution authority. Selection changes require revision-checked Apply and revalidation before Run and Restart. A missing or changed library fails before active replacement.

<a id="verification"></a>
<a id="verification--verified-package-admission"></a>
## Verified package admission

[中文](../../../../docs_cn/isos/interfaces/package/README_cn.md#verification) · [Release](README.md#release) · [Resources](README.md#resources)

Installation verifies one exact `{id,version}` release and asset, then publishes receipt and payload atomically. A local link freezes the accepted metadata and payload fingerprint; changed linked bytes require explicit relink. A package installed with `pkg install --source <absolute-checkout> --allow-source-build` records the checkout as `local_source`. Apply and Run require a clean checkout and verify the installed package. When HEAD matches the receipt source commit, they reuse that verified candidate; when HEAD changes, they rebuild and replace the previous candidate only after verification succeeds. A failed rebuild rejects that operation and preserves the previous package. Published packages remain pinned to exact versions and byte digests. No unrelated installed package can block a graph that does not pin it.

Admission is a single sequence: exact pin → release contract and payload receipt → registered node type and placement → declared resource IDs and contained file digests → owner and lifecycle capability → run-bound handoff. The pin collector covers all supported graph pin locations and sorts duplicate diagnostics by canonical graph pointer. There is no package-specific priority or fallback parser.

| Failure | Outcome | State effect |
| --- | --- | --- |
| Missing required generic contributions | `unsupported_package_contract` | No saved graph or active run change. |
| Invalid release, unsafe path, symlink, digest mismatch, or oversized metadata | Bounded package diagnostic with graph pointer and stage | No partial install or replacement. |
| Unknown registered type, placement, owner, or adapter | Typed admission diagnostic | No child spawn or graph publication. |
| Unknown resource ID or changed resource bytes | Typed resource diagnostic | No child spawn or graph publication. |
| Stale saved revision or run fence | Conflict diagnostic | No active replacement. |

Direct install/link errors use an empty graph pointer; graph-time errors use the actual selected node pointer. Diagnostics contain a stable code, stage, exact pin, sanitized message, and canonical pointer; they never expose local paths, raw metadata, or secrets. See [HTTP packages](../api/http.md#http_packages) for the response envelope.

<a id="verification--reader-and-feature-preflight"></a>
### Reader and feature preflight

IsOS checks the selected executable and its exact verified package contributions before Apply, Run, and Restart. A product version, a different checkout, an executable path alone, or a package type suffix does not prove support. Capability probes are bounded, start no runtime, and must describe the bytes that will launch. A changed executable, interpreter, module tree, package receipt, or generated revision blocks replacement. Missing interpreter does not trigger `uv sync` or another bootstrap. The external setup reply uses schema 3; IsOS-owned HTTP contracts use `"0.1.0"` independently.

<a id="verification--reader-identity-fields"></a>
### Reader identity fields

| Field | JSON type / unit | Purpose, absence and validation |
| --- | --- | --- |
| `supported_graph_versions` | nonempty array of unique strings / graph versions | Exact versions read by the selected binary. Required when the selected reader declares graph-version support. |
| `supported_node_type_versions` | nonempty array of unique strings / type suffixes | Exact suffixes read by the binary; never sufficient to admit a full type ID. |
| `supported_node_types` | array of unique full IDs / types | Built-in IDs implemented by the reader. A contributed type additionally requires a verified exact package pin and owner catalog. |
| `features` | array of unique ASCII strings / capabilities | Actual capabilities of that binary. Missing means empty; required feature then fails. |
| `binary` | string / selected executable identity | Runtime `--capabilities` role, such as `aether-runtime`; setup uses `core`. |
| `product_version` | nonempty string / release version | Display and provenance only, not the compatibility predicate. |
| `setup_schema_version` | integer / external setup schema | Current selected setup identity must equal `3`. |

IsOS checks graph structure, exact pins and verified contributions before consuming the combined owner catalog. It then checks graph/type versions and required features against the selected setup reader, graph-reading runtime, and transport children. A `package-process-v1` contribution requires owner capability; the selected runtime may also require `managed-control-grants`, `managed-config-v1`, or `eos-gateway-client-v2` according to the admitted graph. A missing feature blocks before spawn. An owner declaring a feature does not prove readiness; health is checked separately. Diagnostics sort by canonical graph path, binary role, and feature ID.

When a selected package declares `compatibility.required_owner_capabilities`, IsOS compares each EI or EOS requirement against the exact selected owner's capability reply. IsOS-owned external node capability requirements currently fail closed until the IsOS owner exposes an equivalent capability surface. A missing capability rejects the selected pin before launch. The declared IsOS version range is checked when the release is verified; it does not substitute for a selected owner's capability proof.

<a id="verification--diagnostic-fields"></a>
### Diagnostic fields

| Field | JSON type / unit | Purpose and validation |
| --- | --- | --- |
| `code` | stable string / failure | One of `reader_identity_invalid`, `reader_graph_unsupported`, `reader_type_unsupported`, `reader_feature_unsupported`, `reader_binary_mismatch`, `reader_package_pin_mismatch`. |
| `path` | JSON Pointer / graph field | Exact selected type/package or `/version`; selected child errors may use `/settings/role`. |
| `message` | bounded string / repair text | Sanitized action; no raw stderr, local paths, or secrets. |
| `stage` | `graph`, `package`, `reader`, `feature` / phase | Identifies the failed admission step. |
| `reader.core`, `.binary`, `.binary_sha256` | strings and optional 64-hex / selected binary | Exact selected owner, executable identity, and byte provenance. |
| `reader.requested` | object / requested claim | Applicable graph version, full type, feature, and exact package pin; inapplicable values are null. |
| `reader.supported` | object / advertised claims | Sorted graph versions, type versions, full IDs, and features; untrusted claims are empty arrays. |

<a id="authentication"></a>
<a id="authentication--sign-in-for-package-publication"></a>
## Sign in for package publication

This page is for publishers using the IsOS CLI with Island-port. `isos auth login` uses GitHub OAuth Device Flow and exchanges a temporary GitHub user token for Island-port access and refresh tokens. It requests only `user:email`; the GitHub token is not written to disk. IsOS stores Island-port credentials in its config directory's `auth.json` with owner-only permissions. This login is separate from IsOS's [local HTTP request admission](../api/http.md#http_envelope).

Run `isos auth login` to start the device flow, `isos auth whoami` to inspect the signed-in account, and `isos auth logout` to revoke the refresh session and remove local credentials. Device Flow must be enabled by the Island-port service. Access credentials refresh after an API 401 and the replacement refresh token is persisted. Publication eligibility is decided by Island-port; IsOS does not acquire GitHub repository write credentials.

[中文](../../../../docs_cn/isos/interfaces/package/README_cn.md#authentication)
