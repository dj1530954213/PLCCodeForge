# Findings & Decisions
<!-- 
  WHAT: Your knowledge base for the task. Stores everything you discover and decide.
  WHY: Context windows are limited. This file is your "external memory" - persistent and unlimited.
  WHEN: Update after ANY discovery, especially after 2 view/browser/search operations (2-Action Rule).
-->

## Requirements
<!-- 
  WHAT: What the user asked for, broken down into specific requirements.
  WHY: Keeps requirements visible so you don't forget what you're building.
  WHEN: Fill this in during Phase 1 (Requirements & Discovery).
  EXAMPLE:
    - Command-line interface
    - Add tasks
    - List all tasks
    - Delete tasks
    - Python implementation
-->
<!-- Captured from user request -->
- Review current project (PLCCodeForge/PlcGen) to understand overall architecture.
- Review previous project at `C:\Program Files\Git\code\PLC_OPEN\PLC_HANDLE\PLC_ANA_WINFORM`.
- Summarize core business logic from both projects (focus on template management).
- Re-design template management aligned to the latest architecture and functions.
- Implement and adjust template management changes after analysis.
- New requirement: template creation is for a complete POU (ladder + variables together), not separate ladder/variable templates.
- New requirement: template creation reads Windows custom clipboard → parse via `plc_core` → produce full JSON (current test outputs), then extract mutable fields in template crate.
- Extracted mutable fields include: POU name, variable instance names, block names, coil names, contact names, and block pin binding variable names; initial values must be designed now (interface reserved) but not implemented yet.
- Store extracted mutable fields in a fixed JSON format for application-layer deserialization and Tauri UI configuration.
- Template registration/discovery is adapter-specific: full template bundle stored in one folder; adapters implement scan/discovery to hide non-standard PLC differences.
- Template rendering output depends on `plc_core` generator; goal is atomic operations on POU with strict validation (invalid outputs must error) and brand-specific rules likely in adapters.
- Point-table import logic should align with `PLC_AUTO_CODE` core flow, optimized for Rust style/architecture.
- Overall system consists of PLC auto programming, UIA automation, and 485/TCP comms configuration modules orchestrated by a core layer.
- New requirement: template rendering must use Tera; scripts hold rules, while Tauri UI provides friendly configuration for script parameters.
- Atomic operations split into two paths: (1) template-based edits on a base POU (binary+JSON), requiring render output to map to structured edits; (2) free-form interlock logic built from scratch via atomic builder ops (add rung/coil/block, bind pins, add variables).
- Free-form path requires libraries/catalogs for variables, function blocks, and elements (distinguish official vs. custom); this is a next-phase need but should be considered in current design.
- Decision inputs: patch output should be path-based with additional composite identifiers for disambiguation; initial value must store full info; free-form builder will live in a separate `plc_pou_builder` crate.
- New requirement: a template is created from a single full POU, but runtime use must expand it into multiple similar rungs/locals/instances (e.g., 1 AI_CONVERT → 20 instances), producing one final POU (not multiple fragments).
- New requirement: some templates contain multiple rungs and variables as a whole set; runtime should render and generate multiple copies of this set within the same final POU.
- New requirement: template storage/discovery must support multi-level classification paths (e.g., "和利时/和利时普通型/IO映射/AI映射" or "和利时/和利时普通型/执行机构/阀门/ESD阀门/ESD阀门带手动按钮").
- New requirement: initial value structure must support hierarchical variables (multi-level child fields), not flat-only.
- Decision: classification uses directory path; first 3 levels are fixed (brand/series/model), deeper levels are flexible and scanned by brand adapters.
- Decision: UI uses cascader for fixed first 3 levels and user-defined deeper categories; user does not touch files/dirs directly.
- Decision: initial value stored as a tree aligned to variable hierarchy; node fields follow parser output (details adjustable later).
- Decision: `template.meta.json` must record variant/serialize_version/format_name for consistency checks.
- Decision: template expansion keeps the original template set as instance 1, then appends clones.
- Decision: network ID and variable name conflicts are auto-handled (rules to be finalized later).
- Decision: patch execution order is "modify base template first, then expand" to ensure top-level POU data is fixed before cloning.
- Decision: variable anchoring uses VariableNode tree path; element anchoring uses network/element tree with multi-field match; pin bindings may be empty.
- Decision: top-level POU header fields beyond name/header_strings are fixed and not editable.
- Decision: temporary network ID regeneration uses "max_id + 1" during expansion (subject to future brand rules).
- Decision: templates are selected from device classification table in point sheets; UI does not directly choose templates.
- Decision: IO mapping templates are fixed for AI/AO/DI/DO; device templates use category_path from classification.
- Decision: plc_logic_gen generates additional third-party comm device point tables and is the unified entry (can route to plc_pou_builder).
- Decision: each template produces one POU; no aggregation for now.
- New requirement: ladder/atomic operation rules must be verified against the configuration software's own definitions to ensure behavioral consistency.
- `plc_logic_gen/src/lib.rs` is currently a stub (only `add` + test).
- `plc_pou_builder` crate does not yet exist in repo; planned as a future module for atomic construction.
- Open: Safety topology tokens are treated as opaque/pass-through unless brand adapter provides editing rules.

## Research Findings
<!-- 
  WHAT: Key discoveries from web searches, documentation reading, or exploration.
  WHY: Multimodal content (images, browser results) doesn't persist. Write it down immediately.
  WHEN: After EVERY 2 view/browser/search operations, update this section (2-Action Rule).
  EXAMPLE:
    - Python's argparse module supports subcommands for clean CLI design
    - JSON module handles file persistence easily
    - Standard pattern: python script.py <command> [args]
-->
<!-- Key discoveries during exploration -->
- Current repo appears Rust-based with multiple crates: `plc_core`, `plc_importer`, `plc_logic_gen`, `plc_templates`, plus `plc_core_tester`.
- Cargo manifests found for: `plc_core`, `plc_core_tester`, `plc_importer`, `plc_logic_gen`, `plc_templates` (no root workspace manifest in repo root).
- Repo root contains `config/`, `Docs/`, crate directories, and IDE metadata; no README found at root.
- Template-related crate likely `plc_templates` with `plc_templates/src/lib.rs`.
- Core domain and adapter layers exist in `plc_core/src/domain` and `plc_core/src/adapters`.
- `plc_templates/src/lib.rs` is currently the default Cargo stub (simple `add` + test), so template logic not yet implemented here.
- `plc_core` is the core POU codec: parse/serialize POU binary to/from domain AST; adapters handle concrete PLC variants.
- `PouService` (application layer) wraps a `PouCodec` to decode/encode clipboard bytes with basic validation.
- `PouService` only validates non-empty POU name; `PouCodec` port trait defines `decode/encode/format_name` boundaries and keeps clipboard IO out of core.
- Domain AST (`plc_core/src/domain/ast.rs`) defines `UniversalPou` (name, header_strings, variables, networks), `VariableNode` tree, ladder `Network` with `LdElement`s, and Safety topology tokens.
- `plc_core/src/domain/hardware.rs` is currently a placeholder (no implementation yet).
- Hollysys adapter exports `HollysysCodec` implementing `PouCodec` with Normal/Safety variants; decode via parser, encode via serializer, and exposes clipboard format names.
- Hollysys parser builds `UniversalPou` by reading header, networks, and variables; Safety reads header string array; Normal/Safety differ in element formats and sequencing.
- Variable organization in parser uses `config/symbols_config.json` to group variables into `VariableNode` groups (matched header strings or dot-notation prefixes).
- `config/symbols_config.json` defines function block member lists used for variable grouping (e.g., AI_ALARM_IO_PLC, PID_CTRL, MOV_CTRL, etc.).
- Serializer writes MFC/GBK binary with variant-specific header + network + variable sections; Normal networks require connection graphs, Safety can use topology tokens.
- `plc_logic_gen` and `plc_importer` crates are still default Cargo stubs (only `add` + test in `src/lib.rs`).
- `plc_importer/src/lib.rs` in PlcGen is currently a stub (only `add` + test).
- In PLC_AUTO_CODE, `ImportResult` is defined in `core/src/model/table.rs`.
- `PLC_AUTO_CODE` `ImportResult` includes `points: HashMap<String, PlcPoint>`, `device_group: Vec<DeviceGroup>`, and `point_order` to preserve IO row order during generation.
- `Docs/第二轮核对的全局规则.md` currently documents low-level parse/serialize fields (e.g., `contact_flag`, `box_flag`, `binding_id`) but does not define ladder semantics such as NO/NC, series/parallel conditions, or FB instance vs non-instance rules.
- `Docs/解析器底层的相关规则.md` notes that Contact has a single byte distinguishing NO/NC, but does not map the byte values explicitly.
- `plc_core/src/domain/ast.rs` defines `LdElement.sub_type` with explicit semantics: `0 = 常开/普通线圈`, `1 = 常闭/取反线圈`.
- `Docs/rule-map.md` confirms contact/coil type ids and presence of contact-specific trailing flag byte in Normal/Safety, but the flag → semantics mapping is still TBD beyond NO/NC.
- `plc_core/src/adapters/hollysys/parser/mod.rs` reads Box `instance` as a separate string after a flag; pins are explicit (name + variable) and input pins in Normal read a `binding_id` when `serialize_version >= 13`.
- `plc_core/src/adapters/hollysys/parser/mod.rs` reads Contact/Coil `sub_type` as a single byte and stores it in `LdElement.sub_type`; additional flags are ignored for semantics.
- `Docs/第二轮核对的全局规则.md` contains an extended “语义编译层（CalcLogic）” section describing ConnRefs graph → tagHccTreeNode tree and linkType semantics (expression vs statement chain), which should guide how builder `connect` operations are modeled.
- Architecture doc proposes layered protocol reproduction (L0 byte primitives, L1 object boundaries, L2 object structures, L3 semantic/engineering layer) and a codec/model/parser/generator/api module split.
- Docs note Normal clipboard format name `POU_TREE_Clipboard_PLC` and core serialization in AutoThink `CPOU::Serialize(CArchive*)`.
- `plc_core` exposes a `PouCodec` port trait (decode/encode/format_name) for adapters and upper-layer integration.
- `plc_core` depends on `anyhow`, `thiserror`, `serde`, `byteorder`, `encoding_rs`, `binrw`, `log`, `serde_json`; lib.rs re-exports `PouService`, `PouCodec`, and Hollysys adapter types.
- `plc_core_tester` is a CLI harness that reads hex dump `.md` samples from `Docs/样本对比/测试用例`, decodes with Hollysys codec (normal/safety), and writes JSON summaries to `parsed_out`.
- `plc_core/src/domain/ast.rs` defines `UniversalPou` with `variables: Vec<VariableNode>` (tree structure) and `Variable` fields including `init_value` string plus metadata (soe_enable, power_down_keep, comment, var_id/addr_id/mode/id2/area_code).
- `plc_core/src` contains `adapters/`, `application/`, `domain/`, `ports/`, `lib.rs`, and `symbols_config.rs`.
- `plc_core/src/ports` only defines the `PouCodec` port (backend.rs) and re-exports it in mod.rs.
- `plc_core/src/adapters` currently only contains the `hollysys` adapter module.
- `plc_core/src/adapters/hollysys` exposes `HollysysCodec`, `HollysysConfig`, `PlcVariant`, and parser entrypoints (`read_pou`, `read_pou_with_config`).
- `HollysysCodec` implements `PouCodec` by calling parser `read_pou_with_config` and serializer `PouSerializer::serialize`, and returns clipboard format names by variant.
- `HollysysConfig` centralizes variant, total length, and serialize version (normal/safety defaults: len 0x2000, version 13).
- `symbols_config.rs` loads `config/symbols_config.json` and provides a lookup map of function block members for variable grouping.
- Hollysys parser groups flat variables into `VariableNode` trees using header strings and dot-prefix grouping, then wraps remaining variables under `Local Variables`.
- Sample JSON (`Docs/样本对比/测试用例/parsed_out/普通型样本1.md_normal.json`) top-level keys are `name`, `header_strings`, `variables`, `networks`; network elements include `type_code/name/instance/pins`.
- Sample variable nodes are group objects (`name`, `children`); leaf nodes include `name/data_type/init_value/soe_enable/power_down_keep/comment/var_id/addr_id/mode/id2/area_code`.
- `PLC_AUTO_CODE` core model defines `PlcPoint` variants (Io/Tcp/Soft/Rtu485) with IO fields like module_type/channel_tag_no/hmi_var_name and alarm ranges.
- `PLC_AUTO_CODE` `DeviceGroup` groups points by `template_name` and `device_no`, with `points` as alias→PlcPoint mappings.
- `PLC_AUTO_CODE` repo root contains `Docs/`, `repo/` and `repo (副本)/` directories plus a `repo (副本).zip` and `workspace_full_listing.txt`; no `AGENTS.md` found at root search.
- `PLC_AUTO_CODE\\repo` appears as a Rust workspace with `core/`, `adapters/`, `apps/`, `api/`, `config/`, `tools/`, and a top-level `Cargo.toml`.
- `PLC_AUTO_CODE` contains a template system in `core/src/port/template.rs` (TemplateRegistryPort/TemplateRendererPort) and a Tera-based adapter in `adapters/template-tera`.
- `core/src/port/template.rs` defines brand-agnostic `TemplateRegistryPort` (list/get) and `TemplateRendererPort` (render with JSON ctx).
- `core/src/model/template.rs` defines `PlcBrand`, `TemplateKind`, `TemplateId`, `TemplateSource`, and output `Bundle/Payload` with `Delivery` (Memory/Clipboard/File) and text/binary payload bodies.
- `core/src/port/generator.rs` defines Preprocessor/Postprocessor/Retriever/Backend ports and orchestration data models (RenderTask, Rendered, Bundle, Delivery, config, manifest, statistics).
- `core/src/error.rs` maps template/renderer/pre/post/retriever failures into `CoreError` variants, keeping third-party errors out of core.
- Docs root contains architecture/rules documents and an empty `plc_code_doc.md`; no existing dedicated template-management design doc found.
- `adapters/template-tera` scans brand/kind directories for `.tera` templates and registers templates by `(brand, kind, name)`; uses Tera for rendering.
- `adapters/template-tera` resolves brand/kind directory candidates, recursively scans `.tera` files, deduplicates by `(kind, name)`, and reads template contents on demand; Tera renderer registers a `none` tester for legacy templates.
- `core/src/usecase/orchestrate.rs` implements the pipeline: preprocessor → registry get + renderer render (concurrent) → postprocessor assemble bundles → retriever deliver; injects runtime config into passthrough for postprocessor use.
- `adapters/plc/plc-hollysys/src/preprocessor.rs` builds RenderTasks from IO points + device groups, selecting template IDs per IoKind and building JSON ctx (device aliases map to variable names).
- `adapters/plc/plc-hollysys/src/postprocessor.rs` parses rendered text sections (program name / vars block / body) using markers `程序名称:` `子程序变量声明文件:` `程序模板区域:`, normalizes whitespace, groups by template kind/name, and emits ST payloads (variables hook planned).
- `core/src/port/generator.rs` defines a preprocessor/postprocessor pipeline around template rendering; brand adapters (e.g., `adapters/plc/plc-hollysys`) implement pre/post to build ctx and split outputs.
- `adapters/sink-clipboard-win` exists but clipboard sink is TODO (Windows clipboard delivery not yet implemented in PLC_AUTO_CODE).
- Hollysys postprocessor expects rendered output with markers like `子程序变量声明文件:` and `程序模板区域:` and splits program/vars payloads accordingly.
- `adapters/plc/plc-hollysys/src/services/variables_parser.rs` parses variable blocks using `变量名称/变量类型/初始值` prefixes and drops entries missing type or initial value (warns instead of error).
- `adapters/plc/plc-hollysys-safety/src/services/ladder_ops.rs` is TODO, indicating atomic ladder operations are planned but not implemented there yet.
- `plc_templates` crate uses edition 2024 and currently contains only the Cargo stub (`add` function + unit test), with no template logic or dependencies.
- Legacy project `PLC_ANA_WINFORM` contains extensive template infrastructure (Scriban templates, template repositories, render services, variable template services).
- Legacy repo root includes `Services/`, `Models/`, `Templates/`, `StGen/`, and a `README.md`; template-related logic is spread across `Services/` and `Models/`.
- `TemplateService` in legacy project saves template artifacts (`.bin` + `.json`) to a location provided by `ITemplateLocationService`.
- `TemplateService` validates non-empty bin/json, then writes `{safe}.bin` and `{safe}.json` into template dir.
- `TemplateLocationService` sanitizes names (invalid filename chars → `_`), defaults to `Template_yyyyMMdd_HHmmss`, searches up to 5 parent dirs for `Templates`, and ensures per-template + `VARIABLE` subdirs.
- `ScribanTemplateGenerator` emits a human-readable ladder template format: per rung headers, left contacts, right blocks/coils, with structured `[ ... ]` blocks and pin bindings; filters out built-in EN/ENO pins and empty names.
- `ScribanTemplateWriter` writes `{safe}.scriban` into the template directory.
- `VariableTemplateService` saves variable bin/json into `VARIABLE/{baseName}.bin|.json` under the template directory, using sanitized template directory name.
- `VariableTemplateGenerator` outputs `[ ... ]` blocks with `变量名称/变量类型/变量分类`, skipping ENG_TAG/END_TAG, mapping VariableKind to 块变量/基础变量.
- `VariableTemplateRenderService` resolves VARIABLE directory by category (IO → `Templates/MAPPING/{key}`, device → `Templates/DEVICE_CONTROL/{name}`), renders Scriban with `TemplateContext` and ScriptObject, loads JSON/BIN snapshots, and returns warnings for missing/parse failures.
- `VariableTemplateParser` scans rendered text line-by-line for `[ ... ]` blocks and `变量名称/变量类型/变量分类` labels, supports missing trailing `]`, and maps 分类 to VariableKind (block/basic) with line numbers.
- `VariableGenerationService` orchestrates variable generation: parse rendered template → generate binary slices → assemble clipboard payload, passing warnings along.
- `VariableGenerationService` short-circuits on missing template/rendered text or zero parsed definitions; otherwise it builds slices and assembles payload, returning warnings.
- `VariableClipboardAssembler` rebuilds Siemens variable clipboard payload by extracting header (optional leading flag + name length + name + count), updating count byte, and prefixing slices with Base/Block defaults (first occurrence) or short prefixes (0x0180/0x0380).
- `VariableBinaryGenerator` slices original variable bytes using JSON overview ranges, strips Siemens Base/Block prefixes, rewrites variable names in ASCII, and returns per-variable payloads.
- `VariableBinaryGenerator` enforces 1:1 mapping between template definitions and JSON instances (excluding ENG_TAG/END_TAG), slices VarsStart/VarsEnd ranges, strips default + feature prefixes, and rewrites name bytes (ASCII-only).
- `VariableFeaturePrefixes` centralizes Siemens clipboard prefix constants (default Base/Block headers and regular 0x0180 prefixes).
- `VariableFeaturePrefixes` defines Base/Block default prefixes (0xFFFF... "CBasedB"/"CFunctionBlockDB") and regular 0x0180 prefixes for Siemens clipboard segments.
- `PointTemplateRepository` caches Scriban templates (IO and device) loaded from `Templates/MAPPING` or `Templates/DEVICE_CONTROL`, with sanitized-name fallback.
- `InMemoryPointTemplateStore` holds the latest rendered `PointTemplateSet` in memory for reuse.
- `PointTemplateRenderService` renders IO/device templates from point table snapshots via Scriban and invokes variable template rendering per item.
- `LadderTemplateBundleLoader` resolves `{name}.bin/.json/.scriban` (fallback to first matching files), loads binary + scriban text, and deserializes `ScanModel` (falls back to parsing service on JSON failure).
- `LadderEditService` applies Scriban edits to a template bundle: parse scriban → diff → plan patches → edit rung binaries → reassemble clipboard bytes (with trailing empty rung trimming).
- `LadderEditService` loads a template bundle, diffs parsed Scriban vs. original, applies per-rung edits via planner/editor/assembler, then trims trailing empty rungs using header scanning heuristics.
- `ScribanTemplateWriter` saves the main ladder scriban template to the per-template directory.
- `VariableTemplateWriter` saves variable scriban templates into the `VARIABLE` subfolder of a template directory.
- `ScribanEditParser` parses the ladder text format using rung separators and section headers, extracting contacts/blocks/coils and pin bindings into a `ScribanDocument`.
- `LadderDiffService` compares Scriban doc vs. parsed rung structure and emits `StringEdit` entries for changed contact/coil names and block instance/pin bindings (with length/data offsets).
- `BinaryPatchPlanner` groups edits per rung, derives rung header start/end, and validates edit offsets fall within rung ranges before building a plan.
- `RungBinaryEditor` applies per-rung string edits (and optional cleanup) by mutating byte slices.
- `StringTokenMutator` rewrites length-prefixed ASCII strings inside rung bytes, resizing the buffer when lengths change.
- `ClipboardAssembler` reassembles edited rung payloads back into the full binary stream.
- Codex skills are loaded from multiple locations with precedence: `$CWD/.codex/skills`, `$CWD/../.codex/skills`, `$REPO_ROOT/.codex/skills`, `$CODEX_HOME/skills` (default `~/.codex/skills`), `/etc/codex/skills`, and built-in system skills.
- Skills can be invoked explicitly via `/skills` or `$skill-name`, or implicitly when task matches a skill description.
- Per-skill enable/disable is configurable via `[[skills.config]]` entries in `~/.codex/config.toml` (experimental).
- Current environment has `CODEX_INTERNAL_ORIGINATOR_OVERRIDE=codex_vscode`, suggesting a VSCode-originated session.

## Technical Decisions
<!-- 
  WHAT: Architecture and implementation choices you've made, with reasoning.
  WHY: You'll forget why you chose a technology or approach. This table preserves that knowledge.
  WHEN: Update whenever you make a significant technical choice.
  EXAMPLE:
    | Use JSON for storage | Simple, human-readable, built-in Python support |
    | argparse with subcommands | Clean CLI: python todo.py add "task" |
-->
<!-- Decisions made with rationale -->
| Decision | Rationale |
|----------|-----------|
|          |           |

## Issues Encountered
<!-- 
  WHAT: Problems you ran into and how you solved them.
  WHY: Similar to errors in task_plan.md, but focused on broader issues (not just code errors).
  WHEN: Document when you encounter blockers or unexpected challenges.
  EXAMPLE:
    | Empty file causes JSONDecodeError | Added explicit empty file check before json.load() |
-->
<!-- Errors and how they were resolved -->
| Issue | Resolution |
|-------|------------|
| `AGENTS.md` not found in repo root search | Proceeding without additional local agent instructions |
| planning-with-files session-catchup script missing at `$env:USERPROFILE\.claude\skills\planning-with-files\scripts\session-catchup.py` | Logged error; will continue with current session state |

## Resources
<!-- 
  WHAT: URLs, file paths, API references, documentation links you've found useful.
  WHY: Easy reference for later. Don't lose important links in context.
  WHEN: Add as you discover useful resources.
  EXAMPLE:
    - Python argparse docs: https://docs.python.org/3/library/argparse.html
    - Project structure: src/main.py, src/utils.py
-->
<!-- URLs, file paths, API references -->
- `plc_templates/Cargo.toml`
- `plc_templates/src/lib.rs`
- `plc_core/src/lib.rs`
- `plc_core/src/application/service.rs`
- `plc_core/src/domain/*.rs`
- https://developers.openai.com/codex/skills/

## Visual/Browser Findings
<!-- 
  WHAT: Information you learned from viewing images, PDFs, or browser results.
  WHY: CRITICAL - Visual/multimodal content doesn't persist in context. Must be captured as text.
  WHEN: IMMEDIATELY after viewing images or browser results. Don't wait!
  EXAMPLE:
    - Screenshot shows login form has email and password fields
    - Browser shows API returns JSON with "status" and "data" keys
-->
<!-- CRITICAL: Update after every 2 view/browser operations -->
<!-- Multimodal content must be captured as text immediately -->
-

---
<!-- 
  REMINDER: The 2-Action Rule
  After every 2 view/browser/search operations, you MUST update this file.
  This prevents visual information from being lost when context resets.
-->
*Update this file after every 2 view/browser/search operations*
*This prevents visual information from being lost*
