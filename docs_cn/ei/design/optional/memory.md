# 可选向量记忆索引

[English](../../../../docs/ei/design/optional/memory.md) · [核心记忆](../cognition/memory.md)

`SqliteVecIndex` 为核心记忆提供可选的 sqlite-vec 检索。缺少 `vec` extra 或导入失败时，构建返回 `None`：计划检索退化为关键词模式，文档检索关闭，任务仍可运行。索引不判断计划成败，也不获得模型或硬件权限。

## 存储与 API

索引数据库用 `plans_ids` 和 `documents_ids` 把调用者提供的字符串 ID 映射到整数 rowid；`plans_vec` 与 `documents_vec` 是 `vec0(embedding float[dim])` 虚拟表。`SqliteVecIndex(path, dim=256)` 创建或打开数据库并加载扩展。`index_plan`、`index_document` 添加或替换条目；`search_plans`、`search_documents` 返回距离最近的 ID，`limit <= 0` 返回空列表；`close` 释放连接。`embed_text` 是可测试的确定性嵌入函数，默认维度为 256。缺少扩展时导入错误明确指出 `vec` extra 及降级方式。

调用者提交文本；文档全文索引，计划文本由任务指令及每步的 `primitive action(s) -> outcome` 行构成，无动作的最终步骤写作 `final -> outcome`。文档检索不依赖持久计划格式。

## 查询和替换

离线 MVP 的嵌入器把小写字母数字 token (`[a-z0-9]+`) 用 SHA-256 定位并赋予正负符号，再做 L2 归一化。查询用相同嵌入器，经 `sqlite_vec.serialize_float32` 序列化，关联 `*_vec` 与 `*_ids`，按向量距离取最近条目。单位向量的 L2 距离排序与余弦相似度等价。

重建已有 ID 时，索引删除旧向量与映射行，插入新映射并用其 rowid 写入向量；不会积累陈旧条目。无索引时 `RecallService.index_plan` 是空操作，关键词检索仍工作，文档片段返回空列表。

## 验证

真实 sqlite-vec 临时数据库测试覆盖 token 大小写、确定性、归一化、无 token 零向量、KNN 排序和替换；规划器桩集成验证参考计划优先、跨来源去重、文档片段和任务结束后索引。缺少 extra 时仅跳过依赖 vec 的测试；无索引降级与步进计划文本渲染仍应测试。
