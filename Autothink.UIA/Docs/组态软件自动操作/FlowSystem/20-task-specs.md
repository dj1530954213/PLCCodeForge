# 任务规格定义（四大流程）

## 总览
- import_variables：变量导入。
- hardware_config：硬件组态（CPU/设备/协议/模块/参数设定）。
- add_program_blocks：程序块创建与粘贴。
- comm_program：通讯程序与指令配置。

## 依赖建议（可由用户调整）
- import_variables → hardware_config/add_program_blocks。
- hardware_config + add_program_blocks → comm_program。
- 用户可选择从任意任务开始，Orchestrator 自动补齐硬依赖。

## Task: import_variables
### 目标
- 将变量表导入工程，作为后续硬件/程序配置依据。

### 输入
- varsXls：变量表路径（字符串）。

### 前置条件
- 主窗口已打开且稳定。
- 无阻塞弹窗。

### 必需 Profile 资源
- selectors：mainWindow、menuProject、menuImportVariables、importDialog、fileDialog、confirmButton。
- anchors：mainWindow。

### 关键步骤（示意）
1) ensure_selector(mainWindow)
2) menu_import_xls(path=@varsXls)
3) wait_until(ElementNotExists, importDialog)

### 后置校验
- importDialog 关闭。
- 可选：状态栏提示或变量列表刷新。

### 输出
- produces: [vars_imported]

### 失败策略
- dialog 未出现：retry 2 次。
- 文件路径无效：立即 fail。

## Task: hardware_config
### 目标
- 依据点表硬件信息完成 CPU/设备/协议/模块添加与模块参数设定。

### 输入
- cpuTypes：可接受 CPU 型号列表（如 LK220/LK220S）。
- deviceType：默认 LK249（可配置）。
- protocolType：默认 DP_MASTER（可配置）。
- racks：机架清单（地址/编号）。
- modules：模块清单（类型/槽位/机架/参数模板）。

### 前置条件
- 硬件树可见。
- CPU 节点存在。

### 必需 Profile 资源
- selectors：hardwareTree、cpuNode、contextMenuAddDevice、deviceDialog、protocolDialog、moduleDialog。
- anchors：mainWindow、mfcPanel（自绘区域）。
- positions：param_tab_*、module_param_*、dialog_ok。
- navSequences：模块参数页导航序列。

### 关键步骤（示意）
1) ensure_selector(hardwareTree)
2) select_node(cpuNode)
3) right_click(cpuNode) → add_device(deviceType, rack.addr)
4) wait_until(deviceDialog_closed)
5) add_protocol(protocolType)
6) for_each(modules):
   - add_module(type, slot, rackIndex)
   - configure_module(profileId, params)

### 模块添加规则说明
- 模块需添加到对应机架或设备节点下。
- 模块地址/槽位由外部规则计算后传入，不在 UIA 内部计算。

### 模块参数设定（核心约束）
- MFC 自绘区域以左上角为 (0,0) 坐标原点。
- 所有点击坐标来自 Profile positions。
- 模块类型差异通过模板分发，不在动作层写死。
- 常见路径：点击参数标签页 → key_nav 移动焦点 → 坐标点击输入区域。

### 后置校验
- 模块节点存在（ensure_selector(moduleNode@id)）。
- 可选：参数页确认状态。

### 输出
- produces: [hardware_ready]

### 失败策略
- 模块对话框未出现：retry。
- 模块类型缺失：记录 Warning，按策略 skip 或 stop。

## Task: add_program_blocks
### 目标
- 创建程序块并粘贴预置程序内容。

### 输入
- programs：程序清单（type/name/text）。

### 前置条件
- 程序块列表可见。
- 编辑区可打开。

### 必需 Profile 资源
- selectors：programList、contextMenuAddProgram、programDialog、programEditor。
- anchors：mainWindow。

### 关键步骤（示意）
1) for_each(programs):
   - right_click(programList) → add_program(type, name)
   - click(programEditor)
   - paste(text)
   - delay(300)

### 后置校验
- 程序块列表存在对应名称。

### 输出
- produces: [programs_ready]

### 失败策略
- 粘贴失败：fallback 为 set_text。
- 编辑区未聚焦：执行 focus + retry。

## Task: comm_program
### 目标
- 按模块类型插入通讯程序与命令逻辑。

### 输入
- modules：模块清单（含 type/profileId）。
- commTemplates：模块类型 → 子模板映射。

### 前置条件
- hardware_ready、programs_ready（硬依赖）。

### 必需 Profile 资源
- selectors：programEditor、commMenu、commDialog。
- anchors：mainWindow、mfcPanel。

### 关键步骤（示意）
1) for_each(modules):
   - use_template(commTemplates[@item.type], params=@item.params)
2) 可选：整体通讯程序收尾模板（验证/保存）。

### 后置校验
- 通讯程序块存在或编译通过提示。

### 输出
- produces: [comm_ready]

### 失败策略
- 模板未匹配：记录 Warning，按策略 skip/stop。

## 用户可选顺序与依赖提示
- 用户可选择只执行 hardware_config 或 add_program_blocks。
- 若选择 comm_program，系统必须提示缺失的硬依赖并自动补齐。

## UI 状态转换建议
- MainWindow → ImportDialog：变量导入期间的对话框状态。
- MainWindow → ProgramEditor：程序粘贴阶段。
- MainWindow → ModuleDialog → MfcPanel：模块参数设定阶段。
- Unknown：出现弹窗或焦点异常时进入恢复流程。
