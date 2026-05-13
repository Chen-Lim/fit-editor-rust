# CLI Command Reference Design

## 命令行接口设计

> **2026-05-13 状态说明：** 本文档同时描述「已实现」与「设计意图」。原始稿件包含一批
> 在实际 `src/cli.rs` 中尚未存在的 flag（例如 `encode --protocol/--profile/--validate`，
> `edit --append/--update-header`，`merge --sort/--no-dedup`）。这些都用 ⚠️ **(planned)** 标注；
> 真实子命令以 `cargo run -- <cmd> --help` 输出为准。同时已知存在但已识别的 bug 用 ⚠️ **(bug)** 标注，
> 指向 [Report.html](../Report.html) / [Rroadmap.md `Phase 4.6`](./Rroadmap.md)。

### 全局选项

```
fit-editor [OPTIONS] <COMMAND>

OPTIONS:
  -v, --verbose        详细输出（显示解码警告等）✅ 实现
  -q, --quiet          静默模式（仅输出结果，无装饰）
                       ⚠️ (bug, H3) 已声明但 12 个子命令均未读取该 flag。
                       Phase 4.6 必须实现或从 Cli struct 删除。
      --no-color       禁用彩色输出 ✅ 实现（TTY 自动检测亦会关闭颜色）
  -h, --help           打印帮助 ✅
  -V, --version        打印版本 ✅
```

### 命令速查

```
fit-editor validate <FILE>                          # 校验文件完整性
fit-editor info <FILE>                              # 显示文件元信息
fit-editor dump <FILE> [OPTIONS]                    # 查看消息内容
fit-editor export <FILE> [OPTIONS]                  # 导出为 JSON/CSV/GPX
fit-editor encode <JSON> -o <OUTPUT>                # JSON 编码为 FIT
fit-editor edit <FILE> [OPTIONS] -o <OUTPUT>        # 编辑 FIT 文件
fit-editor merge <FILE>... -o <OUTPUT>              # 合并 FIT 文件
fit-editor split <FILE> [OPTIONS] -o <PREFIX>       # 拆分 FIT 文件
fit-editor diff <FILE1> <FILE2>                     # 对比差异
fit-editor summary <FILE>                           # 运动摘要
fit-editor hexdump <FILE> [OPTIONS]                 # 十六进制查看
fit-editor batch <GLOB> -- <COMMAND>                # 批量处理
```

### 详细命令设计

#### validate

```
fit-editor validate <FILE>

校验 FIT 文件的结构完整性。
  - 检查文件签名 (".FIT")
  - 校验 Header CRC
  - 校验 File CRC
  ⚠️ (bug, H2) 当前实现不调用 Decoder，无法检测 definition/data mismatch
     或字段类型异常等消息层错误。Phase 4.6 必须扩展为:
       - 调用 Decoder::read_all 并报告 errors.len()
       - --verbose 下逐条打印 decoder error
       - errors 非空时返回 exit code 2

EXIT CODES (Phase 4.6 之后):
  0  文件有效，无 decoder 警告
  1  文件无效（CRC、签名、IO）
  2  CRC 通过但 decoder 报告非零错误（消息层不健康）

EXIT CODES (当前 0.1.0-alpha):
  0  CRC + 签名通过
  1  其他

EXAMPLES:
  fit-editor validate Activity.fit
  fit-editor validate *.fit          # 每个文件单独校验
```

#### info

```
fit-editor info <FILE>

显示 FIT 文件的元信息摘要。

OUTPUT:
  File:            Activity.fit
  Protocol:        2.0 (0x20)
  Profile:         21.200
  Data Size:       1.2 MB (1,234,567 bytes)
  CRC:             0xAB12 — valid
  Messages:        1,234 total
    file_id:       1
    session:       1
    lap:           5
    record:        1,200
    event:         20
    device_info:   7
```

#### dump

```
fit-editor dump <FILE> [OPTIONS]

OPTIONS:
  -m, --message <TYPE>     仅显示指定类型的消息 (e.g. record, session)
  -f, --field <NAME>       仅显示指定字段 (配合 --message)
  -n, --limit <N>          最多显示 N 条消息
      --raw                显示原始值（跳过 scale/offset、enum 转换）
      --no-color           禁用彩色
      --compact            紧凑模式（单行/消息）

OUTPUT FORMAT (default):
  [0] file_id
    manufacturer = garmin
    product = 3121
    time_created = 2024-01-15T10:30:00+00:00
    type = activity

  [1] session
    sport = running
    total_distance = 5000.0 m
    total_timer_time = 1800.0 s
    avg_heart_rate = 155 bpm
    ...

  [2] record
    timestamp = 2024-01-15T10:30:01+00:00
    position_lat = 39.9042°
    position_long = 116.4074°
    heart_rate = 120 bpm
    ...

EXAMPLES:
  fit-editor dump Activity.fit
  fit-editor dump Activity.fit --message record --field heart_rate --limit 10
  fit-editor dump Activity.fit --raw
```

#### export

```
fit-editor export <FILE> [OPTIONS]

OPTIONS:
  -f, --format <FORMAT>    输出格式: json, csv, gpx [default: json] ✅
  -o, --output <FILE>      输出文件路径 (默认 stdout) ✅
  -m, --message <TYPE>     仅导出指定类型 ✅
      --pretty             格式化 JSON (默认开启) ✅
      --compact            紧凑 JSON ✅

JSON STRUCTURE:
  {
    "file_header": { "protocol_version": "...", "profile_version": ..., "data_size": ... },
    "messages": [
      { "type": "record", "index": 0, "fields": { ... } },
      ...
    ]
  }

CSV FORMAT (默认导出 record messages，可用 --message 切换):
  timestamp,position_lat,position_long,heart_rate,speed,...
  2024-01-15T10:30:01Z,39.9042,116.4074,120,2.5,...

KNOWN ISSUES:
  ⚠️ (bug, C2, export.rs:205-216) GPX 导出对没有 GPS 坐标但有 timestamp/altitude 的
     record 会输出 `<trkpt lat="0.0000000" lon="0.0000000">`。室内活动（跑步机、骑行台、
     游泳）会被写成 "Null Island" 假轨迹点。Phase 4.6 必须改为:
       if !has_coords { continue; }
     并对非空间数据考虑用 <wpt> 或迁移到 TCX。

EXAMPLES:
  fit-editor export Activity.fit -f json -o activity.json
  fit-editor export Activity.fit -f csv -o records.csv --message record
  fit-editor export Activity.fit -f gpx -o track.gpx
```

#### encode

```
fit-editor encode <JSON> -o <OUTPUT>

从 JSON 文件编码为 FIT 文件。

INPUT JSON FORMAT:
  与 export --format json 的输出格式兼容，实现 round-trip。

OPTIONS:
  -o, --output <FILE>      输出 FIT 文件路径 (required) ✅ 实现
      --protocol <VER>     ⚠️ (planned) 协议版本 [default: 0x20]
      --profile <VER>      ⚠️ (planned) Profile 版本 [default: 21200]
      --validate           ⚠️ (planned) 实际行为：当前 encode 总是在写盘前调用 check_integrity，
                              所以此 flag 暂时是空操作；保留以备未来需要关闭快速 round-trip

KNOWN ISSUES:
  ⚠️ (bug, C1, encode.rs:123) JSON enum 字段解码用 Box::leak，常驻泄漏
  ⚠️ (bug, M3, encode.rs:97-112) JSON 数值 → 整数静默截断:
       - n.as_f64().unwrap_or(0.0) 丢失 u64 > 2^53 的精度
       - f as u64 / f as i64 对超界饱和，不报错
     Phase 4.6 改为: 优先 n.as_u64() / n.as_i64()，超界返回 Err

EXAMPLES:
  fit-editor encode activity.json -o modified.fit
```

#### edit

```
fit-editor edit <FILE> [OPTIONS] -o <OUTPUT>

OPTIONS:
  -o, --output <FILE>                     输出文件路径 (required) ✅ 实现
      --set <FIELD=VALUE>                 设置字段值 (可多次使用) ✅ 实现
      --remove-message <TYPE>             删除指定类型消息 ✅ 实现
      --append <JSON>                     ⚠️ (planned) 追加消息（从 JSON 文件）
      --update-header <field=value>       ⚠️ (planned) 修改文件头元数据

FIELD PATH SYNTAX:
  session.total_distance=5000.0           修改 session 消息的 total_distance
  session[0].sport=cycling                修改第一个 session 的 sport
  file_id.product=1234                    修改 file_id 的 product 字段

KNOWN ISSUES:
  ⚠️ (bug, C1, edit.rs:167) enum 字段 --set 用 Box::leak 实现，重复编辑/批量场景会泄漏

EXAMPLES:
  fit-editor edit Activity.fit --set session.total_distance=5000.0 -o modified.fit
  fit-editor edit Activity.fit --remove-message device_info -o cleaned.fit
```

#### merge

```
fit-editor merge <FILE1> <FILE2> [FILE...] -o <OUTPUT>

按时间戳合并多个 FIT 文件。

OPTIONS:
  -o, --output <FILE>      输出文件路径 (required) ✅ 实现
      --sort <ORDER>       ⚠️ (planned) 时间排序: asc, desc [default: asc]
      --no-dedup           ⚠️ (planned) 不去除重复消息

KNOWN ISSUES:
  ⚠️ (bug, C3, merge.rs:31-35, 53-55)
     - 当前实现不去重 file_id / file_creator / device_info 等 metadata-only 消息，
       导致输出包含 N 份 file_id，不符合 FIT 规范，下游消费者
       (Garmin Connect / Strava / fitparse) 可能拒绝
     - extract_timestamp 用 Value::as_u64()，但 SDK 默认解码后 timestamp 是
       Value::DateTime，as_u64 永远返回 None，所有消息被视作"无 timestamp"，
       实际行为退化为按文件顺序 concat（不是按时间戳排序）
     Phase 4.6 必须:
       - 用 match { Value::DateTime(dt) => Some(dt.timestamp()), _ => None } 取真实 epoch
       - 后续文件丢弃 file_id / file_creator / device_info / developer_data_id / field_description

EXAMPLES:
  fit-editor merge morning.fit afternoon.fit -o full_day.fit
```

#### split

```
fit-editor split <FILE> [OPTIONS] -o <PREFIX>

按时间戳或消息索引拆分 FIT 文件。

OPTIONS:
  -o, --output <PREFIX>    输出文件名前缀 (required) ✅ 实现
      --at <TIMESTAMP>     拆分时间点 (RFC3339) ✅ 实现
      --at-index <N>       在第 N 条消息处拆分 ✅ 实现

OUTPUT:
  <PREFIX>_part1.fit
  <PREFIX>_part2.fit

KNOWN ISSUES:
  ⚠️ (bug, M6, split.rs:48-58) output prefix 无 path-traversal 校验，
     `--output ../../tmp/evil` 可写到任意位置。Phase 4.6 必须 canonicalize 父目录、
     拒绝包含 `..` 段的 prefix。

EXAMPLES:
  fit-editor split Activity.fit --at 2024-01-15T11:00:00Z -o split
```

#### diff

```
fit-editor diff <FILE1> <FILE2>

对比两个 FIT 文件的差异。

OPTIONS:
      --ignore-timestamps   忽略时间戳差异
  -m, --message <TYPE>      仅对比指定类型

OUTPUT FORMAT:
  --- file1.fit
  +++ file2.fit
  @@ session @@
  - total_distance = 5000.0
  + total_distance = 5200.0
  @@ record[10] @@
  - heart_rate = 145
  + heart_rate = 150
```

#### summary

```
fit-editor summary <FILE>

显示运动活动的摘要信息。

OUTPUT:
  Activity Summary
  ─────────────────────────────────────────
  Sport:          Running
  Start Time:     2024-01-15 10:30:00 UTC
  Duration:       30:00 (timer) / 32:15 (elapsed)
  Distance:       5.00 km
  Avg Speed:      2.78 m/s (10.0 km/h)
  Max Speed:      3.89 m/s (14.0 km/h)
  Avg Heart Rate: 155 bpm (max: 185)
  Calories:       350 kcal
  Ascent:         50 m
  Descent:        45 m
```

#### hexdump

```
fit-editor hexdump <FILE>

OPTIONS:
  -n, --bytes <N>          最多显示 N 字节 ✅
      --skip-header        跳过文件头 ✅
      --annotate           标注消息边界和字段位置 ✅

OUTPUT:
  00000000: 0e 20 d0 52 78 4a 12 00  2e 46 49 54 de 67 40 00  | . .R.J...FIT.g@.
  00000010: 00 00 00 00 04 00 01 04  86 04 01 02 84 05 02 01  | ................
  -- annotate 模式额外输出:
  [0x00] File Header (14 bytes)
  [0x0E] Definition Message (local=0, global=file_id)
  [0x1E] Data Message (local=0)

KNOWN ISSUES:
  ⚠️ (bug, H1, hexdump.rs:122-128) estimate_definition_size 边界检查少 1 字节
     (检查 < 5 但访问 data[5])。喂一个长度恰为 5 的截断输入即 panic。
     诊断损坏文件的命令被损坏文件干掉。
     Phase 4.6 改为 `if data.len() < 6 { return data.len(); }`，
     并把越界检查抽出 helper。
```

### Shell 补全

```
fit-editor completion bash > /etc/bash_completion.d/fit-editor
fit-editor completion zsh > ~/.zsh/completions/_fit-editor
fit-editor completion fish > ~/.config/fish/completions/fit-editor.fish
fit-editor completion powershell > $PROFILE
```
