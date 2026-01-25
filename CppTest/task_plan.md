# Task Plan: AutoThink 硬件管理方式分析
<!-- 
  WHAT: This is your roadmap for the entire task. Think of it as your "working memory on disk."
  WHY: After 50+ tool calls, your original goals can get forgotten. This file keeps them fresh.
  WHEN: Create this FIRST, before starting any work. Update after each phase completes.
-->

## Goal
<!-- 
  WHAT: One clear sentence describing what you're trying to achieve.
  WHY: This is your north star. Re-reading this keeps you focused on the end state.
  EXAMPLE: "Create a Python CLI todo app with add, list, and delete functionality."
-->
基于 MCP/IDA 与现有文档，分析 AutoThink 硬件管理方式与关键函数链路，并将结论整理到 `Docs/硬件自动化组态规则.md`。

## Current Phase
<!-- 
  WHAT: Which phase you're currently working on (e.g., "Phase 1", "Phase 3").
  WHY: Quick reference for where you are in the task. Update this as you progress.
-->
Phase 9

## Phases
<!-- 
  WHAT: Break your task into 3-7 logical phases. Each phase should be completable.
  WHY: Breaking work into phases prevents overwhelm and makes progress visible.
  WHEN: Update status after completing each phase: pending → in_progress → complete
-->

### Phase 1: Requirements & Discovery
<!-- 
  WHAT: Understand what needs to be done and gather initial information.
  WHY: Starting without understanding leads to wasted effort. This phase prevents that.
-->
- [x] 确认分析范围与目标文档结构
- [x] 复核关键模块与函数链（Logic/Frame/Source）
- [x] 更新 findings.md
- **Status:** complete
<!-- 
  STATUS VALUES:
  - pending: Not started yet
  - in_progress: Currently working on this
  - complete: Finished this phase
-->

### Phase 2: Planning & Structure
<!-- 
  WHAT: Decide how you'll approach the problem and what structure you'll use.
  WHY: Good planning prevents rework. Document decisions so you remember why you chose them.
-->
- [x] 定义需要补齐的“硬件管理方式”要点
- [x] 规划哪些函数/调用链需验证
- [x] 记录关键决策
- **Status:** complete

### Phase 3: Implementation
<!-- 
  WHAT: Actually build/create/write the solution.
  WHY: This is where the work happens. Break into smaller sub-tasks if needed.
-->
- [x] 通过 MCP 获取反编译/交叉引用/字符串证据
- [x] 将分析结果写入 `Docs/硬件自动化组态规则.md`
- [x] 同步 findings.md
- **Status:** complete

### Phase 4: Testing & Verification
<!-- 
  WHAT: Verify everything works and meets requirements.
  WHY: Catching issues early saves time. Document test results in progress.md.
-->
- [x] 对照 IDA 数据检查文档内容一致性
- [x] 在 progress.md 记录验证步骤
- [x] 标注仍待验证的点
- **Status:** complete

### Phase 5: Delivery
<!-- 
  WHAT: Final review and handoff to user.
  WHY: Ensures nothing is forgotten and deliverables are complete.
-->
- [x] 回顾文档与日志
- [x] 确保交付完整
- [x] 提供总结与下一步建议
- **Status:** complete

### Phase 6: Re-analysis & Stabilization
<!-- 
  WHAT: Re-check UI entry points and stabilize the automation approach.
  WHY: Current low-level resolver is unstable; need a safer path.
-->
- [x] 复核 UI 层 Add 相关入口与参数路径（OnAddSlave/OnDPTree_*）
- [x] 识别最稳定的“自动添加”入口与上下文获取方式
- [x] 形成替代方案与验证步骤（不修改代码）
- **Status:** complete

### Phase 7: Code Update & Validation
<!-- 
  WHAT: Implement stable parameter model in HwHack and validate.
-->
- [x] 修正 `OnMakeNewLogicData_Slave` 参数顺序与 typeName
- [x] 引入 Frame 侧逻辑 ID -> Parent 映射
- [x] 增加 NameMap/LinkByRaw 解析以命中 MASTER 树节点
- [x] 确认 `OnMakeNewLogicData_Slave` 第 3 参数为重复创建标志并修正传参
- [x] 启用 `OnSlaveOperate` UI 插入路径并禁用 `OnDPTree` 回退，避免重复插入
- [x] 增加 TreeView SendMessageTimeout 防止 DumpTreeChildren 卡住
- [x] 默认关闭注入后 TreeDump（避免 UI 线程卡死）
- [x] 将 TreeDump 移出注入路径，注入后延迟定时输出
- [x] 关闭设备信息探测与 Link->Comm 探测（避免 UI 线程阻塞）
- [x] 优先走 OnAddSlave UI 入口，失败才回退低层注入
- [x] 修正 OnAddSlave 传参与 count/extra
- [x] 重新编译并在 AutoThink 运行验证
- **Status:** complete

### Phase 8: UI Refresh Deep Dive & Doc Update
<!-- 
  WHAT: Validate UI refresh paths and document root cause/fix options.
-->
- [x] 反编译 UI 刷新相关入口（OnAddSlave/RefreshDPTreeForAdd/OnSlave_Operate 等）
- [x] 将 UI 刷新与映射更新结论补充到 `Docs/硬件自动化组态规则.md`
- **Status:** complete

### Phase 9: Git Push & Handoff
<!-- 
  WHAT: Stage, commit, and push the validated fix with clear notes.
-->
- [ ] 复核 CppTest 变更清单并选择需要提交的文件
- [ ] 提交带详细说明的 commit
- [ ] 推送到远端
- **Status:** in_progress

## Key Questions
<!-- 
  WHAT: Important questions you need to answer during the task.
  WHY: These guide your research and decision-making. Answer them as you go.
  EXAMPLE: 
    1. Should tasks persist between sessions? (Yes - need file storage)
    2. What format for storing tasks? (JSON file)
-->
1. 全局实例指针的真实类型与获取路径是什么？
2. Link 指针在内部的来源/表结构如何定位？
3. 数据层（CHWDataContainer）与视图层（树控件/Frame）如何衔接刷新？

## Decisions Made
<!-- 
  WHAT: Technical and design decisions you've made, with the reasoning behind them.
  WHY: You'll forget why you made choices. This table helps you remember and justify decisions.
  WHEN: Update whenever you make a significant choice (technology, approach, structure).
  EXAMPLE:
    | Use JSON for storage | Simple, human-readable, built-in Python support |
-->
| Decision | Rationale |
|----------|-----------|
|          |           |

## Errors Encountered
<!-- 
  WHAT: Every error you encounter, what attempt number it was, and how you resolved it.
  WHY: Logging errors prevents repeating the same mistakes. This is critical for learning.
  WHEN: Add immediately when an error occurs, even if you fix it quickly.
  EXAMPLE:
    | FileNotFoundError | 1 | Check if file exists, create empty list if not |
    | JSONDecodeError | 2 | Handle empty file case explicitly |
-->
| Error | Attempt | Resolution |
|-------|---------|------------|
| Get-ChildItem -Filter with array failed | 1 | Use Test-Path or separate Get-ChildItem calls |
| Missing path `Docs/本地相关工具及MCP` | 1 | Use `Docs/本地相关工具及MCP.md` |
| `lookup_funcs` query not found (newline list) | 1 | Use comma-separated queries or address lookup |
| RVAs 0xDB560/0x117830 not functions in current IDB | 1 | Locate via name/xrefs or adjust for version |
| `session-catchup.py` path missing | 1 | Use `.codex` path and rerun script |
| MCP ida-pro-mcp-1 disasm request failed (connection error) | 3 | Defer IDA verification; ask user to confirm MCP port 13338 health |
| ResolveContext exception at stage=map_get_device | 1 | Restore `sub_10045E80` __thiscall with `pContainer+0x250` as this |

## Notes
<!-- 
  REMINDERS:
  - Update phase status as you progress: pending → in_progress → complete
  - Re-read this plan before major decisions (attention manipulation)
  - Log ALL errors - they help avoid repetition
  - Never repeat a failed action - mutate your approach instead
-->
- Update phase status as you progress: pending → in_progress → complete
- Re-read this plan before major decisions (attention manipulation)
- Log ALL errors - they help avoid repetition
