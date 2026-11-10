# Write and validate a node

[中文](../../../../docs_cn/isos/interfaces/node/authoring_cn.md) · [Profile](profile.md) · [Type](type.md)

Start from the [current demo graph](../../../../config/graph-pool/demo.nodes.json). Graph format `0.2.0` requires exactly `isos`, `ei`, and `eos` cores. Each registered ID follows `<core>.<layer>.<group>.<type>@v0.2`; place its instance under that same core, layer, and group. The demo contains 16 registered node instances: eight for control and eight for observation.

1. Implement and test the EI or EOS owner interface and lifecycle before registering a wrapper. A wrapper declares the interface's place in the graph; it does not duplicate owner logic.
2. Register the type in its owning core. Add ports only for implemented graph delivery routes; health-only profiles need no ports.
3. Add an instance with matching map key and `id`, allowed settings, and any required input references. Use `<core>/<layer>/<group>/<id>` paths.
4. Validate and Apply the graph at its saved revision. Run launches the EI and EOS owner processes. Inspect each process's health, then explicitly Stop the deployment.

The demo declares typed control and observation edges. One activation travels through EI to EOS; EOS converts it to a bounded six-joint delta for the mock adapter. The same adapter's observation returns through EOS and EI to the UI monitor asynchronously. Unknown types, wrong placement, missing required inputs, and incompatible ports fail admission. The mock adapter has no physical actuator route.
