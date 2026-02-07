# Task Plan: [Brief Description]
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
Document core business logic across PlcGen and PLC_ANA_WINFORM, then redesign and implement a new template-management subsystem in `plc_templates` aligned with the latest architecture.

## Current Phase
<!-- 
  WHAT: Which phase you're currently working on (e.g., "Phase 1", "Phase 3").
  WHY: Quick reference for where you are in the task. Update this as you progress.
-->
Phase 3

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
- [x] Understand user intent
- [x] Identify constraints and requirements
- [x] Document findings in findings.md
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
- [x] Define technical approach for template management in PlcGen
- [x] Compare legacy template workflow vs current architecture and map responsibilities
- [x] Document decisions with rationale
- **Status:** complete

### Phase 3: Implementation
<!-- 
  WHAT: Actually build/create/write the solution.
  WHY: This is where the work happens. Break into smaller sub-tasks if needed.
-->
- [ ] Implement new template management in `plc_templates` and supporting crates
- [ ] Wire interfaces into `plc_core` application/ports if needed
- [ ] Add minimal tests or validation helpers
- **Status:** pending

### Phase 4: Testing & Verification
<!-- 
  WHAT: Verify everything works and meets requirements.
  WHY: Catching issues early saves time. Document test results in progress.md.
-->
- [ ] Verify design matches requirements and legacy behavior where needed
- [ ] Document test/verification results in progress.md
- [ ] Fix any issues found
- **Status:** pending

### Phase 5: Delivery
<!-- 
  WHAT: Final review and handoff to user.
  WHY: Ensures nothing is forgotten and deliverables are complete.
-->
- [ ] Review all output files
- [ ] Ensure deliverables are complete
- [ ] Deliver to user
- **Status:** pending

## Key Questions
<!-- 
  WHAT: Important questions you need to answer during the task.
  WHY: These guide your research and decision-making. Answer them as you go.
  EXAMPLE: 
    1. Should tasks persist between sessions? (Yes - need file storage)
    2. What format for storing tasks? (JSON file)
-->
1. What responsibilities must the new template manager cover (storage, render, binary/JSON snapshots, variable handling)?
2. How should template data be represented and versioned in PlcGen?
3. Where does template management integrate with `plc_core`/`plc_templates` boundaries?
4. Which mutable fields are extracted from full POU JSON, and what fixed JSON schema should the UI configure?
5. Which validation rules stay brand-specific (adapter) vs. generic (core) for atomic POU operations?
6. How will template discovery/registration be implemented per PLC brand (scan API + folder layout)?
7. What is the render output contract (e.g., JSON patch/DSL) that maps Tera output to POU edits?
8. How will the “free-form builder” atomic ops and catalogs (variables/blocks/elements) be separated from template-based edits?
9. How will a single-POU template expand into N repeated units (including multi-rung/variable sets) while still producing one final POU?
10. What is the final multi-level template classification scheme for storage/discovery and UI navigation?
11. How should hierarchical initial values be represented (tree vs. path list vs. full snapshot reuse)?
12. Which top-level POU header fields (beyond name/header_strings) must be editable for template generation?
13. How should network ID regeneration be defined (adapter-specific rule TBD)?
14. Should Safety topology tokens remain opaque (pass-through) during template edits?

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
| Patch 输出采用“路径 + 多字段匹配”的结构化操作列表 | 便于与模板渲染/Tera 对接，同时避免仅靠路径造成歧义 |
| 初始值字段保留完整信息结构（后续实现） | 需要支持元素库维护与从 POU 解析回填 |
| 自由构造原子操作独立为 `plc_pou_builder` | 降低与模板链路耦合，便于阶段交付 |
| 模板分类固定 3 级（品牌/系列/型号），其下为可变多级目录 | 满足分类需求且保持品牌适配器可控扫描范围 |
| 分类以目录路径为准，UI 负责固定三级选择 + 自定义后续分类 | 用户不直接接触文件/目录，体验一致 |
| 初始值采用树结构存储 | 支持层级变量初始值表达，与解析结构对齐 |
| 模板扩展保留原集合并追加克隆 | 与现有模板使用习惯一致 |
| 模板元信息记录 variant/serialize_version/format_name | 保证解码/编码一致性校验 |
| 冲突（网络 ID/变量名）由执行器自动处理 | 降低前端配置复杂度，规则后续落地 |
| Patch 执行顺序为“先修改原模板，再扩展复制” | 顶层 POU 头数据需先统一，再做实例扩展 |
| 变量定位使用变量树路径 | 层级变量是稳定锚点 |
| 元件定位使用梯级树 + 多字段匹配 | 与现有 JSON 结构一致 |
| Pin 绑定允许为空 | 兼容模板中未绑定情况 |
| 顶层头字段固定不编辑 | 稳定结构，减少模板复杂度 |
| 网络 ID 扩展临时采用 max_id+1 | 在规则明确前保持一致性 |
| 模板选择来自点表设备分类表，IO 模板固定 | 保证模板来源一致且自动化 |
| plc_logic_gen 为统一入口并生成第三方通讯设备点表 | 统一编排入口，扩展通讯场景 |
| 每个模板输出一个 POU | 当前阶段不做聚合 |

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
| session-catchup script missing at `$env:USERPROFILE\.claude\skills\planning-with-files\scripts\session-catchup.py` | 1 | Logged in findings/progress; continued with current session state |
| `plc_pou_builder` directory not found | 1 | Documented as planned crate; will proceed with design-level interactions |

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
