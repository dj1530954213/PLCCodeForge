# Rule Map (Hollysys POU Serialization)

## Purpose
- Track verified rules derived from HEX + IDA for Normal/Safety variants.
- Each rule should cite sample IDs and offsets.

## Conventions
- Byte order: little endian.
- String encoding: GBK with MFC CString length prefix.
- Alignment: 4-byte boundary after CString where observed.
- Notation: offsets are hex, ranges are [start, end) in bytes.
- Sample IDs: Sxx-N (Normal) / Sxx-S (Safety).

## Environment
- Normal: AutoThink V3.1.11B1Patch2 (Build@a99de9d3, 2025.01.03), clipboard format `POU_TREE_Clipboard_PLC`.
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

## Rules
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
- [S00-N] 0x0084: list header begins with `03 00 FF FF 00 00 09 00 "CLDAssign"` (sequence observed: CLDAssign -> CLDNetwork -> CLDElement).
- [S00-S] 0x0060: same list header sequence as Normal (offset shift due to header differences).
- [S01-N] 0x0084: list header is `block_hint=0x0004` + class sig `CLDNetwork` (class sig name length uses u16: `0A 00`).
- [S01-S] 0x0060: list header is `block_hint=0x0004` + class sig `CLDNetwork` (class sig name length uses u16: `0A 00`).
- [S01-N] Network object layout (after class sig): `id(u32)=2`, `type(u16)=0x000A`, `pad(u16)=0`, `flag(u16)=1`, `pad(u16)=0`, `rung_id(u16)=3`, `pad(u16)=0`, then `label=""`, `comment=""`, followed immediately by next class sig `FF FF`.
- [S01-S] Network object layout: `id(u32)=2`, `type(u16)=0x0009`, `flag(u32)=1`, `rung_id(u32)=3`, then `label=""`, `comment=""`, followed immediately by next class sig `FF FF`.

### Elements
- Box (no instance): TBD
- Box (with instance): TBD
- Contact/Coil: TBD
- [S00-N] 0x00E3: CString len=0x13 with GBK text "请在此处添加注释..." (default comment). Not present in S00-S.
- [S01-N] CLDContact object (0x00B8): `id(u32)=4`, `type(u8)=0x05`, `name="Tag1"`, then 8x 0x00 bytes before next class sig. Subtype/address fields TBD.
- [S01-S] CLDContact object (0x0092): `id(u32)=4`, `type(u8)=0x04`, `name="Tag1"`, then 5x 0x00 bytes before next class sig. Subtype/address fields TBD.
- [S02-N] CLDContact object: `id(u32)=4`, `type(u8)=0x05`, `name="Tag1"`, trailer `00 00 00 00 00 00 01 00` (NC marker vs S01 NO).
- [S02-S] CLDContact object: `id(u32)=7`, `type(u8)=0x04`, `name="Tag1"`, trailer `00 00 00 00 01` (NC marker vs S01 NO).
- [S03-N] Coil uses class `CLDOutput` (not CLDCoil). Object: `id(u32)=4`, `type(u8)=0x06`, `name="Tag1"`, trailer `00` x10.
- [S03-S] Coil uses class `CLDOutput`. Object: `id(u32)=4`, `type(u8)=0x05`, `name="Tag1"`, trailer `00` x6.
- [S01-N] CLDElement object (0x00DA): bytes `01 00 00 00 00 00 00 00 01 00 00 00 02 00 00 00` (field meanings TBD).
- [S01-S] CLDElement object (0x00B1): bytes `01 00 00 00 00 00 01 00 00 00 02 00 00 00` (field meanings TBD).
- [S01-N] CLDAssign object (0x00F9): starts `03 00 00 00 09 00 00 00 01 00 00 00 04 00 00 00 ...` then GBK comment string appears.
- [S01-S] CLDAssign object (0x00CE): starts `03 00 00 00 08 00 01 00 00 00 04 00 00 00 00 02 41 34 08 "S01_TAG1" ...` (field meanings TBD).
- [S04-S] CLDBox (MOVE, no instance) payload: `00 00 00 00 03 00 02 00 00 00 02 "EN" 00 00 06 "INPUT1" 02 00 00 00 03 "ENO" 00 00 07 "OUTPUT1"` (two 0x00 bytes between pin name and var name). EN/ENO unbound -> empty var string. Unknown field `0x0003` after padding.
- [S04-N] CLDBox (MOVE, no instance) payload contains extra padding + `FF FF FF FF`, and pin names `EN/IN/ENO/OUT` with var names `INPUT1/OUTPUT1`; unbound vars appear as `"???"` and 0xFFFFFFFF separators (EN/ENO not bound to variables). Exact field map TBD.
- [S05-S] CLDBox (RS, instance) payload: `00 00 00 00 01 06 "TAG_RS"` then `input_count(u32)=3` with pairs `(EN,"") (Set,"TAG_SET") (Reset,"TAG_RESET")`, then `output_count(u32)=2` with `(ENO,"") (Q,"TAG_Q")`. EN/ENO unbound -> empty var string.
- [S05-N] CLDBox (RS, instance) payload: `00` x10 + `FF FF FF FF`, then `01 06 "TAG_RS"`; `input_count(u32)=3`, each input entry = `flag(u16)=1 + name + var + addr(u32)=0xFFFFFFFF` (unbound var uses `"???"`); `output_count(u32)=2`, each output entry = `flag(u16)=1 + name + var` (no addr). EN/ENO unbound -> `"???"` placeholder.

### Variables (Tag 0x15)
- [S01-N] Var entry starts at 0x013B: `15 04 "Tag1" 00 00 04 "BOOL" 00 05 "FALSE" 04 FF FF FF FF FF FF FF FF 00 06 D3 C5 00 F2 C3 67 69 ...` (structure TBD).
- [S01-S] Var entry starts at 0x00EE: `15 04 "Tag1" 00 00 00 00 00 04 "BOOL" 00 05 "FALSE" 04 FF FF FF FF FF FF 00 42 D3 C5 01 00 00 00 ...` (structure TBD).
- [S02-N/S02-S] Tag1 variable continues to use `BOOL` + `FALSE` (layout matches S01 with NC contact).
- [S03-N/S03-S] Tag1 variable continues to use `BOOL` + `FALSE` (layout matches S01 with coil).
- [S04-N] Variables: `INPUT1` and `OUTPUT1` are `INT`, init value `"0"`; two Tag 0x15 entries.
- [S04-S] Same variables (`INPUT1`, `OUTPUT1`, `INT`, `"0"`), but extra zero padding vs Normal in the pre-type section.
- [S05-N] Variables: `Set/Reset/Q` and `TAG_SET/TAG_RESET/TAG_Q`, type `BOOL`, init `FALSE`; Normal includes extra metadata strings like `"@139@"`, `"@140@"`, `"@141@"` and additional numeric fields.
- [S05-S] Same variable set for RS; Safety entries include extra prefix fields (e.g., `00 01 00 00 00 02 45 4E ...`) before `BOOL/FALSE`, structure TBD.

### Tail / Padding
- [S00-N/S] total length = 0x2000; last non-zero byte at 0x010F (Normal) and 0x00C5 (Safety), rest zero.
- [S00-N] 0x0060-0x0083: 36-byte block present only in Normal (`00 69 B7 3B ...` + `EF 41 ...` + `EF 41 00 00`), meaning TBD.

## Open Questions
- TBD
