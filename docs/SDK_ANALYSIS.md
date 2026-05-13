# SDK Integration Analysis

## fit-sdk-rust SDK 深度分析

### 1. SDK 概览

| 属性 | 值 |
|------|-----|
| 仓库 | `https://github.com/Chen-Lim/fit-sdk-rust` |
| 包名 | `fit-sdk-rust` (lib name: `fit`) |
| 版本 | 0.1.0 |
| Rust MSRV | 1.75 |
| 许可证 | Apache-2.0 |
| FIT 协议 | v2.0 (0x20), Profile v21.200 |
| 外部依赖 | `thiserror` 2.0, `chrono` 0.4 |

### 2. 核心 API 矩阵

| API | 类型 | 功能 | fit-editor 用途 |
|-----|------|------|----------------|
| `fit::is_fit(&[u8])` | 函数 | 快速判断是否为 FIT 文件 | validate 命令 |
| `fit::check_integrity(&[u8])` | 函数 | 完整 CRC 校验 | validate 命令 |
| `fit::crc16(&[u8])` | 函数 | 计算 CRC-16 | 内部/调试 |
| `fit::FileHeader::parse(&[u8])` | 方法 | 解析文件头 | info 命令 |
| `fit::Decoder::new(&[u8])` | 构造器 | 原始消息流解码器 | 原始模式 dump |
| `fit::Decoder::builder(&[u8])` | Builder | Profile-aware 类型化解码器 | 所有查看/导出命令 |
| `fit::Encoder::new()` | 构造器 | FIT 编码器 | encode/edit 命令 |
| `fit::Encoder::builder()` | Builder | 可配置编码器 | encode（自定义版本） |
| `fit::Encoder::encode(&[Message])` | 方法 | 编码为 FIT 二进制 | encode 命令 |
| `fit::Encoder::encode_chain(&[&[Message]])` | 方法 | 编码多段链式 FIT | merge 命令 |
| `fit::check_integrity(&[u8])` | 函数 | 编码后校验 | encode --validate |

### 3. 数据类型详解

#### 3.1 Value 枚举 (类型化值)

```rust
pub enum Value {
    Invalid,                      // 无效值
    SInt(i64),                    // 有符号整数
    UInt(u64),                    // 无符号整数
    Float(f64),                    // 浮点（含 scale/offset 转换后的物理值）
    String(String),               // UTF-8 字符串
    Bytes(Vec<u8>),               // 原始字节
    Bool(bool),                    // 布尔
    Enum(&'static str),           // 解析后的枚举名称 (e.g. "running")
                                  // ⚠️ API 设计缺陷，见下文
    DateTime(DateTime<Utc>),      // 转换后的 UTC 时间
    Array(Vec<Value>),            // 数组类型字段
}
```

> **⚠️ `Value::Enum(&'static str)` 是已知 API 缺陷（fit-editor C1）。**
>
> 当消费者想要从动态数据（用户 CLI 输入、JSON 字段）构造一个 `Value::Enum` 时，
> 必须把动态 `String` 转成 `&'static str`，唯一可行的办法就是 `Box::leak`，
> 这是确定的内存泄漏。fit-editor 在 [`edit.rs:167`](../src/commands/edit.rs) 和
> [`encode.rs:123`](../src/commands/encode.rs) 已经踩了这个坑。
>
> **Required SDK change (Phase 4.6 C1):** 把 `Value::Enum` 改为：
> - `Value::Enum(Arc<str>)` —— 推荐；profile 表静态字符串用 `Arc::from(&'static str)` 是 O(1) 封装；
> - 或 `Value::Enum(Cow<'static, str>)` —— borrow 由 SDK 静态供给，own 由消费者动态构造；
> - 或 `Value::Enum(String)` —— 简单但 SDK 内部失去 'static 优化。
>
> fit-sdk-rust 与 fit-editor 是同一作者维护，此处协调改动可行。

#### 3.2 Field 结构

```rust
pub struct Field {
    pub name: String,            // snake-case 字段名
    pub kind: FieldKind,         // Standard { field_def_num } | Developer { ... }
    pub value: Value,            // 解码后的值
    pub units: Option<String>,   // 单位 (e.g. "m/s", "bpm")
}
```

#### 3.3 Message 结构

```rust
pub struct Message {
    pub global_mesg_num: u16,    // Profile 消息号
    pub name: &'static str,      // snake-case 消息名 (e.g. "record", "session")
    pub fields: Vec<Field>,      // 所有字段
}
```

### 4. Transform 管线

TypedDecoder 的处理顺序（由 `TransformOptions` 控制）:

```
RawMessage
  ├─ 1. SubField 选择       (expand_subfields)
  │     根据同消息中其他字段值选择语义子类型
  │
  ├─ 2. Accumulator         (自动)
  │     处理跨消息的计数器翻转补偿
  │
  ├─ 3. DateTime 转换       (convert_datetime)
  │     u32 秒数 → chrono::DateTime<Utc>
  │
  ├─ 4. Enum 字符串化       (convert_types_to_strings)
  │     枚举整数值 → &'static str 名称
  │
  ├─ 5. Scale/Offset        (apply_scale_and_offset)
  │     原始整数 → 物理浮点值
  │
  ├─ 6. Components 展开     (expand_components)
  │     LSB-first 位域解包为独立字段
  │
  └─ 7. Developer Fields    (自动)
        通过 DevFieldRegistry 解析开发字段
```

**fit-editor 默认:** 全部开启 (dump/export)，`--raw` 模式全部关闭。

### 5. 支持的消息类型 (126 种)

关键运动类消息:

| MesgNum | 名称 | 场景 |
|---------|------|------|
| 0 | file_id | 文件标识（类型、设备、时间） |
| 18 | session | 运动会话摘要 |
| 19 | lap | 圈数据 |
| 20 | record | GPS 轨迹点（心率、速度、海拔等） |
| 21 | event | 事件（计圈、GPS 信号等） |
| 23 | device_info | 设备信息 |
| 26 | workout | 训练计划 |
| 27 | workout_step | 训练步骤 |
| 31 | course | 路线 |
| 32 | course_point | 路线点 |
| 34 | activity | 活动总览 |
| 49 | file_creator | 创建工具信息 |
| 101 | length | 泳池趟数据 |
| 145 | memo_glob | 备注文本 |
| 233 | split / split_summary | 分段数据 |

完整列表见 `mesg_num.rs` (共 126 种消息类型)。

### 6. 编码器限制与注意事项

| 限制 | 影响 | 应对 |
|------|------|------|
| 不生成压缩时间戳 | 输出文件略大 | 文档说明语义等价 |
| 16 个 local definition 槽位 | 单 segment 最多 16 种消息类型 | LRU 自动处理，多 segment 链式编码 |
| Developer 字段需要 DevFieldRegistry | 编码前必须有 field_description 消息 | 编码器自动从输入消息中收集 |
| Components-synthesised 字段跳过写入 | 避免重复编码 | 编码器内部过滤 |
| `Value::Enum(&'static str)` | 动态构造 enum 值需要 `Box::leak`，造成内存泄漏 | ⚠️ **SDK 必须改 API，见 §3.1** |
| Decoder 错误是 `Vec<FitError>`，不是 `Result` | 消费者必须自己决定如何处理 | 建议 SDK 提供 `Decoder::builder.strict()` 选项 |

### 7. 与 SDK 的集成模式

#### 7.1 作为 path dependency

```toml
# Cargo.toml
[dependencies]
fit-sdk-rust = { path = "../fit-sdk-rust" }
# 或 git 依赖:
# fit-sdk-rust = { git = "https://github.com/Chen-Lim/fit-sdk-rust" }
```

#### 7.2 Round-trip 一致性保证

```rust
fn roundtrip_test(original: &[u8]) {
    let (messages, _) = fit::Decoder::builder(original).build().read_all();
    let reencoded = fit::Encoder::new().encode(&messages).unwrap();
    fit::check_integrity(&reencoded).unwrap(); // 结构合法

    let (messages2, _) = fit::Decoder::builder(&reencoded).build().read_all();
    // messages == messages2 (语义等价)
}
```

#### 7.3 自定义 TransformOptions

```rust
// --raw 模式: 关闭所有转换
let (msgs, errs) = Decoder::builder(&bytes)
    .apply_scale_and_offset(false)
    .convert_datetime(false)
    .convert_types_to_strings(false)
    .expand_components(false)
    .expand_subfields(false)
    .build()
    .read_all();
```

### 8. 未覆盖的需求（需要 fit-editor 自行实现）

| 需求 | SDK 提供 | fit-editor 需要 |
|------|---------|----------------|
| JSON/CSV/GPX 序列化 | 无 | 自行实现 |
| 字段路径解析 (`session.total_distance`) | 无 | 自行实现 |
| 文件合并排序 | 无 | 按时间戳排序逻辑 |
| 文件拆分 | 无 | 消息分组 + 分段编码 |
| 差异对比 | 无 | 消息/字段 diff 算法 |
| 运动统计计算 | 无 | 聚合计算 |
| Hex 带注释输出 | 无 | 消息边界追踪 |

### 9. Required SDK changes (Phase 4.6)

来自 2026-05-13 strict review（[`../Report.html`](../Report.html)）的 SDK 层面需求：

| ID | 改动 | 理由 | 影响范围 |
|----|------|------|----------|
| SDK-1 | `Value::Enum(&'static str)` → `Value::Enum(Arc<str>)`（或 `Cow<'static, str>` / `String`） | 当前 API 强迫消费者 `Box::leak` 动态 enum 字符串 | fit-editor C1 完全消失；其他下游消费者同样受益 |
| SDK-2 | `Decoder::builder.strict()` 模式：当 `read_all` 期间出现任何 `FitError` 时直接返回 `Err`，而不是 `(Vec<Message>, Vec<FitError>)` | 当前所有 fit-editor 命令一律 `let (msgs, _errors) = ...`，把错误吞掉，导致 `validate` 对消息层错误失明（H2） | fit-editor H2 修复的前提 |
| SDK-3 | 提供小型损坏 fixture（截断 header、错 CRC、definition/data 不一致）供下游做错误路径测试 | 当前下游无法覆盖错误路径 | fit-editor 集成测试 A4 |

SDK 是同一作者维护的 path dep，这些改动可以与 fit-editor Phase 4.6 同时落地。
