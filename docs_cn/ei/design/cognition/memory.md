# EI 记忆与召回

[English](../../../../docs/ei/design/cognition/memory.md) · [架构](../architecture.md) · [可选向量索引](../optional/memory.md)

记忆保存已准入观测、运行中的计划、已结束计划和人工审核后的参考计划，并为规划器提供有界召回。它不判断计划结果：规划器写入 `succeeded`、`failed`、`aborted`，只有操作员审核路径写入 `effective` 或 `ineffective`。记忆不调用模型，也不访问 `core/link`。决策层写入与检索，IO 的 `/review` 路径可以提升参考记录。

## 三层存储

- `EpisodicStore` 使用 SQLite 存放按 ID 去重的观测和完整 `PlanRecord`。观测列包括 `id`、时间、类别、来源、文本及 JSON 元数据；计划列包括计划/任务 ID、指令、结果、起止时间及完整 JSON。`append` 与 `save_plan` 对重复 ID 返回 `False`，关闭后的 `save_plan` 也返回 `False`，避免关闭竞态崩溃。`recent`、`get_plan`、`list_plans`、`search_plans` 提供读取和大小写不敏感的指令子串查询。
- `WorkingStore` 每个运行任务保存一条 `mission_id`、`plan_json`、`updated_at`，每完成一步即覆盖。带版本的 Mission JSON 才是重启恢复的权威状态。启动时先删除已由恢复检查点认领的工作行，避免将它们误判为中断计划。
- `ReferenceStore` 保存人工审阅后提炼的计划，包括计划 ID、指令、结果、审核时间、可选备注和完整记录。参考命中在召回排序中始终优先于普通情景记录。

## 召回入口

`RecallService(episodic, reference, index=None, docs=None)` 是规划器唯一的召回入口。`search_plans(order, limit)` 合并两库的关键词命中与可选向量索引命中，从存储重新载入完整记录；参考记录优先，按首次出现去重，再截到 `limit`。没有向量索引时仅用关键词。`search_docs(query, limit)` 从已载入文档语料返回 `(doc_id, excerpt)`，片段约 200 字符，通常从最早查询词前最多 50 字符开始；没有索引或语料时返回空列表。

`index_plan` 把已结束计划的指令和逐步结果编成向量文本；没有索引时为空操作。步骤的动作名以 `; ` 连接，无动作的最终步骤记作 `final`。规划器在用户消息中注入先前相似计划与相关文档的有界摘要；失败计划仍作为反例，空摘要直接省略。候选计划数量和文档数量均不超过 `recall_limit`（`0..16`）；指令超过 80 字符时截为前 77 字符加 `...`；每计划最多显示六步，多余步骤以省略标记表示。参考记录的审核备注会显示。

`load_docs_corpus` 递归读取语料目录中的 `.md` 和 `.txt`，以相对 POSIX 路径作为 ID。目录缺失或为空返回空映射，不阻止启动；配置非空但目录缺失时记录 `docs_corpus_missing`。当 `[launch] memory` 启用、`[memory] docs_corpus` 非空且向量索引可用时，启动阶段调用 `index_docs_corpus`；默认空路径关闭文档召回。

## 计划生命周期与恢复

任务开始前先生成召回摘要。每个已执行的原语步骤更新工作记录；每种结束路径都写入情景 `PlanRecord`、为后续召回建索引，并删除工作行。记录保留任务优先级、修订、明确的机器人目标和原语结果；未尝试的步骤不入记录。审核路径可以把已结束记录提升为参考计划。

`memory_consolidation_entry` 是受监管子系统：先标记 ready，随后每次观测准入变化时唤醒，按记录 ID 排空队列到情景库，再检查工作库。仍属于当前活动任务的行保留；其他过期行若尚无情景记录，则以 `aborted`、当前结束时间和 `crash_recovery` 注记保存，随后删除工作行。相同观测重复排空是幂等的；没有定时轮询。Stop 的 `finally` 关闭情景库和工作库，参考库由组合根在监管器停止后关闭。存储在关闭竞态期间不可用时，规划器记录 `plan_recall_failed` 并继续任务，不把召回故障当作动作证据。

## 验证

使用临时真实 SQLite 文件测试三层存储和幂等性；异步测试驱动真实准入与整合循环；规划器集成边界用桩模型和桩安全门，同时保留真实存储与规划器，检查摘要、恢复、结束记录和无索引降级。
