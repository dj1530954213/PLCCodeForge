# 联合 xlsx 输入规范（冻结 v1）

> 适用范围：`comm_import_union_xlsx(path, options?)` 的输入文件（IO 表 + 设备表合并后的“联合点表”）。
>
> 目标：把“联合 xlsx 的 Sheet/列名/地址基准/枚举值”冻结为稳定契约，避免列名或 Sheet 轻微变化导致静默漏导入。
>
> 冻结清单的代码侧单一真源：`src-tauri/src/comm/union_spec_v1.rs`（`SPEC_VERSION_V1 = "v1"`）。

---

## 1. 文件与 Sheet 选择规则

### 1.1 文件格式
- 文件扩展名：`.xlsx`

### 1.2 Sheet 名（冻结 v1）
- **默认目标 Sheet 名（v1）**：`联合点表`
- command 选项：
  - `options.sheetName`：指定要读取的 Sheet 名
  - `options.strict=true` 时：
    - 找不到目标 Sheet → **硬失败**（返回错误，包含可用 Sheet 列表）
  - `options.strict=false` 时：
    - 找不到目标 Sheet → **fallback 到第一个 Sheet**，并产生 warning（可观测）

---

## 2. 表头（列名）冻结规范

### 2.1 表头行
- 表头必须位于 **第 1 行**
- **列名按“逐字匹配”识别**（建议不要有前后空格；实现会对表头做 `trim()` 后比对）

### 2.2 必填列（冻结 v1，strict=true 缺任意一列即失败）
| 列名（逐字） | 说明 |
|---|---|
| 变量名称（HMI） | 点位业务展示名称，不能为空 |
| 数据类型 | `Bool/Int16/UInt16/Int32/UInt32/Float32`（大小写不敏感） |
| 字节序 | `ABCD/BADC/CDAB/DCBA`（大小写不敏感） |
| 通道名称 | 用于生成/匹配 profile 的 `channelName`，不能为空 |
| 协议类型 | `TCP` 或 `485`（大小写不敏感；允许全角字符输入，读取时会做半角化） |
| 设备标识 | TCP: UnitId；485: SlaveId（建议 1~247，读取时按整数解析） |

### 2.3 可选列（冻结 v1，存在则解析；不存在不影响导入）
| 列名（逐字） | 说明 | 默认值 |
|---|---|---|
| 起始地址 | **点位绝对地址**（单位：寄存器/线圈），不支持 `40001/30001` 风格 | 空=不提供 |
| 长度 | profile 范围长度（寄存器/线圈数量），若为空将按点位推导最小值 | 0=按点位推导 |
| 缩放倍数 | 数值缩放 | `1.0` |
| 读取区域 | `Holding/Input/Coil/Discrete`（大小写不敏感） | 空=按数据类型推断（Bool→Coil，其它→Holding） |
| TCP:IP | TCP 连接参数 | 空=生成 skeleton profile + warning |
| TCP:端口 | TCP 连接参数 | 空=生成 skeleton profile + warning |
| 485:串口 | 485 连接参数（Windows `COMx` / Linux `/dev/ttyUSB0`） | 空=生成 skeleton profile + warning |
| 485:波特率 | 485 连接参数 | `9600` |
| 485:校验 | `None/Even/Odd`（大小写不敏感） | `None` |
| 485:数据位 | 5/6/7/8 | `8` |
| 485:停止位 | 1/2 | `1` |
| 超时ms | 单次请求超时 | `1000` |
| 重试次数 | 失败重试次数 | `0` |
| 轮询周期ms | 引擎轮询周期 | `1000` |

---

## 3. 地址基准（0/1-based）冻结规范

### 3.1 起始地址（点位绝对地址）基准（冻结 v1）
- `起始地址` 列（若提供）的**输入基准**：**1-based（v1）**
  - 例：Excel 填 `1` → 内部存储为 `0`
  - 例：Excel 填 `100` → 内部存储为 `99`
- command 选项：
  - `options.addressBase`：
    - `one`：按 1-based 解析（v1 默认）
    - `zero`：按 0-based 解析（不做减 1）

### 3.2 不支持的地址风格（强制）
- 不支持 `40001/30001` 风格输入。
- `起始地址` 必须是纯数字（寄存器/线圈索引），不带区号前缀。

---

## 4. 去重与 pointKey 规则（冻结 v1）

- `pointKey` 生成：**确定性**（禁止随机 UUID）
  - key 参与字段：`变量名称（HMI） + 通道名称 + 设备标识`
  - 算法：UUID v5（SHA1），name=`"{hmiName}|{channelName}|{deviceId}"`
- 去重策略（拍板）：
  - 若同一 `(HMI + 通道名称 + 设备标识)` 重复出现：
    - **first-wins**（保留第一条）
    - 产生 warning（可观测）

---

## 5. strict 校验模式（冻结行为）

### 5.1 strict=true（硬失败）
以下任一条件触发 → command 返回错误（不会返回 points/profiles）：
- 找不到目标 Sheet（返回可用 Sheet 列表）
- 缺少任何必填列（返回缺失列清单 + 实际列清单）
- 必填列存在但值非法：
  - `数据类型` 不在枚举
  - `字节序` 不在枚举
  - `协议类型` 非 `TCP/485`
  - `设备标识` 不是整数
  - `起始地址` 提供但在 `addressBase=one` 下为 0

### 5.2 strict=false（宽松导入）
- 尽量导入并返回 warnings：
  - Sheet 找不到 → fallback 到第一个 Sheet（并 warning）
  - 单行字段无法识别/缺失 → warning + 跳过或填默认（不会 panic）

---

## 6. 建议的最小示例（表头）

| 变量名称（HMI） | 数据类型 | 字节序 | 通道名称 | 协议类型 | 设备标识 | 起始地址 | 缩放倍数 | TCP:IP | TCP:端口 |
|---|---|---|---|---|---:|---:|---:|---|---:|
| TEMP_1 | UInt16 | ABCD | tcp-1 | TCP | 1 | 1 | 1.0 | 192.168.0.10 | 502 |
