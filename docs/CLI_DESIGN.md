# CLI Command Reference Design

## 命令行接口设计

### 全局选项

```
fit-editor [OPTIONS] <COMMAND>

OPTIONS:
  -v, --verbose        详细输出（显示解码警告等）
  -q, --quiet          静默模式（仅输出结果，无装饰）
      --no-color       禁用彩色输出
  -h, --help           打印帮助
  -V, --version        打印版本
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

EXIT CODES:
  0  文件有效
  1  文件无效
  2  IO 错误

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
  -f, --format <FORMAT>    输出格式: json, csv, gpx [default: json]
  -o, --output <FILE>      输出文件路径 (默认 stdout)
  -m, --message <TYPE>     仅导出指定类型
      --pretty             格式化 JSON (默认开启)
      --compact             紧凑 JSON

JSON STRUCTURE:
  {
    "messages": [
      { "type": "record", "index": 0, "fields": { ... } },
      ...
    ]
  }

CSV FORMAT (record messages):
  timestamp,position_lat,position_long,heart_rate,speed,...
  2024-01-15T10:30:01Z,39.9042,116.4074,120,2.5,...

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
  -o, --output <FILE>      输出 FIT 文件路径 (required)
      --protocol <VER>     协议版本 [default: 0x20]
      --profile <VER>      Profile 版本 [default: 21200]
      --validate           编码后自动校验 CRC

EXAMPLES:
  fit-editor encode activity.json -o modified.fit
  fit-editor encode activity.json -o modified.fit --validate
```

#### edit

```
fit-editor edit <FILE> [OPTIONS] -o <OUTPUT>

OPTIONS:
  -o, --output <FILE>                     输出文件路径 (required)
      --set <FIELD=VALUE>                 设置字段值 (可多次使用)
      --remove-message <TYPE>             删除指定类型消息
      --append <JSON>                     追加消息 (从 JSON 文件)
      --update-header <field=value>       修改文件头元数据

FIELD PATH SYNTAX:
  session.total_distance=5000.0           修改 session 消息的 total_distance
  session[0].sport=cycling                修改第一个 session 的 sport
  file_id.product=1234                    修改 file_id 的 product 字段

EXAMPLES:
  fit-editor edit Activity.fit --set session.total_distance=5000.0 -o modified.fit
  fit-editor edit Activity.fit --remove-message device_info -o cleaned.fit
  fit-editor edit Activity.fit --append extra_records.json -o extended.fit
```

#### merge

```
fit-editor merge <FILE1> <FILE2> [FILE...] -o <OUTPUT>

按时间戳合并多个 FIT 文件。

OPTIONS:
  -o, --output <FILE>      输出文件路径 (required)
      --sort <ORDER>       时间排序: asc, desc [default: asc]
      --no-dedup           不去除重复消息

EXAMPLES:
  fit-editor merge morning.fit afternoon.fit -o full_day.fit
```

#### split

```
fit-editor split <FILE> [OPTIONS] -o <PREFIX>

按时间戳或消息索引拆分 FIT 文件。

OPTIONS:
  -o, --output <PREFIX>    输出文件名前缀 (required)
      --at <TIMESTAMP>     拆分时间点 (RFC3339)
      --at-index <N>       在第 N 条消息处拆分

OUTPUT:
  <PREFIX>_part1.fit
  <PREFIX>_part2.fit

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
  -n, --bytes <N>          最多显示 N 字节
      --skip-header        跳过文件头
      --annotate           标注消息边界和字段位置

OUTPUT:
  00000000: 0e 20 d0 52 78 4a 12 00  2e 46 49 54 de 67 40 00  | . .R.J...FIT.g@.
  00000010: 00 00 00 00 04 00 01 04  86 04 01 02 84 05 02 01  | ................
  -- annotate 模式额外输出:
  [0x00] File Header (14 bytes)
  [0x0E] Definition Message (local=0, global=file_id)
  [0x1E] Data Message (local=0)
```

### Shell 补全

```
fit-editor --generate-completion bash    # → /etc/bash_completion.d/fit-editor
fit-editor --generate-completion zsh     # → ~/.zsh/completions/_fit-editor
fit-editor --generate-completion fish    # → ~/.config/fish/completions/fit-editor.fish
```
