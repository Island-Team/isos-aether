# Shared node documentation

<!-- doc-navigation:start -->
<details>
<summary>Folder: node</summary>

Browse documents and subfolders here.

- [Parent folder](../README.md)
- Subfolders:
  - [interfaces/](interfaces/README.md)

</details>
<!-- doc-navigation:end -->

[中文](../../docs_cn/node/README_cn.md) · [Documentation](../README.md)

The [shared node interfaces](interfaces/README.md) define graph placement, registered types, ports, data descriptors, edges, and registry admission for IsOS, EI, and EOS. [IsOS node groups](../isos/interfaces/node/README.md) define the IsOS-owned UI nodes; EI and EOS own their execution semantics in their respective component interfaces.

## Documentation verification

From the repository root, run `python3 scripts/check_doc_links.py --all`, `python3 crates/isos/tools/check_isos_docs.py`, and the focused reader-feature, R03, R07, P06, and UI fixture scripts. They passed on 2026-11-16; complete output is in ignored `data/test/20261116-shared-node-interfaces/checks.log`.
