# UI Profile 规范

## 目标
- 把 UI 变化隔离在 Profile 中，避免影响流程 DSL。
- 支持多版本并行，允许快速切换与回滚。

## Profile 字段（建议）
- metadata
  - app：应用名
  - version：UI 版本号
  - lang：语言
  - resolution：分辨率
  - dpiScale：缩放系数
- anchors：锚点集合
- positions：坐标/矩形集合
- selectors：逻辑选择器别名
- navSequences：键盘导航序列
- overlays：局部覆盖（可选）

## metadata 细化字段（建议）
- app：应用名（固定）
- version：UI 版本号（可与 build 号区分）
- build：可选，构建号或补丁号
- lang：语言，如 zh-CN/en-US
- resolution：屏幕分辨率，如 1920x1080
- dpiScale：缩放系数，如 1.0/1.25/1.5
- owner：Profile 维护者（可选）
- createdAt：创建时间（可选）

## selectors 语法（建议）
- selector 为路径数组 Path，按层级匹配。
- 每级可包含：
  - Search：Descendants/Children
  - ControlType：Window/Button/Edit/TreeItem 等
  - Name/NameContains
  - AutomationId
  - ClassName
  - Index：在多匹配时的索引
- 支持多个 selector 候选，按优先级回退。

## anchors 规则
- type: selector | window | fallback
- selector：ElementSelector
- window：标题/进程名匹配
- offset：锚点偏移（坐标原点修正）
- 备注：当前运行时仅支持 selector 类型，window/fallback 预留。

## anchors 选择策略
- selector 优先：最稳定且可复用。
- window 回退：仅用于顶层窗体或固定标题。
- fallback：用于极端场景（例如启动阶段）。

## positions 规则
- point：绝对坐标点（x,y）
- rect：矩形区域（x,y,w,h）
- anchor：所属锚点
- 可选字段：desc、version、tags

## positions 补充约束
- point 与 rect 二选一，禁止同时出现。
- rect 建议用于 click_rel 或区域中心点击。
- tags 可用于按场景筛选（例如 "comm"、"program"）。

## selectors 规则
- key -> [selectorList]
- selectorList 按优先级排序
- 支持 override（按版本或语言覆盖）

## navSequences 规则
- keys：方向键序列
- intervalMs：节流
- verify：可选的完成验证条件

## navSequences 建议
- keys 中允许特殊键：TAB/ENTER/ESC/CTRL+V。
- intervalMs 建议 50-120ms（避免 UI 卡死）。
- verify 支持 selector 或 UIState 校验。

## overlays 规则
- 允许按场景覆盖少量坐标/选择器。
- 用于同版本不同分辨率的快速修补。
- 覆盖优先级：overlay > base profile。

## 覆盖合并规则（建议）
- anchors/positions/selectors/navSequences：按 key 覆盖。
- metadata：仅允许覆盖 dpiScale/resolution。
- 若 overlay 缺失字段，继承 base。

## Profile 选择逻辑
1) 根据 app/version/lang/resolution 匹配候选 Profile。
2) 选择 version 最匹配的 Profile。
3) 读取 overlay（若存在）并合并。

## Profile 选择优先级（建议）
- 精确匹配优先：app+version+lang+resolution。
- 次级匹配：app+version+lang。
- 最低匹配：app+version。

## 坐标换算规则
- 运行时坐标 = Profile 坐标 * dpiScale。
- 若 anchor 解析失败，进入 fallback 逻辑。
- MFC 自绘区域坐标以左上角 (0,0) 为基准。

## 版本兼容建议
- Profile 主版本变化时，Flow/Template 必须显式绑定版本范围。
- minor/patch 变化允许自动适配（仅坐标调整）。

## 命名规范
- anchors：驼峰命名，如 mfcPanel/mainWindow。
- positions：语义命名，如 baudRateField/param_tab_1。
- selectors：业务语义命名，如 importDialog/buildButton。
- navSequences：语义命名，如 nav_to_baud。

## 校验规则（建议）
- anchors 必须至少 1 个可解析。
- positions 引用的 anchors 必须存在。
- selectors 每个 key 至少 1 个 selector。

## 校准流程（建议）
1) 选择目标版本/分辨率/语言。
2) 使用校准工具确定锚点与坐标。
3) 运行 Profile 校验（锚点是否可识别）。
4) 生成 Profile 版本并归档。

## 校准注意事项
- 坐标点应尽量避开边缘与可变区域。
- 记录 DPI 缩放，避免跨机器漂移。
- 将常用坐标抽为 positions，避免在模板中重复出现。

## 示例片段
```json
{
  "metadata": {
    "app": "Autothink",
    "version": "1.0.0",
    "lang": "zh-CN",
    "resolution": "1920x1080",
    "dpiScale": 1.0
  },
  "anchors": {
    "mfcPanel": {
      "type": "selector",
      "selector": { "Path": [ { "Search": "Descendants", "ClassName": "MfcPanel" } ] },
      "offset": [0, 0]
    }
  },
  "positions": {
    "param_tab_1": { "anchor": "mfcPanel", "point": [120, 42] },
    "baudRateField": { "anchor": "mfcPanel", "point": [220, 120] }
  },
  "selectors": {
    "importDialog": [
      { "Path": [ { "ControlType": "Window", "NameContains": "变量导入" } ] }
    ]
  },
  "navSequences": {
    "nav_to_baud": { "keys": ["RIGHT", "RIGHT", "DOWN"], "intervalMs": 80 }
  }
}
```

## 详细参考
- 校准流程详见 `13-profile-calibration-guide.md`。
- 目录结构详见 `16-directory-layout.md`。
