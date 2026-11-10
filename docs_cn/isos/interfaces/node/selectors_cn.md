# 设置与选择器

本页供类型发布者和 UI 作者使用。IsOS 当前依据注册类型验证实例设置，并从声明的来源投影选择器选项。选择器是设置声明，不是软件包专属表单或请求时的驱动参数。[资源](../packages/resources_cn.md)解释经过验证的软件包选项。

| 字段 | 类型／单位 | 含义与验证 | 缺失／默认值 |
| --- | --- | --- | --- |
| `name` | 非空字符串／设置 ID | 在一个注册类型内唯一。 | 必填。 |
| `data_type` | 枚举／JSON 值类别 | `string`、`number`、`integer`、`boolean`、`array`、`object`；当前别名还包括 `string_array`、`string_table`、`positive_integer`、`nonnegative_integer`。 | 必填。 |
| `required` | 布尔值／存在规则 | 必填实例值缺失时，需要类型声明的默认值。 | 必填。 |
| `description` | 非空字符串／展示文字 | 告诉作者选择什么及其单位。 | 必填。 |
| `unit` | 非空字符串／物理或展示单位 | 说明数值或文本如何度量。 | 可选。 |
| `min`、`max` | 有限数字／设置单位 | 限定数值实例；`min` 不得大于 `max`。 | 可选；无对应边界。 |
| `default` | 与 `data_type` 匹配的值 | 填补省略的实例值。 | 可选；无默认值。 |
| `choices` | 非空匹配值数组 | 限定合法选项。 | 可选；无有限列表。 |
| `selector.provider` | 非空字符串／选项提供者 | 指定 `package`、`graph`、`owner` 或本地 `ui` 选项来源。`owner` 从选中的 EOS 或 EI 设置服务取得上下文选项；`ui` 在浏览器本地提供选项。 | 存在 `selector` 时必填。 |
| `selector.resource` | 非空字符串／资源类别 | 指定请求的选项类型。 | 存在 `selector` 时必填。 |
| `selector.package_from` | 非空字符串／设置引用 | 选择提供选项的精确软件包锁定。 | 可选。 |
| `selector.filter` | 对象／提供者过滤条件 | 不解释归属方负载而限制选项。 | 可选；空过滤条件。 |

IsosUI 根据声明渲染提供者选项。图、软件包和归属方选项只是预览；Apply 会再次验证，避免过期选择获准。归属方未返回选项时，选择器被阻止，不允许输入未经验证的文本。按键或组合键说明固定节点设置：它属于实例配置，而发出的信号属于输出端口单独声明的负载契约。本次更改不启用浏览器按键选择器或键盘控制路由。

[English](../../../../docs/isos/interfaces/node/selectors.md)
