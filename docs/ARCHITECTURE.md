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

> **注：** 原始设计稿曾规划独立的 `output/` / `service/` / `util/` 层，实际实现把序列化、字段编辑、合并逻辑直接放在对应的 `commands/*.rs` 里。下面是 HEAD = 5ef7e1c 的真实结构。

```
fit-editor-rust/
├── Cargo.toml
├── build.rs                 # clap_mangen 生成 man page
├── README.md
├── Report.html              # 2026-05-13 strict review 输出
├── src/
│   ├── main.rs              # 入口；CLI 解析 + 12 路 subcommand 路由（105 LOC）
│   ├── cli.rs               # clap 定义：Cli / Command / ExportFormat（208 LOC）
│   ├── error.rs             # CliError enum + From 转换（91 LOC）
│   └── commands/            # 每个子命令一个文件，自带 I/O + 序列化逻辑
│       ├── mod.rs
│       ├── validate.rs      # ⚠️ H2: 仅 check_integrity，未调 Decoder
│       ├── info.rs
│       ├── dump.rs
│       ├── export.rs        # ⚠️ C2: GPX 对无 GPS record 写 (0,0)
│       ├── encode.rs        # ⚠️ C1: Box::leak; M3: 数值静默截断
│       ├── edit.rs          # ⚠️ C1: Box::leak
│       ├── merge.rs         # ⚠️ C3: 不去重 file_id；timestamp 排序失效
│       ├── split.rs         # ⚠️ M6: 无 ../ 校验
│       ├── diff.rs          # ⚠️ M2: 主循环含 unreachable!()
│       ├── summary.rs
│       ├── hexdump.rs       # ⚠️ H1: estimate_definition_size 越界
│       └── batch.rs         # ⚠️ M1/M4: .unwrap() + process::exit
├── tests/
│   ├── phase2.rs            # 7 个集成测试（encode/edit/hexdump）
│   └── phase4.rs            # 6 个集成测试（completion/info/summary/batch）
└── docs/                    # 项目文档
```

**⚠️ 未来重构（Phase 4.6 A1）：** 引入 `src/lib.rs`，把 `commands/*` 改为 lib-callable，
打开单元测试与 doctests 的可能性。当前仓库是 binary-only crate，纯函数无法独立测试。

### 3. 依赖选择（实际 `Cargo.toml`）

| Crate | 类别 | 用途 |
|-------|------|------|
| `fit-sdk-rust` (path dep) | runtime | FIT 编解码核心 |
| `clap` 4 (derive) + `clap_complete` 4 | runtime | CLI 解析 + shell 补全 |
| `serde` 1 + `serde_json` 1 | runtime | JSON 序列化 |
| `csv` 1 | runtime | CSV 导出 |
| `colored` 2 | runtime | 彩色输出（TTY 自动禁用） |
| `tabled` 0.15 | runtime | info/summary 表格输出 |
| `indicatif` 0.17 | runtime | batch 进度条 |
| `chrono` 0.4 | runtime | RFC3339 时间解析 |
| `glob` 0.3 | runtime | batch 命令文件匹配 |
| `anyhow` 1 | runtime | ⚠️ **死依赖**：`src/` 下零引用，错误处理实际用 `CliError`。Phase 4.6 L4 删除 |
| `clap` + `clap_complete` + `clap_mangen` 0.2 | build | `build.rs` 生成 man page |
| `assert_cmd` 2 + `predicates` 3 + `tempfile` 3 | dev | 集成测试 |

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

实际错误类型在 [`src/error.rs`](../src/error.rs) 的 `CliError` enum 中定义（**不**使用 `anyhow`）。

| 场景 | 处理方式 | 当前 `CliError` 变体 |
|------|----------|----------------------|
| 文件不存在 / 无权限 | `From<io::Error>` → `CliError::Io` | `Io(io::Error)` |
| 非 FIT 文件 | `is_fit()` 返回 false → 显式构造 | `NotFit(String)` |
| 文件截断 | SDK `FitError::TooShort` | `Truncated { expected, actual }` |
| CRC 校验失败 | SDK 错误 → 显示存储值 vs 计算值 | `CrcMismatch { stored, calculated, which }` |
| 字段路径不合法 | `edit` 命令显式构造 | `InvalidFieldPath(String)` |
| JSON 解析失败 | `From<serde_json::Error>` | `Json(serde_json::Error)` |
| CSV 写入失败 | `From<csv::Error>` → 包成 `Io` | `Io(...)` |
| 解码非致命错误 | `Vec<FitError>` → 当前**静默丢弃**，仅 `--verbose` 下打印 | （未类型化） |

#### Phase 4.6 错误模型补强

- **M5**：新增 `CliError::BadUsage(String)`。当前 `batch` / `merge` 把业务校验（"需要至少 2 个文件"）硬转成 `io::Error::new(InvalidInput, ...)`，类型说谎。
- **M4**：新增 `CliError::BatchPartialFailure { failed: usize }`，让 `batch` 走标准 error 流，移除 `process::exit(1)`。
- **H2 / 架构层**：建议 `Decoder::builder` 支持 `.strict()`；让 decoder error 成为命令本身的错误，不再每个 callsite 单独决定丢不丢。
- 死变体 `CliError::UnknownMessage`（`error.rs:16` 标 `#[allow(dead_code)]`）：移除。

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

**目标状态**

| 测试类型 | 覆盖范围 |
|---------|---------|
| 单元测试 | 每个 command 模块的纯函数 |
| 集成测试 | decode-encode-decode 往返、CLI 端到端、所有子命令的快乐与错误路径 |
| Fixture 测试 | SDK 附带的 `Activity.fit`、损坏文件、空文件、超长文件 |
| Fuzz 测试 | `Decoder::read_all` / `FileHeader::parse` 喂随机字节 |
| 性能测试 | 典型文件的处理时间基准 |

**当前状态（HEAD = 5ef7e1c）**

| 类别 | 计数 |
|------|------|
| 集成测试 | 13 |
| 单元测试 | 0（无 `lib.rs`） |
| doctests | 0 |
| fuzz target | 0 |
| benches | 0 |
| CI workflow | 0 |

详见 [TESTING.md](./TESTING.md) 与 [Report.html](../Report.html) 第 6 节。Phase 4.6 必须把这张表打到至少
"集成测试 ≥ 30 / 单元测试 ≥ 20 / fuzz target ≥ 2 / CI workflow = 1"。

### 9. Known issues & tech debt（2026-05-13 strict review 输出）

详见 [`../Report.html`](../Report.html)。架构相关的高优先级条目：

1. **SDK API 把 lifetime 推给消费者**（C1）。`fit::Value::Enum(&'static str)` 强迫所有 CLI 端 enum 字段编辑用 `Box::leak`。修复需要协调上游 `fit-sdk-rust`，把 `Value::Enum` 改为 `Arc<str>` / `Cow<'static, str>` / `String`。
2. **Decoder 错误是 silent 的**。每个 callsite 都 `let (msgs, _errors) = ...read_all();`，错误丢弃。需要 SDK 提供 `Decoder::builder.strict()`，并把错误层级提升到命令决策。
3. **没有 `lib.rs`**。所有 `commands::*` 不能被外部 crate 复用、不能写单元测试、不能写 doctests。半天工作量，价值高。
4. **错误模型不连贯**。`CliError` 缺 `BadUsage` / `BatchPartialFailure` / `DecodeFailed` 等业务变体，业务错误目前借 `io::Error` 承载。
5. **无 CI / fuzz / lint 防线**。0.1.0 已经 push 到 GitHub 但无任何自动化保护。`#![forbid(unsafe_code)]` 这一行的成本为零。
