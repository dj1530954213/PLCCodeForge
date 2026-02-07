# PlcGen 模板管理设计与实施计划（草案）

> 目标：以六边形架构落地模板管理，保留 Tera 规则脚本与前端配置能力，支持“模板 POU → 多实例扩展 → 单一最终 POU”的核心流程。

## 1. 背景与范围
- 模板创建：从 Windows 自定义剪贴板读取 POU 二进制 → `plc_core` 解码 → 完整 POU JSON 快照。
- 模板使用：基于模板执行渲染与批量操作，生成多套梯级/变量集合，**最终仍是一个完整 POU**。
- Tera：模板规则脚本继续使用 Tera，UI 负责参数配置表单化。
- 原子化操作分两条路径：
  1) **模板驱动（当前优先）**：基于模板快照做复制/替换。
  2) **自由构造（后续阶段）**：从零构建 POU，独立 `plc_pou_builder`。

## 2. 关键需求汇总
1. 模板创建时 ladder + variables 一起生成（完整 POU）。
2. 可变字段：POU 名称、变量实例名、块名称、线圈名称、触点名称、块针脚绑定变量名、变量信息(包括注释)。
3. 初始值字段保留完整信息结构（先设计，后续实现）。
4. 模板包：同一模板所需文件放入一个目录；扫描/发现由品牌适配器实现。
5. 渲染后必须校验，不合法直接报错，避免组态软件崩溃。
6. 模板可包含多个梯级/变量集合，整体重复多套并合并为一个最终 POU。

## 3. 总体架构（六边形）
```
UI/Tauri
   │
   ├── 模板配置 UI（表单化输入）
   │
plc_templates（核心域 + 应用服务 + 端口）
   │
   ├── TemplateStoragePort（模板包读写）
   ├── TemplateDiscoveryPort（模板发现/扫描）
   ├── TemplateExtractorPort（可变字段抽取）
   ├── TemplateRendererPort（Tera 渲染）
   ├── TemplatePatchExecutor（应用 Patch）
   ├── PouValidatorPort（校验）
   └── PouEncoderPort（编码，复用 plc_core）
   │
品牌适配器（brand-specific）
   ├── 目录结构差异、扫描规则
   ├── 约束/校验规则
   ├── 元素定位差异（路径/字段差异）
   └── 编码细节与扩展

plc_core（POU 编解码/AST）
plc_pou_builder（后续，自由构造原子操作）
```

### 3.1 plc_templates 与 plc_core 交互（起点）
**核心定位：** plc_core 提供 POU 编解码与 AST 数据契约；plc_templates 负责模板提取、渲染与扩展。

**交互点 A：模板创建（Decode）**
1) 读取剪贴板二进制  
2) 选择品牌适配器（Hollysys/Siemens...）→ `PouCodec::decode`  
3) 输出 `UniversalPou`（含 `variables: Vec<VariableNode>` 的树结构）  
4) `TemplateExtractor` 从 AST 抽取可变字段与 `template_sets`

**交互点 B：模板渲染与应用（Patch → AST）**
1) UI 配置生成 `template.config.json`  
2) Tera 渲染输出 Patch JSON  
3) `TemplatePatchExecutor` 将 Patch 作用在 `UniversalPou` 上  
4) `PouValidator` 执行校验（通用 + 品牌规则）

**交互点 C：模板产物生成（Encode）**
1) `PouCodec::encode` 将修改后的 `UniversalPou` 编码回二进制  
2) 交付到剪贴板/文件/内存  

**数据契约与依赖**
- `UniversalPou` 是唯一真源结构，模板逻辑不得绕开 AST 直接操作二进制。  
- 变量层级以 `VariableNode` 树为准（解析器会根据 `symbols_config.json` 进行分组）。  
- `LdElement` 的 `name/instance/pins` 对应模板中“块名/实例名/针脚绑定”。

### 3.2 数据契约与定位规则（当前已定）
- **顶层字段（已见）**：`name` / `header_strings` / `variables` / `networks`。  
  其他头部字段（程序类型/语言等）当前固定不编辑。
- **变量定位**：使用变量树路径（Group 链 + Leaf name）作为稳定锚点。  
- **元件定位**：使用梯级树（networks 列表）定位；结合 `network.id`、元素序号、`type_code/name/instance` 做多字段匹配。  
- **针脚绑定**：允许为空（未绑定）。  
- **Normal/Safety 差异**：当前优先在模板层屏蔽差异（Safety 拓扑 token 先透传不编辑）；若需改动则由品牌适配器处理。
- **网络 ID 规则（临时）**：扩展时采用 “max_id + 1” 递增策略（后续可替换为品牌规则）。

### 3.3 plc_templates 与 plc_importer 交互（数据流）
**核心定位：** plc_importer 只负责“点表 → 统一模型”，不掺入模板规则；模板实例化逻辑在 plc_templates/品牌适配器侧完成。

**输入数据（来自 plc_importer）**
- `ImportResult`：  
  - `points: HashMap<String, PlcPoint>`（点位字典）  
  - `device_group: Vec<DeviceGroup>`（设备组：template_name + device_no + alias→点位）  
  - `point_order: Vec<String>`（保持 IO 行序，用于确定性生成）

**交互流程**
1) UI/第三方平台传入 `brand/series/model`（前三层固定）  
2) plc_importer 读取点表（含设备分类表）→ 输出 `ImportResult`  
3) 模板选择规则：  
   - IO 点位：模板固定为 AI/AO/DI/DO 映射（硬点通道只有这四类）  
   - 设备组：根据设备分类表确定 `category_path + template_name`  
4) plc_templates（或品牌适配器的 InstancePlanner）基于 `ImportResult` 生成 `TemplateConfig`：  
   - IO 点位：按 `point_order` 保序映射  
   - 设备组：按 `template_name + device_no` 形成实例，并将 alias 映射到变量名  
5) `TemplateConfig` 作为 Tera 渲染上下文 → Patch → 扩展 → 最终 POU  

**边界约束**
- plc_importer 不依赖模板目录，也不感知分类路径。  
- 模板**不由 UI 直接选择**；分类路径来自设备分类表，模板精确定位需 `brand/series/model + category_path`。  
- 若 ImportResult 缺字段或映射失败，模板层应给出 warning/错误，不在 importer 内硬编码模板规则。

### 3.4 plc_templates 与 plc_logic_gen 交互（自动编程编排）
**核心定位：** plc_logic_gen 负责自动编程的编排与流程控制，模板生成/扩展由 plc_templates 负责。

**输入来源**
- `ImportResult`（来自 plc_importer）
- `brand/series/model`（来自 UI 或第三方平台）
- 生成策略（扩展数量、实例命名规则、输出方式）

**交互流程**
1) plc_logic_gen 组织“生成请求”并调用 plc_templates  
2) plc_templates 读取模板包 → Tera 渲染 Patch → 扩展为最终 POU  
3) plc_logic_gen 负责产物编排与交付（剪贴板/文件/内存）  
4) 额外的第三方通讯设备点表在 plc_logic_gen 内生成并参与后续流程  

**边界约束**
- plc_logic_gen 不直接解析/修改 AST，只编排流程与策略。  
- 当前规则：**每个模板对应一个 POU**，暂不处理聚合。

**建议的生成请求结构（草案）**
```json
{
  "brand": "和利时",
  "series": "和利时普通型",
  "model": "默认",
  "templates_root": "X:/templates_root",
  "output_dir": "X:/output",
  "import_result_ref": "ImportResult",
  "strategy": {
    "io_mapping_fixed": ["AI", "AO", "DI", "DO"],
    "device_category_path": "执行机构/阀门/ESD阀门",
    "naming_rule": "{template_name}_{index}",
    "delivery": "clipboard"
  }
}
```

**产物规则**
- IO 模板与设备模板均输出单一 POU（无聚合）。
- 每个模板对应一份产物（POU + 可选变量表）。

**通讯设备点表（plc_logic_gen 内）**
- 第三方通讯设备（485/TCP）的点表由 plc_logic_gen 负责生成与注入。
- 生成后的点表进入同一编排流程，参与模板渲染与 POU 产出。

### 3.5 plc_templates 与 plc_pou_builder 交互（统一入口）
**核心定位：** plc_logic_gen 作为统一入口，按场景分流到模板路径或 builder 路径。  

**交互规则**
- 模板路径：plc_templates 负责渲染/扩展 → 产出 `UniversalPou`  
- 构造路径：plc_pou_builder 负责从零构建 → 产出 `UniversalPou`  
- 编码与交付统一走 plc_core + plc_logic_gen 流程

**plc_pou_builder 能力边界（规划）**
- 提供原子化构造 API：创建网络、插入触点/线圈/功能块、绑定针脚、创建变量节点。  
- 维护元素/功能块/变量库（官方 + 自定义），供 UI 选择与校验。  
- 仅输出 `UniversalPou`，不直接编码或落盘。  

**与模板路径的一致性**
- 模板路径与 builder 路径最终都产出 `UniversalPou`。  
- 通用校验与编码复用 plc_core；品牌差异校验放在适配器。  

**当前状态**
- `plc_pou_builder` crate 尚未落地（规划阶段），先按接口契约设计与文档推进。

### 3.6 plc_pou_builder 详细设计（规则待补齐）
**定位**
- 仅负责“从零构造 `UniversalPou`”，不负责模板渲染、编码或落盘。
- 语义合法性由品牌适配器规则约束，规则来源见 `Docs/第二轮核对的全局规则.md`。

**输入/输出**
- 输入：`BuildPlan`（plc_logic_gen 生成，包含品牌信息、头字段、构造步骤、库引用）
- 输出：`UniversalPou`（交给 plc_logic_gen → plc_core 编码）

**BuildPlan（草案）**
```
BuildPlan
  meta: { brand, series, model, variant, format_name }
  pou_header: { name, header_strings, ... }
  steps: Vec<BuildStep>
  libs: { block_lib_ref, var_lib_ref, element_lib_ref }
```

**BuildStep（原子）**
```
create_network(id?)
add_contact(network_id, var_ref, sub_type)
add_coil(network_id, var_ref, sub_type)
add_block(network_id, block_ref, instance_name?)
bind_pin(block_id, pin_name, var_ref)
connect(src_id, dst_id)
add_variable(group_path, var_def)
```

**拓扑与语义处理**
- builder 只生成“元素 + 连接图/步骤序列”，不内置梯级语义。
- 适配器提供 `TopologyBuilder/Validator`：将连接图转为 Normal `connections` 或 Safety `tokens`。
- 违反语义规则时返回 `BuildError::RuleViolation(RuleID)`，用于前端提示与回溯。

**库结构（官方 + 自定义）**
- `BlockLibrary`：块定义（FB/FC/指令）、pin 列表、输入/输出类型、实例要求。
- `VariableLibrary`：变量模板 + 初始值结构（支持层级变量）。
- `ElementLibrary`：Contact/Coil/Box 等元素模板与限制。

#### 3.6.1 BuildPlan JSON 结构（v1 草案）
```json
{
  "schema_version": 1,
  "meta": {
    "brand": "和利时",
    "series": "和利时普通型",
    "model": "默认",
    "variant": "Normal",
    "format_name": "POU_TREE_Clipboard_PLC"
  },
  "pou_header": {
    "name": "AI_GROUP_1",
    "header_strings": ["..."]
  },
  "libraries": {
    "block_lib_ref": "hollysys/official@1.0",
    "variable_lib_ref": "hollysys/vars@1.0",
    "element_lib_ref": "hollysys/elements@1.0"
  },
  "steps": [
    { "op": "create_network", "id": "net_1", "label": "AI" },
    { "op": "add_contact", "id": "c1", "network_id": "net_1",
      "var_ref": { "path": ["Local Variables", "AI", "AI_1"] }, "sub_type": 0 },
    { "op": "add_block", "id": "b1", "network_id": "net_1",
      "block_ref": { "name": "AI_CONVERT", "source": "official" }, "instance": "AI_CONV_1" },
    { "op": "bind_pin", "block_id": "b1", "pin_name": "IN",
      "var_ref": { "path": ["Local Variables", "AI", "AI_1"] } },
    { "op": "add_coil", "id": "o1", "network_id": "net_1",
      "var_ref": { "path": ["Local Variables", "AI", "AI_OK"] }, "sub_type": 0 },
    { "op": "connect", "src_id": "c1", "dst_id": "b1", "link_type": "expr" },
    { "op": "connect", "src_id": "b1", "dst_id": "o1", "link_type": "stmt" },
    { "op": "add_variable", "group_path": ["Local Variables", "AI"],
      "var_def": {
        "name": "AI_1",
        "data_type": "INT",
        "init_value": { "raw": "0" }
      }
    }
  ]
}
```

**字段说明（要点）**
- `schema_version`：BuildPlan 结构版本。
- `meta`：品牌/系列/型号/变体/format_name，用于适配器选择与校验。
- `pou_header`：仅包含允许编辑的头字段（当前仅 name/header_strings）。
- `libraries`：引用官方/自定义库，避免重复定义。
- `steps`：原子操作序列；语义约束来自全局规则。
- `var_ref.path`：变量树路径（与解析器输出一致）。
- `connect.link_type`：`expr`=表达式树挂接，`stmt`=语句链挂接（见 `Docs/第二轮核对的全局规则.md`）。
- `init_value`：先保留完整结构，后续按解析字段扩展。

**BuildStep 枚举（v1）**
- `create_network(id?, label?, comment?)`
- `add_contact(id, network_id, var_ref, sub_type, comment?)`
- `add_coil(id, network_id, var_ref, sub_type, comment?)`
- `add_block(id, network_id, block_ref, instance?, comment?)`
- `bind_pin(block_id, pin_name, var_ref)`
- `connect(src_id, dst_id, link_type)`
- `add_variable(group_path, var_def)`

#### 3.6.2 TopologyBuilder/Validator 接口（适配器层）
**目标**
- 将 BuildPlan 的“元素 + 连接图”转换为可序列化的 `Network` 数据。
- 语义规则来源：`Docs/第二轮核对的全局规则.md`（CalcLogic/语义规则表）。

**输入结构（草案）**
```
TopologyInput
  networks: Vec<NetworkDraft>

NetworkDraft
  id: i32
  label: String
  comment: String
  nodes: Vec<ElementDraft>
  edges: Vec<EdgeDraft>

ElementDraft
  id: String
  type_code: ElementType
  name: String
  instance: String
  pins: Vec<BoxPin>
  sub_type: u8
  comment: String
  desc: String

EdgeDraft
  src_id: String
  dst_id: String
  link_type: "expr" | "stmt"
```

**接口草案**
```
TopologyValidatorPort
  validate(input: &TopologyInput) -> Result<(), RuleViolation>

TopologyBuilderPort
  build(input: TopologyInput) -> Result<Vec<Network>, BuildError>
```

**行为约束**
- Normal：根据 `edges` 生成 `LdElement.connections`（ConnRefs 图），并保持网络归属一致。
- Safety：生成 `safety_topology`（Token 流），多元素网络必须提供 Token 流。
- `link_type` 映射到 CalcLogic 的挂接规则：`expr`=表达式树，`stmt`=语句链。

**错误返回**
- `RuleViolation`：包含 RuleID / network_id / element_id / detail。
- `BuildError`：非法拓扑、缺失依赖、未支持元素类型等。

## 4. 目录结构设计

### 4.1 PlcGen 工程目录（plc + template）
```
PlcGen/
  plc_core/
  plc_templates/
    Cargo.toml
    Cargo.lock
    src/
      lib.rs
      domain/
        mod.rs
        template_bundle.rs
        template_spec.rs
        template_config.rs
        template_patch.rs
        template_set.rs
        template_errors.rs
      application/
        mod.rs
        template_create_service.rs
        template_render_service.rs
        template_expand_service.rs
        template_apply_service.rs
      ports/
        mod.rs
        storage_port.rs
        discovery_port.rs
        extractor_port.rs
        renderer_port.rs
        patch_executor_port.rs
        validator_port.rs
        encoder_port.rs
      adapters/          # 可选：也可拆成独立 brand crate
        mod.rs
        fs_storage.rs
        tera_renderer.rs
    tests/
  plc_pou_builder/       # 后续阶段（自由构造原子操作）
  plc_importer/
  plc_logic_gen/
  plc_core_tester/
  Docs/
```
> 说明：以上为 **规划的文件级结构**（落地后以实际文件为准），`target/` 为构建产物不纳入设计文档。

### 4.2 模板包目录结构（运行期仓库）
**固定层级（前 3 级）：** 品牌 / 系列 / 型号（或一级业务域，由调用方传入）  
**可变层级（后续 N 级）：** 分类路径（任意深度）
```
{templates_root}/{brand}/{series}/{model}/{category_path...}/{template_name}/
  pou.bin                # 原始剪贴板二进制
  pou.json               # plc_core 解码后的完整 JSON
  template.spec.json     # 可变字段/集合描述/表单 schema
  template.tera          # 规则脚本（输出 Patch JSON）
  template.meta.json     # 版本/品牌/创建时间/备注
```
**分类规则：**
- 分类路径以**目录层级**为准（由适配器扫描目录树得到）。
- 同名模板允许存在于不同分类路径下；模板精确定位必须携带 `brand/series/model` + 分类路径。
- 用户**不直接接触文件/目录**：UI 提供前三层固定选择，后续层级由用户在 UI 中自定义维护。

**示例：**
```
templates_root/和利时/和利时普通型/IO映射/AI映射/AI_CONVERT/
templates_root/和利时/和利时普通型/执行机构/阀门/ESD阀门/ESD阀门带手动按钮/
```

### 4.3 PLC_AUTO_CODE 模板脚本示例（文件级）
参考旧项目模板目录：`C:\Program Files\Git\code\PLC_AUTO_CODE\repo\adapters\template-tera\plc-hollysys`
```
plc-hollysys/
  AI/MAPPING_AI.tera
  AO/MAPPING_AO.tera
  DI/MAPPING_DI.tera
  DO/MAPPING_DO.tera
  TCP通讯/MAPPING_TCP_A.tera
  TCP通讯/MAPPING_TCP_D.tera
  设备/阀门/ESDV_CTRL.tera
  设备/阀门/PID_CTRL.tera
  设备/阀门/XV_CTRL.tera
```

## 5. 模板生命周期（端到端）
1) 读取剪贴板 → plc_core 解码  
2) 生成 POU JSON（快照）  
3) 提取可变字段与集合信息 → `template.spec.json`  
4) UI 生成配置表单 → 输出 `template.config.json`  
5) Tera 渲染脚本 → 输出 Patch JSON  
6) Patch 执行器扩展/修改 POU  
7) 校验（品牌规则 + 通用规则）  
8) plc_core 编码 → 最终 POU 二进制  
9) 交付（剪贴板/文件/内存）
10) `template.meta.json` 必须记录 `variant/serialize_version/format_name`，用于后续一致性校验

## 6. TemplateSpec（可变字段与集合描述）
**目标**：让 UI 可配置、让 Tera 可渲染、让执行器可定位与扩展集合。

示意：
```json
{
  "meta": { "brand": "Hollysys", "version": "1.0" },
  "inputs": [
    { "key": "pou_name", "label": "POU 名称", "type": "string", "required": true },
    { "key": "ai_count", "label": "AI 数量", "type": "number", "min": 1 }
  ],
  "anchors": [
    { "id": "BLOCK_AI_CONVERT", "path": "$.networks[0].elements[2]", "match": { "type": "AI_CONVERT", "instance": "AI_CONVERT_1" } }
  ],
  "template_sets": [
    {
      "id": "AI_SET",
      "networks": [0,1,2],
      "variables": ["AI_CONVERT_1", "AI_PV_1"],
      "anchors": ["BLOCK_AI_CONVERT"]
    }
  ]
}
```

## 7. Patch 输出格式（对接原子接口）
**原则**：路径 + 多字段匹配；支持“重复集合”。

```json
{
  "schema_version": "1.0",
  "ops": [
    {
      "op": "rename_pou",
      "target": { "path": "$.pou.name", "match": { "original": "TEMPLATE_POU" } },
      "value": { "name": "{{ cfg.pou_name }}" }
    },
    {
      "op": "repeat_template_set",
      "set": "AI_SET",
      "instances": [
        { "index": 1, "vars": { "AI_TAG": "AI_001", "PV": "PV_001" } },
        { "index": 2, "vars": { "AI_TAG": "AI_002", "PV": "PV_002" } }
      ]
    }
  ]
}
```

### 7.1 Patch → 原子操作映射
| Patch op | 原子接口 |
|---|---|
| rename_pou | rename_pou |
| rename_var | rename_variable |
| rename_block | rename_block |
| rename_contact | rename_contact |
| rename_coil | rename_coil |
| bind_pin | bind_pin |
| set_var_init | set_variable_init（预留） |
| repeat_unit / repeat_template_set | expand_set（克隆 + 替换） |

### 7.2 执行器规则
- `path` 先定位候选，`match` 多字段过滤，必须唯一。
- `expected` 可选，用于防止模板漂移。
- `repeat_template_set`：克隆 networks + variables + 引用，按 instance 替换名称与绑定。
- **执行顺序：先修改原模板集合 → 再扩展**。  
  原因：POU 顶层信息（如 POU 名称、程序类型）是全局数据，应先完成基准修正，再复制实例。

## 8. 多实例扩展（核心能力）
- 模板可能是单个梯级，也可能是多个梯级 + 多变量的“整体集合”。
- 执行器必须支持：
  1) 克隆网络集合  
  2) 克隆局部变量集合  
  3) 批量替换名称与绑定  
  4) 合并为单一 POU  
  5) 保持网络顺序与 ID 的一致性  
  6) 基于原模板集合扩展（原集合保留为第 1 套，其余为追加克隆）

**冲突处理：**
- 网络 ID 与变量名冲突由执行器自动处理（规则待补充，后续结合“全局规则地图”落地）。

## 9. 初始值结构（预留）
初始值必须支持**层级变量**，采用树结构存储，节点字段尽量保留解析器输出内容（与解析一致即可）。  
建议与 `VariableNode` 层级一致：Group 节点仅含 `children`，Leaf 节点含完整初始值细节。

示意：
```json
{
  "init_tree": {
    "name": "FB1",
    "children": [
      {
        "name": "PV",
        "data_type": "REAL",
        "init_value": "12.34",
        "soe_enable": false,
        "power_down_keep": false,
        "comment": "",
        "var_id": 123,
        "addr_id": 456,
        "mode": 6,
        "id2": 0,
        "area_code": 4
      },
      {
        "name": "LIMITS",
        "children": [
          { "name": "HI", "data_type": "REAL", "init_value": "100.0" },
          { "name": "LO", "data_type": "REAL", "init_value": "0.0" }
        ]
      }
    ]
  }
}
```
> 字段细节按解析器输出为准，层级深度可灵活调整。

## 10. 品牌适配器职责
- 模板扫描/发现（目录结构差异）
- 结构差异修正（路径/字段差异）
- 校验规则（长度、字符集、合法性）
- 编码细节与品牌差异处理

## 11. 与 UI/Tauri 的接口
- UI 读取 `template.spec.json` 生成表单  
- 用户输入生成 `template.config.json`  
- Tera 渲染输出 Patch  
- 渲染/执行日志回传 UI（warnings/errors）

## 12. 与三大模块的关系
1) PLC 自动编程：模板生成与 POU 生成的主流程  
2) UIA 自动操作：负责外部软件的自动化流程编排  
3) 485/TCP 通讯组态：生成通讯程序 → 走同一模板/POU 生成入口  

核心编排层负责把三个模块的调用结果汇总为统一产物（POU）。

## 13. 实施计划（阶段化）
### Phase A（当前优先）
1. `TemplateSpec` 数据模型与 JSON schema  
2. `TemplateRendererPort`（Tera 渲染 Patch）  
3. `TemplatePatchExecutor`（路径 + match）  
4. `repeat_template_set` 扩展能力  
5. 最小 UI 表单对接  

### Phase B（验证与补强）
1. 校验规则抽取（通用 + 品牌）  
2. 初始值结构保留与存储  
3. 跨模板的多品牌差异验证  

### Phase C（后续）
1. `plc_pou_builder` 原子化构造路径  
2. 变量库/功能块库/元素库  
3. 自由构造流程与模板流程合并到统一编排层  

## 14. 风险与对策
- 模板漂移导致 Patch 不一致 → 引入 `match` + `expected`
- 多实例扩展引起冲突 → 执行器必须做唯一性校验
- 品牌差异复杂 → 适配器收敛规则，核心保持抽象

## 15. 结论
当前设计以“模板路径优先交付 + 扩展 Patch 格式 + 独立 Builder 后续迭代”为主线，满足多实例扩展与单一 POU 输出的核心要求。后续在实现与测试中持续调整细节。
