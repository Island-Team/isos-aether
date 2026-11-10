# U03 工作区面板契约

一个持久 EOS 工作台在 Graph/EOS 页面间保持控制、监控和视觉状态；一个日志消费者持有 P07 游标。停靠或收起时内容仍挂载；关闭控制面板时隐藏面板，同时在内存中保留本次运行的命令记录；关闭日志面板会卸载，重新打开时必须检查 IsOS 状态和当前范围。隐藏或收起的监控、视觉、日志面板停止轮询或流；隐藏控制面板暂停自动结果读取。所有数据仍按所有者运行和活动修订围栏。

**Stop 围栏解除。** 请求未完成时围栏适用于整个工作区，不受发起页面提供的运行 ID 影响。多个重叠的 Stop 请求须等待全部所有者响应。IsOS 返回 Stop 结果后，工作区重新读取所有者状态。只有新的已知运行 ID 与记录的页面、工作区及类型化终态运行 ID 不同时，或先观察到已停止／无运行状态，才可解除围栏。身份不可用、仍是已记录的运行或 Stop 尚未完成都不能解除。若旧身份未知，也没有已停止过渡或终态运行 ID，界面会继续围栏，直到能安全确认新运行。窄屏强制堆叠面板，禁用“浮动”；原先浮动的面板仍可停靠。

**Vision 来源显示已实现：** [Vision 来源显示契约](#vision-来源验证)为 Vision 面板增加运行／版本／stream／generation 围栏和实际字节摘要校验。隐藏或停止面板时清除当前证据，并阻止延迟帧重新绘制。Q03 final-17 与 Q04 final-5 回放分别在各自固定输入上观察到渲染画面；final-5 在回环地址模拟托管 Origin。

UI 只接受在当前已证明 stream 中严格大于上次显示值的 generation。仅凭二进制帧不得重置基线。新 stream 须经所选端点重新订阅握手，新 IsOS 运行须重新读取 process-status 并订阅。Stop 意图、运行/版本/选择变化、socket 关闭、隐藏/卸载或 process-status 丢失时，UI 撤销显示 generation token、关闭 socket、移除当前 JPEG，并分别显示 `stopped`、`stale`、`disconnected` 或 `evidence unavailable`。该 token 改变后，延迟的解码或摘要 Promise 不能重新绘制。畸形、重复、较旧、错误版本/来源/gate/stream 帧被拒绝，不推进显示 generation；须显示带稳定本地原因的明确**帧被拒绝**诊断，不能显示空白成功帧。健康 stream 上的 `no new frame` 与拒绝、断开、缺少绑定证据区分。EOS 停机后不得发送缓存帧。

## Vision 来源验证

解码器现将元数据限制为 4096 字节，在显示前校验所选所有者字段与 JPEG 摘要。面板显示所选来源、gate、运行、stream、generation、摘要、订阅上限、实际帧率及本地观察到的跳过代次；不把它们称作 EOS 路由器的全局限频或丢帧总数。聚焦测试覆盖成功和拒绝决策，以及挂载后的 Stop／延迟摘要边界。所选版本的浏览器回放仍属 Q01 验收工作。此显示没有图像上传、提示或执行器路径。

**历史设计轮验证，2026-11-02：** `npm ci`、`npm test`（179 项通过）及 `npm run build`（本地与托管）通过。变更页面的本地链接只读检查和 `git diff --check` 通过。完整输出在忽略的 `data/test/20261102-q01-camera-design/`。这些检查验证当时的 UI 基线；后续实现证据见下文。

### 公共浏览器端点

固定的 [IsosUI Q01 用例](../../../ui/tests/fixtures/q01_vision_display/README_cn.md) 覆盖有效显示、首帧前握手、重复／较旧 generation、错误版本／来源／stream、摘要失败、缺少选择和停止后的延迟结果。它们是设计 fixture，不是浏览器回放证据。

**契约修订验证，2026-11-02：** `python3 tests/fixtures/q01_vision_display/check_cases.py` 校验了 12 个固定用例。`npm test`（179 项通过）、`npm run build`（本地与托管）及 `git diff --check` 通过。变更页面的链接通过只读检查。完整命令输出在忽略的 `data/test/20261102-q01-contract-repair/`；不声称已完成现场浏览器帧回放。

**历史实现验证：** `npm test`、本地与托管 `npm run build`、fixture 结构检查及 `git diff --check` 的完整输出保存在忽略目录 `data/test/20261103-q01-vision-ui/`。该阶段的挂载测试使用 EOS socket stub；此记录不声称已完成跨所有者 Q01 回放。

**摘要乱序修复，2026-11-03：** 面板在 SHA-256 完成后、替换图像前再次比较当前代次。挂载测试先完成代次 2、后完成代次 1，确认显示保持为代次 2。`npm test` 的 41 个文件共 186 项通过；`npm run build` 的 TypeScript、本地与托管包及 manifest 校验通过；12 个设计 fixture 的结构检查和 `git diff --check` 通过。完整输出位于忽略目录 `data/test/20261103-q01-vision-ui-race-repair/`。
