# Technical Architecture

## 技术架构设计

### 1. 整体架构

```
┌─────────────────────────────────────────────┐
│                  CLI Layer                   │
│  clap (子命令路由、参数解析、输出格式化)      │
├─────────────────────────────────────────────┤
│                Command Layer                 │
│  validate / info / dump / export /           │
│  encode / edit / merge / split / ...         │
├─────────────────────────────────────────────┤
│               Service Layer                  │
│  FitFileService: 编解码编排、消息过滤、       │
│  字段修改、格式转换协调                        │
├─────────────────────────────────────────────┤
│          fit-sdk-rust (Core SDK)             │
│  Decoder / TypedDecoder / Encoder            │
│  Profile / Transforms / CRC                  │
└─────────────────────────────────────────────┘
```

### 2. 目录结构

```
fit-editor-rust/
├── Cargo.toml
├── src/
│   ├── main.rs              # 入口、CLI 解析
│   ├── cli.rs               # clap 定义（Args, Subcommands）
│   ├── error.rs             # 统一错误类型
│   ├── commands/            # 各子命令实现
│   │   ├── mod.rs
│   │   ├── validate.rs
│   │   ├── info.rs
│   │   ├── dump.rs
│   │   ├── export.rs
│   │   ├── encode.rs
│   │   ├── edit.rs
│   │   ├── merge.rs
│   │   ├── split.rs
│   │   ├── diff.rs
│   │   ├── summary.rs
│   │   ├── hexdump.rs
│   │   └── batch.rs
│   ├── output/              # 输出格式化
│   │   ├── mod.rs
│   │   ├── json.rs
│   │   ├── csv.rs
│   │   ├── table.rs         # 终端表格
│   │   └── gpx.rs           # GPX 格式
│   ├── service/             # 业务逻辑层
│   │   ├── mod.rs
│   │   ├── fit_file.rs      # FIT 文件读写编排
│   │   ├── field_edit.rs    # 字段修改逻辑
│   │   └── merge.rs         # 文件合并逻辑
│   └── util/
│       ├── mod.rs
│       └── human.rs         # 人类可读格式化（字节大小、持续时间）
├── tests/                   # 集成测试
│   ├── fixtures/            # 测试用 FIT 文件
│   └── roundtrip.rs         # decode-encode-decode 测试
├── docs/                    # 项目文档
└── build.rs                 # 构建脚本（shell 补全、man page）
```

### 3. 依赖选择

| Crate | 用途 | 理由 |
|-------|------|------|
| `fit-sdk-rust` (path dep) | FIT 编解码 | 项目核心 SDK |
| `clap` (derive) | CLI 解析 | 最流行的 Rust CLI 框架，derive 模式简洁 |
| `serde` + `serde_json` | JSON 序列化 | 行业标准 |
| `csv` | CSV 读写 | `serde` 生态，简单可靠 |
| `colored` / `owo-colors` | 彩色输出 | 提升终端体验 |
| `anyhow` | 错误传播 | MVP 阶段简化错误处理 |
| `tabled` | 表格输出 | info/summary 命令需要 |
| `indicatif` | 进度条 | batch 命令需要 |
| `chrono` | 时间格式化 | SDK 已依赖，复用 |
| `glob` | 文件匹配 | batch 命令需要 |
| `tempfile` | 测试临时文件 | 集成测试 |

### 4. SDK 集成方式

#### 4.1 解码流程

```
File (bytes)
  → fit::Decoder::builder(&bytes).build()    // RawMessage 迭代器
  → .read_all() → (Vec<Message>, Vec<FitError>)
  → 所有 Transform 默认开启 (scale/offset, datetime, enum, components, subfields)
```

关键 API 调用:
```rust
use fit::{Decoder, Message, Value};

let bytes = std::fs::read(&path)?;
let (messages, errors) = Decoder::builder(&bytes).build().read_all();
// messages: Vec<Message> — 每个包含 global_mesg_num, name, fields
// errors: Vec<FitError> — 解码过程中的非致命错误
```

#### 4.2 编码流程

```
Vec<Message>
  → fit::Encoder::new().encode(&messages)
  → Vec<u8> (FIT binary)
  → fs::write() + CRC 校验
```

关键 API 调用:
```rust
use fit::{Encoder, Message};

let encoded: Vec<u8> = Encoder::new().encode(&messages)?;
fit::check_integrity(&encoded)?; // 可选：编码后立即校验
```

#### 4.3 完整性校验

```rust
// 快速检查：是否是 FIT 文件
fit::is_fit(&bytes) → bool

// 完整 CRC 校验
fit::check_integrity(&bytes) → Result<(), FitError>
```

### 5. 核心数据流

#### 5.1 dump 命令数据流

```
输入文件
  → fs::read() → bytes
  → Decoder::builder(&bytes).build().read_all()
  → messages (Vec<Message>)
  → 遍历每条 Message:
      → 输出 name (消息类型)
      → 遍历每个 Field:
          → name: 字段名
          → value: 根据 Value 枚举变体格式化
            - DateTime → RFC3339
            - Enum → "enum_name (raw_value)"
            - Float → "{:.N}"
            - UInt/SInt → 数字
            - String → 字符串
            - Invalid → "<invalid>"
          → units: 单位后缀
```

#### 5.2 export --format json 数据流

```
输入文件
  → Decoder 解码
  → serde_json::to_string_pretty(&SerializableMessages)
  → 输出到 stdout 或文件
```

JSON 结构设计:
```json
{
  "file_header": {
    "protocol_version": "2.0",
    "profile_version": 21200,
    "data_size": 123456
  },
  "messages": [
    {
      "type": "file_id",
      "fields": {
        "manufacturer": "garmin",
        "product": 3121,
        "time_created": "2024-01-15T10:30:00Z"
      }
    }
  ]
}
```

#### 5.3 edit --set 数据流

```
输入文件
  → Decoder 解码 → Vec<Message>
  → 定位目标消息 (by type / by index)
  → 修改字段值
  → Encoder 编码 → Vec<u8>
  → 写入输出文件
```

### 6. 错误处理策略

| 场景 | 处理方式 |
|------|----------|
| 文件不存在 / 无权限 | `anyhow::Error` → 用户友好消息 |
| 非 FIT 文件 | `is_fit()` 返回 false → 明确提示 |
| CRC 校验失败 | `FitError::HeaderCrcMismatch` / `FileCrcMismatch` → 显示存储值 vs 计算值 |
| 解码非致命错误 | `Vec<FitError>` → stderr 警告，继续处理 |
| 编码字段溢出 | `FitError::FieldTooLarge` → 显示字段名和限制 |
| JSON 解析失败 | `serde_json::Error` → 显示行号和错误位置 |

### 7. 性能优化策略

| 策略 | 适用场景 |
|------|----------|
| `--release` 编译 | 所有场景 |
| 预分配 Vec 容量 | Encoder/Decoder 内部已实现 |
| 避免不必要的 clone | dump/export 使用引用 |
| 流式 CSV 写入 | 大文件 CSV 导出 |
| `BufWriter` | 文件输出 |
| `rayon` 并行 | batch 命令批量处理 |

### 8. 测试策略

| 测试类型 | 覆盖范围 |
|---------|---------|
| 单元测试 | 每个 command 模块的逻辑 |
| 集成测试 | decode-encode-decode 往返、CLI 端到端 |
| Fixture 测试 | SDK 附带的 `Activity.fit`、`WithGearChangeData.fit` 等 |
| Fuzz 测试 | 对编码输出进行随机修改后的解码鲁棒性 |
| 性能测试 | 典型文件的处理时间基准 |
