# Task Plan: Re-analyze IDA Call Chains Then Fix Protocol Add Flow
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
重新梳理三个 IDA 实例（dllDPLogic/dllDPFrame/dllDPSource）中“添加协议/设备”调用链路，明确必要上下文参数；在此基础上制定修复方案并验证 MODBUSTCP_MASTER -> MODBUSSLAVE_TCP 的原子化流程。

## Current Phase
<!-- 
  WHAT: Which phase you're currently working on (e.g., "Phase 1", "Phase 3").
  WHY: Quick reference for where you are in the task. Update this as you progress.
-->
Phase 2

## Phases
<!-- 
  WHAT: Break your task into 3-7 logical phases. Each phase should be completable.
  WHY: Breaking work into phases prevents overwhelm and makes progress visible.
  WHEN: Update status after completing each phase: pending → in_progress → complete
-->

### Phase 1: IDA 调用链路梳理
<!-- 
  WHAT: Understand what needs to be done and gather initial information.
  WHY: Starting without understanding leads to wasted effort. This phase prevents that.
-->
- [x] 在 dll_DPFrame 中梳理协议添加链路（OnAddProcotol/OnAddGateWayProtocol/树插入/NameMap 更新）
- [x] 在 dllDPLogic 中梳理逻辑数据创建链路（OnMakeNewLogicData/MakeNewData/Link 依赖）
- [x] 在 dllDPSource 中梳理协议来源/配置加载链路（SourceContainer/Modbus 资源）
- [x] 汇总三模块调用顺序与关键入参
- **Status:** complete
<!-- 
  STATUS VALUES:
  - pending: Not started yet
  - in_progress: Currently working on this
  - complete: Finished this phase
-->

### Phase 2: 交叉验证与疑点确认
<!-- 
  WHAT: Decide how you'll approach the problem and what structure you'll use.
  WHY: Good planning prevents rework. Document decisions so you remember why you chose them.
-->
- [ ] 针对不确定的函数签名/参数依赖，准备 x32dbg/x64dbg 断点验证点
- [ ] 结合运行日志确认真实调用链与参数值
- **Status:** in_progress

### Phase 3: 修复方案与最小改动
<!-- 
  WHAT: Actually build/create/write the solution.
  WHY: This is where the work happens. Break into smaller sub-tasks if needed.
-->
- [ ] 选择最稳妥的创建入口（OnMakeNewLogicData vs OnAddProcotol）并补齐必要上下文
- [ ] 调整注入流程与日志，避免卡死与 UI 不一致
- **Status:** pending

### Phase 4: 验证与回归
<!-- 
  WHAT: Verify everything works and meets requirements.
  WHY: Catching issues early saves time. Document test results in progress.md.
-->
- [ ] 手工测试：先建 MODBUSTCP_MASTER 再建 MODBUSSLAVE_TCP
- [ ] 校验 Tree/NameMap/ID 映射一致性
- [ ] 记录验证结果到 progress.md
- **Status:** pending

### Phase 5: 交付
<!-- 
  WHAT: Final review and handoff to user.
  WHY: Ensures nothing is forgotten and deliverables are complete.
-->
- [ ] 总结调用链路与修复点，给出下一步建议
- **Status:** pending

## Key Questions
<!-- 
  WHAT: Important questions you need to answer during the task.
  WHY: These guide your research and decision-making. Answer them as you go.
  EXAMPLE: 
    1. Should tasks persist between sessions? (Yes - need file storage)
    2. What format for storing tasks? (JSON file)
-->
1. OnMakeNewLogicData 创建 MASTER 的最小必要上下文是什么（parent/link/control/desc/extraFlag）？
2. OnAddProcotol 与 OnMakeNewLogicData 的差异点（UI/NameMap/Tree 插入）？
3. dllDPSource 是否存在必须的协议配置加载步骤？

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
| 先做完整调用链路复盘再改代码 | 避免在参数不明时继续试错 |

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
| IDA decompile failed: CHWContainer::AddHolliTcpSlaveDevice (0x102B6E00) | 1 | Address is .rdata string; need real function entry via xrefs/name table |
| IDA decompile failed: CHWContainer::AddEthernetDev2MC (0x102B6DD7) | 1 | Address is .rdata string; need real function entry via xrefs/name table |
| C2713/C2712 (SEH __try in C++ function) | 1 | 移除 __try，改为直接调用并保留日志 |

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
