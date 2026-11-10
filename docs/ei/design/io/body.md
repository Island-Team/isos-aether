# EOS body discovery

[中文](../../../../docs_cn/ei/design/io/body.md) · [Southbound overview](eos-exchange.md)


The robot descriptor carries `id`, `name`, `model`, `description`, operational `state`, `health`, emergency-stop state, optional power percentage, and robot-scoped primitive bindings. A robot-scoped binding, including `robot.stop` or `safety.estop`, is called with an empty device ID.

Each device descriptor carries `id`, `class`, `part`, `description`, `state`, `health`, and device-scoped primitive bindings. Backend names, driver versions, endpoints, raw telemetry, configuration internals, and media do not cross this boundary.


Discovery failures do not replace the last complete generation. Successful rediscovery removes vanished devices, resources, skills, and primitive targets together. The monitor uses `GetStatus.started_at` as the EOS process-generation identity and performs bounded rediscovery so changed health and bindings cannot remain stale.
