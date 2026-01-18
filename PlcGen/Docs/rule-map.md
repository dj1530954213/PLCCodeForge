# Rule Map (Hollysys POU Serialization)

## Update Log
- 2025-01-16: Confirmed class hierarchy and serialization responsibility via x32dbg/IDA (Normal); CLDElement base handles core fields; added 1dmdl.dll offsets for Normal.
- 2025-01-17: Corrected CLDElement type_id to u8; clarified MFC CString length prefix (u8 or 0xFF+u16) with no alignment; confirmed Safety vs Normal element differences (Contact no geo in Safety; Box skips version u32s).
- 2025-01-18: Added variable table (Tag 0x15) structure from S09/S10/S11 (Normal/Safety): comment placement, init_flag (BOOL=0, TIME=7), Safety comment prefix (`extra u32 + \"CH\"`), retain/SOE markers (partial).
- 2025-01-19: Added S12/S13/S14 variable tail structure: Normal tail layout (retain + addr_id + mode + var_id + id2 + SOE), Safety tail layout (area_code + FF*6 + 0x00 0x42 + var_id + SOE), plus var_id evidence.
- 2025-01-20: Added CBaseDB::Serialize (Normal) field order: 4 strings before init_flag (name1/name2/comment_or_res/type), init_value stored after init_flag; clarified tail bytes after init_value (retain + u32/u32 + extra_str + mode + var_id + id2 + SOE); noted Safety SOE flag discrepancy (0x0100 vs 0x0001).
- 2025-01-21: Refined Safety variable structure to CBaseDB lang_count + (lang, comment) pairs; clarified Safety tail fields (u16=0xFFFF, addr_id u32, mode=0x42) and SOE bytes (00 01 vs 01 00).
- 2025-01-22: Added topology marker semantics (0x8001/0x8003/0x8005/0x8007) and var_id sequential evidence (TAG_1..TAG_4).
- 2025-01-23: Confirmed Normal topology uses CLDElement connection lists (conn_count + conn_ids) instead of 0x80xx token stream.
- 2025-01-24: Confirmed Safety area_code mapping (Local=0x04, M=0x00, R=0x03), Safety CLDNetwork header layout, var_id sequential rule, and Safety SOE rule (BOOL=0x0100, others=0x0001).
- 2025-01-25: Marked core reverse rules as complete; reprioritized open questions to non-core items.
- 2025-01-26: Confirmed Safety inline topology token `0x800C` (in-stream CLDBox definition), topology stream ends at next class signature, Safety CLDBox has no `0x0100` magic, and Safety variable table may be prefixed by `00 02 41 78`.

## Purpose
- Track verified rules derived from HEX + IDA for Normal/Safety variants.
- Each rule should cite sample IDs and offsets.

## Conventions
- Byte order: little endian.
- String encoding: GBK with MFC CString length prefix (u8 length, or 0xFF + u16 length).
- Alignment: header uses 4-byte padding after POU name; no alignment after CLDElement strings.
- Notation: offsets are hex, ranges are [start, end) in bytes.
- Sample IDs: Sxx-N (Normal) / Sxx-S (Safety).
- Element Type fields are u8 (Type ID).
- CLD* class signature lengths use u16 (e.g., `0A 00` for "CLDNetwork").
- CString serialization uses `AfxWriteStringLength` / `AfxReadStringLength`; observed samples use 1-byte (GBK) strings.

## Environment
- Normal: AutoThink V3.1.11B1Patch2 (Build@a99de9d3, 2025.01.03), clipboard format `POU_TREE_Clipboard_PLC`.
- Normal: 1dmdl.dll Base = `0x5F930000` (IDA mapping for class Serialize offsets).
- Safety: Safety FA-AutoThink V1.3.1Patch2, clipboard format `POU_TREE_Clipboard_ITCC`.

## Sample Index
| ID | Variant | POU | Structure | Source | Notes | Status |
| --- | --- | --- | --- | --- | --- | --- |
| S00-N | Normal | AUTO_GEN | 空白 POU | Docs/样本对比/普通型/空白POU.md | len=0x2000 | parsed |
| S00-S | Safety | AUTO_GEN | 空白 POU | Docs/样本对比/安全型/空白POU.md | len=0x2000 | parsed |
| S01-N | Normal | S01_TAG1 | 1 network + 1 NO contact + local var Tag1 | Docs/样本对比/普通型/S01_TAG1.MD | len=0x2000 | parsed |
| S01-S | Safety | S01_TAG1 | 1 network + 1 NO contact + local var Tag1 | Docs/样本对比/安全型/S01_TAG1.md | len=0x2000 | parsed |
| S02-N | Normal | S02_NC | 1 network + 1 NC contact + local var Tag1 | Docs/样本对比/普通型/S02_NC.md | len=0x2000 | parsed |
| S02-S | Safety | S02_NC | 1 network + 1 NC contact + local var Tag1 | Docs/样本对比/安全型/S02_NC.md | len=0x2000 | parsed |
| S03-N | Normal | S03_COIL | 1 network + 1 coil + local var Tag1 | Docs/样本对比/普通型/S03_COIL.MD | len=0x2000 | parsed |
| S03-S | Safety | S03_COIL | 1 network + 1 coil + local var Tag1 | Docs/样本对比/安全型/S03_COIL.MD | len=0x2000 | parsed |
| S04-N | Normal | S04_MOVE | 1 network + MOVE (no instance) + vars INPUT1/OUTPUT1 | Docs/样本对比/普通型/S04_MOVE.MD | len=0x2000 | parsed |
| S04-S | Safety | S04_MOVE | 1 network + MOVE (no instance) + vars INPUT1/OUTPUT1 | Docs/样本对比/安全型/S04_MOVE.MD | len=0x2000 | parsed |
| S05-N | Normal | S05_RS | 1 network + RS (instance TAG_RS) + vars TAG_SET/TAG_RESET/TAG_Q | Docs/样本对比/普通型/S05_RS.MD | len=0x2000 | parsed |
| S05-S | Safety | S05_RS | 1 network + RS (instance TAG_RS) + vars TAG_SET/TAG_RESET/TAG_Q | Docs/样本对比/安全型/S05_RS.MD | len=0x2000 | parsed |
| S06-N | Normal | S06_TP | 1 network + TP (instance TAG_TP) + vars IN/PT/Q/ET/StartTime + TP_IN/TP_Q/TP_ET | Docs/样本对比/普通型/S06_TP.MD | len=0x2000 | parsed |
| S06-S | Safety | S06_TP | 1 network + TP (instance TAG_TP) + vars IN/PT/Q/ET/StartTime + TP_IN/TP_Q/ET_Q | Docs/样本对比/安全型/S06_TP.MD | len=0x2000 | parsed |
| S07-N | Normal | S07_DESC | 1 network + MOVE + vars INPUT1/OUTPUT1 with comments | Docs/样本对比/普通型/S07_DESC.MD | len=0x2000 | parsed |
| S07-S | Safety | S07_DESC | 1 network + MOVE + vars INPUT1/OUTPUT1 with comments | Docs/样本对比/安全型/S07_DESC.MD | len=0x2000 | parsed |
| S08-N | Normal | S08_NETWORKS | multi-element network, vars TAG_1..TAG_4 | Docs/样本对比/普通型/S08_NETWORKS.md | len=0x2000 | parsed |
| S08-S | Safety | S08_NETWORKS | multi-element network, vars TAG_1..TAG_4 | Docs/样本对比/安全型/S08_NETWORKS.md | len=0x2000 | parsed |
| S09-N | Normal | S09_VAR_FLAGS | var flags: retain/SOE | Docs/样本对比/普通型/S09_VAR_FLAGS.MD | len=0x2000 | parsed |
| S09-S | Safety | S09_VAR_FLAGS | var flags: SOE | Docs/样本对比/安全型/S09_VAR_FLAGS.md | len=0x2000 | parsed |
| S10-N | Normal | S10_VAR_COMMENT | var comments (输入/输出) | Docs/样本对比/普通型/S10_VAR_COMMENT.MD | len=0x2000 | parsed |
| S10-S | Safety | S10_VAR_COMMENT | var comments (输入/输出) | Docs/样本对比/安全型/S10_VAR_COMMENT.md | len=0x2000 | parsed |
| S11-N | Normal | S11_VAR_TIME | TIME init values | Docs/样本对比/普通型/S11_VAR_TIME.MD | len=0x2000 | parsed |
| S11-S | Safety | S11_VAR_TIME | TIME init values | Docs/样本对比/安全型/S11_VAR_TIME.md | len=0x2000 | parsed |
| S12-N | Normal | S12_VAR_ID_SEQ | var id sequence | Docs/样本对比/普通型/S12_VAR_ID_SEQ.MD | len=0x2000 | parsed |
| S12-S | Safety | S12_VAR_ID_SEQ | var id sequence | Docs/样本对比/安全型/S12_VAR_ID_SEQ.MD | len=0x2000 | parsed |
| S13-N | Normal | S13_VAR_NAME_HASH | var id vs name | Docs/样本对比/普通型/S13_VAR_NAME_HASH.MD | len=0x2000; name=A/AAAAAA/B(中文名不可用) | parsed |
| S13-S | Safety | S13_VAR_NAME_HASH | var id vs name | Docs/样本对比/安全型/S13_VAR_NAME_HASH.MD | len=0x2000; name=A/AAAAAA/B(中文名不可用) | parsed |
| S14-S | Safety | S14_VAR_AREA | area code changes | Docs/样本对比/安全型/S14——VAR——AREA.MD | len=0x2000 | parsed |

## Rules
### Class Hierarchy & Stream Order (IDA)
- Clipboard data is a flattened CObList serialization; objects appear in list order in the stream.
- CLDNetwork inherits from CNetwork.
- CLDContact / CLDOutput / CLDBox inherit from CLDElement.
- CLDNetwork serializes only its own header; element objects follow as siblings in the list (no internal element loop).

### Factory Case Mapping (Normal, IDA)
- `CLDPOU::Factory` uses `v2 = *(a2 + 0xB8)` (DWORD) as case selector; low byte is `type_id` (u8), upper bytes are 0 or adjacent fields.
- case 0  -> sub_10038050 -> CLDElement
- case 1  -> sub_10051040 -> CLDOr
- case 2  -> sub_10005580 -> CLDAnd
- case 3  -> sub_100278F0 -> CLDBox
- case 4  -> sub_10027D60 -> CLDBracket
- case 5  -> sub_10035930 -> CLDContact
- case 6  -> sub_100606F0 -> CLDOutput
- case 7  -> sub_100893D0 -> CLDReturn
- case 8  -> sub_1004A5C0 -> CLDJump
- case 9  -> sub_100070A0 -> CLDAssign
- case 10 -> sub_1004CED0 -> CLDNetwork
- case 11 -> sub_1002E290 -> CLDBranches

### Factory Case Mapping (Safety, IDA)
- `CLDPOU::Factory` uses `v2 = *(a2 + 0xB0)` (DWORD) as case selector; low byte is `type_id` (u8).
- case 0 -> CLDElement (size 0xC0)
- case 1 -> CLDOr (size 0xF4)
- case 2 -> CLDAnd (size 0xE4)
- case 3 -> CLDBox (size 0x18C)
- case 4 -> CLDContact (size 0x110)
- case 5 -> CLDOutput (size 0x114)
- case 6 -> CLDReturn (size 0xF4)
- case 7 -> CLDJump (size 0x108)
- case 8 -> CLDAssign (size 0xD0)
- case 9 -> CLDNetwork (size 0x124)

### Header
- [S00-N] 0x0000: CString "AUTO_GEN" (len=0x08), 0x0009-0x000B padding to 4 bytes.
- [S00-N] 0x000C-0x000F: u32 timestamp present only in Normal (bytes `1D 4D 67 69`).
- [S00-N] 0x0010: CString "AUTO_GEN" (second name), 0x0019-0x001B padding.
- [S00-N] 0x001C-0x0027: 3x u32 = 0.
- [S00-N] 0x0028: u32 Language ID = 1.
- [S00-N] 0x002C: CString "BOOL"; 0x0031: u32 = 1; 0x0035: u32 = 0x00000100.
- [S00-N] 0x003A: second CString "BOOL" observed (present in Normal, absent in Safety). Meaning TBD.
- [S00-S] 0x0000: CString "AUTO_GEN" (len=0x08), 0x0009-0x000B padding to 4 bytes.
- [S00-S] 0x000C: CString "AUTO_GEN" (second name), 0x0015-0x0017 padding.
- [S00-S] 0x0018-0x002B: 5x u32 flags = [0x00000000, 0x00010000, 0x00000000, 0x00000000, 0x00000000].
- [S00-S] 0x002C: u32 Language ID = 1.
- [S00-S] 0x0030: CString "BOOL"; 0x0035: u32 = 1; 0x0039: u32 = 0x00000100; 0x003D: u32 = 0.
- [S01-N/S01-S] Header layout matches S00 (timestamp only in Normal, Safety flags include 0x00010000).

### Networks (CObList)
- [IDA/Normal/1dmdl.dll] CLDNetwork::Serialize offset 0x501F0.
- [IDA/Normal/1dmdl.dll] CNetwork::Serialize offset 0x387B0 (VA 0x5F9687B0); CString::Read @ 0x5F980234 / 0x5F980242.
- [IDA/Normal] Base field order: `id(u32)` -> `type(u16)=0x000A` -> `flag(u16)=0x0001` -> `rung_id(u16)` -> `pad(u16)=0`, then `label`/`comment` CStrings.
- [S00-N] 0x0084: list header begins with `03 00 FF FF 00 00 09 00 "CLDAssign"` (sequence observed: CLDAssign -> CLDNetwork -> CLDElement).
- [S00-S] 0x0060: same list header sequence as Normal (offset shift due to header differences).
- [S01-N] 0x0084: list header is `block_hint=0x0004` + class sig `CLDNetwork` (class sig name length uses u16: `0A 00`).
- [S01-S] 0x0060: list header is `block_hint=0x0004` + class sig `CLDNetwork` (class sig name length uses u16: `0A 00`).
- [S01-N] Network object layout (after class sig): `id(u32)=2`, `type(u16)=0x000A`, `flag(u16)=1`, `rung_id(u16)=3`, `pad(u16)=0`, then `label=""`, `comment=""`, followed immediately by next class sig `FF FF`.
- [S01-S] Network object layout: `id(u32)=2`, `type(u16)=0x0009`, `flag(u32)=1`, `rung_id(u32)=3`, then `label=""`, `comment=""`, followed immediately by next class sig `FF FF` (Safety 布局已由 IDA 确认，无 padding)。
- [S08-N/S08-S] Multi-element network introduces topology markers after `label/comment` (not present in single-element samples).
  - Markers (little-endian in stream): `0x8001`(01 80)=Branch Open, `0x8003`(03 80)=Branch Close, `0x8005`(05 80)=Series Connector, `0x8007`(07 80)=Terminator.
  - 解析方式：使用 u16 标记递归构建梯形图拓扑；元素被这些标记“夹住”以形成图结构。
  - Normal 版不使用 0x80xx 标记，改为在 CLDElement 基类中写入 `conn_count + conn_ids` 形成图连接表（S08-N 证据）。
  - Safety 拓扑流没有全局结束符依赖，读取直到遇到下一个 Class Signature (`0xFFFF`) 或数据结束。
  - Safety 支持 `0x800C` 内联元件定义：`0x800C` 后直接跟紧凑 CLDBox 结构（无类签名）。
  - Safety 拓扑中的普通元素引用为 `id(u32) + type_id(u16)`（无 `0x80xx` 前缀），部分样本后跟 4 字节占位。

### Elements
- [IDA/Normal/LDMDL.dll] CLDContact::Serialize (sub_10034920) calls CLDElement::Serialize (sub_100387B0, `ecx=this`, `arg0=CArchive*`) then reads/writes extra fields: `byte @ +0xDC (220)` and `CString @ +0x244 (580)` via helper `sub_1000A380/sub_10022780`.
- [IDA/Normal/LDMDL.dll] CLDElement::Serialize (sub_100387B0) reads/writes: `u32 @ +0x04`, `type_id(u8) @ +0xB8`, `name @ +0xBC`, `comment @ +0xC8`, `desc @ +0xCC`, then `conn_count(u32) @ +0x4C` followed by `conn_count` x `u32` via helpers `sub_10037610/sub_100375D0`.
- [IDA/Normal/LDMDL.dll] `sub_1000A380` = CString write (`AfxWriteStringLength` + `CArchive::Write`), `sub_10022780` = CString read (`AfxReadStringLength` + `CArchive::Read`, supports 1- or 2-byte char size).
- [IDA/Normal/LDMDL.dll] CLDOutput::Serialize (sub_1005F5E0) calls CLDElement::Serialize then reads/writes: `byte @ +0xDC` and `byte @ +0x24C`, optional `byte @ +0x374` if `CAppGlobalFunc::GetSerilizeVersion()` is true, plus `CString @ +0x244`.
- [IDA/Normal/LDMDL.dll] CLDBox::Serialize (sub_10022A10) calls CLDElement::Serialize then reads/writes:
  - `u32 @ +0x164` and `u32 @ +0x168` (write always; read only if `GetSerilizeVersion() >= 6`).
  - `byte @ +0x190` (Box flag).
  - `CString @ +0x194` (purpose TBD).
  - `input_count(u32) @ +0x124`, then `count` x InputPin::Serialize (objects size 0x244, ctor sub_1004A2B0).
  - `output_count(u32) @ +0x140`, then `count` x OutputPin::Serialize (objects size 0x23C, ctor sub_10059950).
- [IDA/Safety/LDMDL.dll] CLDContact::Serialize calls CLDElement::Serialize then reads/writes only `byte @ +0xC4 (196)`; no extra CString call observed (no `sub_1000A380/sub_10022780`).
- [IDA/Safety/LDMDL.dll] CLDBox::Serialize calls CLDElement::Serialize then reads/writes:
  - `byte @ +0x164 (356)` (Box flag).
  - `CString @ +0x168 (360)` via inline `AfxWriteStringLength` / `CArchive::Write` and reader `sub_100152F0` (confirmed MFC CString read).
  - `input_count(u32) @ +0xFC (252)`, then `count` x InPin::Serialize (objects size 0xD0).
  - `output_count(u32) @ +0x118 (280)`, then `count` x OutPin::Serialize (objects size 0xC0).
  - No version-gated `u32` fields observed in Safety (unlike Normal `+0x164/+0x168`).
- [Safety] CLDBox 结构为 `flag(u8)` 直接跟实例名 CString（无 `0x0100` 魔数）；`0x800C` 内联盒子使用同一布局。
- [IDA/Normal/Safety] Base field order: `id(u32)` -> `type_id(u8)` -> `name` CString -> `comment` CString -> `desc` CString -> `conn_count(u32)` -> `conns[u32] * count`.
- [Observed/Safety] `conn_count` 仍存在且样本恒为 0（含 0x800C 内联元件）。
- [IDA/Normal] Type values: Contact=0x05, Coil=0x06 (u8).
- [Observed/Safety] Type values: Contact=0x04, Coil=0x05 (u8).
- [IDA/Normal/Safety] `05 00` observed in HEX = `type_id=0x05` followed by empty name length (`0x00`).
- Box (no instance): derived layout adds pin lists after CLDElement base (exact fields TBD).
- Box (with instance): derived layout adds pin lists after CLDElement base (exact fields TBD).
- [S00-N] 0x00E3: CString len=0x13 with GBK text "请在此处添加注释..." (default comment). Not present in S00-S.
- [S01-N] CLDContact object (0x00B8): `id(u32)=4`, `type_id(u8)=0x05`, `name=""`, then `comment/desc` CStrings and conn list; extra fields (flag/geo) follow in CLDContact::Serialize.
- [S01-S] CLDContact object (0x0092): `id(u32)=4`, `type_id(u8)=0x04`, `name=""`, then `comment/desc` CStrings and conn list; Safety has only `flag(u8)` after base.
- [S02-N] CLDContact object: `id(u32)=4`, `type_id(u8)=0x05`, NC marker appears in contact-specific trailing data (flag/geo).
- [S02-S] CLDContact object: `id(u32)=7`, `type_id(u8)=0x04`, NC marker appears in contact-specific trailing data (flag only).
- [S03-N] Coil uses class `CLDOutput` (not CLDCoil). Object: `id(u32)=4`, `type_id(u8)=0x06`, base strings follow; output-specific flags/geo follow.
- [S03-S] Coil uses class `CLDOutput`. Object: `id(u32)=4`, `type_id(u8)=0x05`, base strings follow; output-specific flags/geo follow.
- [S01-N] List node `CLDElement` (0x00DA): bytes `01 00 00 00 00 00 00 00 01 00 00 00 02 00 00 00` (field meanings TBD; not the base-class fields of contact/coil).
- [S01-S] List node `CLDElement` (0x00B1): bytes `01 00 00 00 00 00 01 00 00 00 02 00 00 00` (field meanings TBD; not the base-class fields of contact/coil).
- [S01-N] CLDAssign object (0x00F9): starts `03 00 00 00 09 00 00 00 01 00 00 00 04 00 00 00 ...` then GBK comment string appears.
- [S01-S] CLDAssign object (0x00CE): starts `03 00 00 00 08 00 01 00 00 00 04 00 00 00 00 02 41 34 08 "S01_TAG1" ...` (field meanings TBD).
- [S04-S] CLDBox (MOVE, no instance) payload: `00 00 00 00 03 00 02 00 00 00 02 "EN" 00 00 06 "INPUT1" 02 00 00 00 03 "ENO" 00 00 07 "OUTPUT1"` (two 0x00 bytes between pin name and var name). EN/ENO unbound -> empty var string. Unknown field `0x0003` after padding.
- [S04-N] CLDBox (MOVE, no instance) payload contains extra padding + `FF FF FF FF`, and pin names `EN/IN/ENO/OUT` with var names `INPUT1/OUTPUT1`; unbound vars appear as `"???"` and 0xFFFFFFFF separators (EN/ENO not bound to variables). Exact field map TBD.
- [S05-S] CLDBox (RS, instance) payload: `00 00 00 00 01 06 "TAG_RS"` then `input_count(u32)=3` with pairs `(EN,"") (Set,"TAG_SET") (Reset,"TAG_RESET")`, then `output_count(u32)=2` with `(ENO,"") (Q,"TAG_Q")`. EN/ENO unbound -> empty var string.
- [S05-N] CLDBox (RS, instance) payload: `00` x10 + `FF FF FF FF`, then `01 06 "TAG_RS"`; `input_count(u32)=3`, each input entry = `flag(u16)=1 + name + var + addr(u32)=0xFFFFFFFF` (unbound var uses `"???"`); `output_count(u32)=2`, each output entry = `flag(u16)=1 + name + var` (no addr). EN/ENO unbound -> `"???"` placeholder.
- [S06-N] CLDBox (TP, instance TAG_TP): `input_count=3` `(EN->???, IN->TP_IN, PT->T#3S)` then `output_count=3` `(ENO->???, Q->TP_Q, ET->TP_ET)`; instance-mode pin list (flag+addr form).
- [S06-S] CLDBox (TP, instance TAG_TP): compact pin list; inputs `(EN,"") (IN,"TP_IN") (PT,"T#3S")`, outputs `(ENO,"") (Q,"TP_Q") (ET,"ET_Q")`.
- [S08-N/S08-S] Elements include `CLDOutput(TAG_2)` and `CLDContact(TAG_1)`; `TAG_3/TAG_4` appear in topology list without full class sig (mapping TBD).

### Variables (Tag 0x15)
- Normal 基本结构（CBaseDB::Serialize, S09-N/S10-N/S11-N/S12-N/S13-N）：
  - `0x15` -> `name1(MfcString)` -> `name2(MfcString)` -> `comment_or_res(MfcString)` -> `type(MfcString)` -> `init_flag(u8)` -> `init_value(MfcString)` -> `tail(...)`.
  - `name1` 为变量名；`name2` 样本中为空。
  - `comment_or_res` 在 S10-N 中存放变量注释（如 `输入变量/输出变量`），IDA 中该字段会经过 `GetStringTOResourceID` 处理。
  - [S11-N] TIME 变量 `init_flag = 0x07`，BOOL 变量 `init_flag = 0x00`。
  - `init_value` 为初始化字符串（如 `FALSE` / `T#3S`）。
- Normal tail 结构（S09-N/S10-N/S11-N/S12-N/S13-N）：
  - `retain(u8)`：`0x03`=掉电保持，`0x04`=不保持。
    - `addr_id(u64)`：tail 中连续两个 `u32` 组成；S12/S13 为 `0xFFFFFFFFFFFFFFFF`（未绑定），S09/S10/S11 为非 FF 值。
  - `extra_str(MfcString)`：`this+60`，样本中为空（仅写入 `00`）。
  - `mode(u8)`：S12/S13 为 `0x06`，S09/S10/S11 为 `0x16`（含义待确认）。
    - `var_id(u16)`：顺序句柄（Sequential），按创建顺序递增。
  - `retain_mirror(u8)`：与 retain 同步（0/1）。
  - `id2(u32)`：随变量变化，疑似内部索引/时间戳（待确认）。
  - `soe(u16)`：`0x0001`=SOE, `0x0000`=非 SOE。
- Safety 基本结构（CBaseDB::Serialize, S09-S/S10-S/S11-S/S12-S/S13-S/S14-S）：
  - 变量表前可能存在外部容器头 `00 02 41 78`（AI_CONVERT 样本），解析时需跳过再读 `count(u32)`。
  - `0x15` -> `name1(MfcString)` -> `name2(MfcString)` -> `lang_count(u32)` -> `(lang(MfcString), comment(MfcString)) * count` -> `type(MfcString)` -> `init_flag(u8)` -> `init_value(MfcString)` -> `tail(...)`.
  - `name2` 样本中为空（Hex 为 `00`）。
  - `lang_count` 样本中为 0 或 1；为 1 时，语言标识固定 `"CH"`，后跟中文注释（S10-S）。
  - [S11-S] TIME 变量 `init_flag = 0x07`，BOOL 变量 `init_flag = 0x00`。
  - Safety tail 结构（S09-S/S12-S/S13-S/S14-S）：
     - `area_code(u8)`：Local=`0x04`，M=`0x00`，R=`0x03`（S14-S 证据）。
     - `flag_78(u16)`：样本固定 `0xFFFF`（疑似保留/标志位）。
     - `addr_id(u32)`：样本为 `0xFFFFFFFF`（未绑定）。
     - `extra_str(MfcString)`：样本为空（仅写入 `00`）。
     - `mode(u8)`：样本固定 `0x42`。
     - `var_id(u16)`：顺序句柄（Sequential），按创建顺序递增。
     - `soe_bytes(u8,u8)`：与 SOE 值对应（BOOL=低0高1，TIME=低1高0）；IDA 显示此两字节分别在版本 `>=0x18` 与 `>=0x24` 时写入。
- [S01-N/S01-S] Tag1 变量使用 `BOOL` + `FALSE`，结构与上述格式一致（comment 为空）。
- [S04-N/S04-S] `INPUT1/OUTPUT1` 为 `INT`，初始值 `"0"`；Safety 在 `extra/comment` 前存在固定前缀（同 S10-S 结构）。
- [S05-N/S05-S] RS 变量存在额外资源字符串与元数据字段（如 `"@139@"` 等），需单独解析。
- [S06-N/S06-S] TP 变量含 TIME/BOOL 混合字段，Normal 含资源字符串，Safety 含双语说明字符串，结构待细化。
- [S08-N/S08-S] Tag 0x15 entries for `TAG_1..TAG_4`, type `BOOL`, init `FALSE` (multi-element network).

### Variable Tail Notes (New Evidence)
- [S12-N/S12-S] `var_id` 在 SEQ_A/B/C 中递增（0x1F4B/0x1F4C/0x1F4D），Normal/Safety 一致。
- [S08-S] TAG_1..TAG_4 的 `var_id` 连续递增（0x0583/0x0584/0x0585...），与顺序句柄规则一致。
- [S13-N/S13-S] 早期样本显示 `var_id` 随名称变化（A=0x8AD5, AAAAAA=0x5CAF, B=0x8AD6）；现统一解释为创建顺序差异导致的非连续值，最终以顺序句柄为准。
- [S14-S] `area_code` 随区域切换改变（示例：AREA_X=0x00, AREA_Y=0x03），默认区域为 0x04。
- [S09-S/S11-S] Safety 的 SOE 值由两个字节组成：BOOL 样本表现为 `00 01`（等价 0x0100），TIME 样本为 `01 00`（等价 0x0001）。

### IDA-Verified Objects (Normal)
| Sample | Class | Verified | Notes |
| --- | --- | --- | --- |
| S01-N | CLDNetwork | Yes | ID/Type/Flag/Rung + Label/Comment from CNetwork/CLDNetwork::Serialize |
| S01-N | CLDContact | Yes | CLDElement base handles all fields |
| S03-N | CLDOutput | Yes | CLDElement base handles all fields |

### Tail / Padding
- [S00-N/S] total length = 0x2000; last non-zero byte at 0x010F (Normal) and 0x00C5 (Safety), rest zero.
- [S00-N] 0x0060-0x0083: 36-byte block present only in Normal (`00 69 B7 3B ...` + `EF 41 ...` + `EF 41 00 00`), meaning TBD.

### Generator Sketch (Normal, IDA)
```cpp
// Header (partial)
ar << pouName;           // CString, e.g. "AUTO_GEN"
ar.Write(&timestamp, 4); // Normal only
ar.Write(&langId, 4);    // 1 = LD

// CLDNetwork
ar.Write(&netId, 4);
ar.Write(&netType, 2);   // 0x000A
ar.Write(&flag, 2);      // 0x0001
ar.Write(&rungId, 2);
ar.Write(&pad, 2);
ar << label;             // CString
ar << comment;           // CString

// CLDElement base (Contact/Coil/Box)
ar.Write(&elemId, 4);
ar.Write(&typeId, 1);    // Contact=0x05, Coil=0x06 (Normal)
ar << name;              // CString (may be empty)
ar << comment;           // CString
ar << desc;              // CString
ar.Write(&connCount, 4);
for (...) ar.Write(&connId, 4);
// tail: contact/output/box specific fields + (CLDBox adds pin lists)
```
Note: Header fields beyond timestamp/langId remain inferred from HEX samples.

## Open Questions (Priority)
- P1 兼容性/完整性（影响更多指令/显示一致性）
  - 变量 `addr_id/mode/id2` 的生成规则（绑定地址/编译后变化规则），Normal/Safety 各自的来源差异。
  - `CLDGeo` 结构（Contact/Output/Box 的几何字段）具体字节布局。
  - Normal `CLDBox` 的额外填充/`FF FF FF FF` 规则，实例/非实例模式的完整字段映射。
  - `CLDBox` 的 `CString @ +0x194`（或 Safety 对应字段）含义与格式。
  - `CLDAssign` 对象字段结构与用途。
  - 列表节点 `CLDElement`（非实体）字节含义。
  - Safety `area_code` 的完整区域映射（除 0x00/0x03/0x04 之外的取值）。
  - `extra_str`（Normal this+60 / Safety this+84）非空样本与用途。
- P2 体验/细节（不影响基础功能但影响还原度）
  - Header 中第二个 `"BOOL"` 字段含义（Normal only）。
  - Normal 头部 `0x0060-0x0083` 的 36 字节块含义。
  - RS/TP 等变量的扩展元数据（资源字符串/双语标签）完整结构。
  - 其他类（`CLDOr/CLDAnd/CLDBracket/CLDReturn/CLDJump/CLDBranches`）序列化细节。
