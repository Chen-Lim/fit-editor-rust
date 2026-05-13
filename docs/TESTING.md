# Testing Strategy

## 测试策略文档

> **2026-05-13 状态对照（必读）：** 本文档原稿描述的金字塔、fixture、CI 配置基本是
> aspirational。实际状态见下表。所有 ⚠️ 项是 [Phase 4.6](./Rroadmap.md) 的硬交付。

### 0. Current state vs target

| 项目 | 设计目标 | 实际 HEAD = 5ef7e1c | 状态 |
|------|----------|----------------------|------|
| 集成测试 | 15–20 | **13**（`tests/phase2.rs` + `tests/phase4.rs`） | ⚠️ 接近下限，但覆盖面浅 |
| 单元测试 | 50+ | **0**（无 `lib.rs`，纯函数不可被外部测试） | ⚠️ Phase 4.6 A1 |
| doctests | 若干 | **0** | ⚠️ 依赖 lib.rs |
| Fuzz target | ≥ 2 | **0**（无 `cargo fuzz`） | ⚠️ Phase 4.6 A2，二进制格式解析器无 fuzz 不可接受 |
| Benchmarks | 1+ | **0**（无 `benches/`） | ⚠️ 推迟到 v0.2.0 |
| CI workflow | 完整 | **0**（无 `.github/workflows/`） | ⚠️ Phase 4.6 A3 |
| Fixture 文件 | 损坏 / 截断 / 空 / 多种来源 | **仅 1 份**（`../fit-sdk-rust/tests/fixtures/test_data/Activity.fit`，跨仓库相对路径） | ⚠️ Phase 4.6 A4 |

### 已知**零覆盖**的代码路径（来自 [Report.html](../Report.html) 第 6 节）

- CRC mismatch 路径
- truncated header（`FileHeader::parse` 错误分支）
- GPX 导出（"gpx" 在 `tests/` 下出现 0 次 → 直接导致 C2 长期未被发现）
- CSV 导出（"csv" 在 `tests/` 下出现 0 次）
- `validate` 子命令
- `merge` 子命令（→ C3 长期未被发现）
- `split` 子命令
- `diff` 子命令（→ M2 的 `unreachable!()` 与边界）
- `summary` 子命令的非快乐路径
- 任何错误输入：不存在的文件、目录而非文件、无 GPS record、空 messages 数组、不合法 timestamp、不合法字段路径

### 1. 测试金字塔

```
        ╱  E2E  ╲         ← CLI 端到端 (5-10 个)
       ╱─────────╲
      ╱ 集成测试   ╲       ← decode-encode roundtrip (15-20 个)
     ╱─────────────╲
    ╱   单元测试     ╲     ← 命令/输出/格式化 (50+ 个)
   ╱─────────────────╲
```

### 2. Fixture 文件

使用 SDK 仓库自带的测试文件，以及手动构造的边界用例:

| 文件 | 用途 | 来源 |
|------|------|------|
| `Activity.fit` | 典型跑步活动 | SDK `tests/fixtures/` |
| `WithGearChangeData.fit` | 含变档事件 | SDK `tests/fixtures/` |
| `HrmPluginTestActivity.fit` | HR 合并测试 | SDK `tests/fixtures/` |
| `empty.fit` | 空文件 | 手动生成 |
| `corrupt.fit` | 损坏文件 (CRC 错误) | 手动修改 |
| `minimal.fit` | 仅 file_id 消息 | 手动生成 |

### 3. 关键测试场景

#### 3.1 Round-trip 测试

```rust
#[test]
fn roundtrip_preserves_field_values() {
    let original = std::fs::read("tests/fixtures/Activity.fit").unwrap();
    let (messages, _) = Decoder::builder(&original).build().read_all();
    let encoded = Encoder::new().encode(&messages).unwrap();
    check_integrity(&encoded).unwrap();

    let (roundtripped, _) = Decoder::builder(&encoded).build().read_all();
    assert_eq!(messages.len(), roundtripped.len());
    for (a, b) in messages.iter().zip(roundtripped.iter()) {
        assert_eq!(a.global_mesg_num, b.global_mesg_num);
        // 逐字段比较...
    }
}
```

#### 3.2 validate 命令测试

- 有效文件 → exit 0
- CRC 错误文件 → exit 1 + 错误信息
- 非 FIT 文件 → exit 1
- 截断文件 → exit 1
- 空文件 → exit 1

#### 3.3 export 命令测试

- JSON 输出可被 serde_json 解析
- CSV 输出列数一致、数据类型正确
- GPX 输出可被 XML 解析器读取
- 仅导出指定消息类型

#### 3.4 encode 命令测试

- 导出的 JSON 可重新编码为合法 FIT
- 编码文件通过 validate 校验
- 空消息数组编码为有效空 FIT

#### 3.5 编辑命令测试

- 修改字段值后重新解码验证
- 删除消息类型后验证数量
- 不存在的字段路径返回错误

### 4. CI 配置

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]
jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        rust: [stable, 1.75]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ matrix.rust }}
      - run: cargo build --release
      - run: cargo test --release
      - run: cargo clippy -- -D warnings
      - run: cargo fmt --check
```

### 5. 性能基准

```rust
// benches/decode.rs
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_decode_activity(c: &mut Criterion) {
    let bytes = std::fs::read("tests/fixtures/Activity.fit").unwrap();
    c.bench_function("decode_activity", |b| {
        b.iter(|| {
            fit::Decoder::builder(&bytes).build().read_all()
        })
    });
}

criterion_group!(benches, bench_decode_activity);
criterion_main!(benches);
```

### 6. Fuzz testing (Phase 4.6 A2)

```toml
# fuzz/Cargo.toml
[dependencies]
fit-sdk-rust = { path = "../../fit-sdk-rust" }
libfuzzer-sys = "0.4"
```

```rust
// fuzz/fuzz_targets/decode_read_all.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = fit::Decoder::builder(data).build().read_all();
});

// fuzz/fuzz_targets/file_header_parse.rs
fuzz_target!(|data: &[u8]| {
    let _ = fit::FileHeader::parse(data);
});
```

**门槛：** 提交 Phase 4.6 之前，每个 fuzz target 本地至少跑 1 小时，无 crash / OOM。

### 7. 强制集成测试（Phase 4.6 A4 必交付）

最低限度补齐以下场景，按命令分类：

| 命令 | 必须新增测试 | 关联 finding |
|------|--------------|--------------|
| `validate` | (1) CRC 错误文件 exit 1；(2) decoder error 文件 exit 2 + warning；(3) 截断 header exit 1；(4) 空文件 exit 1 | H2 |
| `merge` | (1) 两个文件 timestamp 交错合并后顺序正确；(2) 输出只有一份 `file_id` | C3 |
| `split` | (1) `--at` 时间点；(2) `--at-index`；(3) `--output ../evil` 被拒绝 | M6 |
| `diff` | (1) 文件长度不等；(2) 字段类型变化；(3) `--ignore-timestamps` | M2 |
| `export gpx` | (1) 室内 record（无 GPS）不产生 `<trkpt>` | C2 |
| `export csv` | (1) 列头与数据列数一致；(2) 嵌入逗号/换行符的字符串字段正确转义 | 当前零覆盖 |
| `edit` | (1) enum 字段 set 后 round-trip 一致；(2) 不存在的字段路径返回 `InvalidFieldPath` | C1 间接验证 |
| `encode` | (1) JSON 超大整数（> 2^53）返回错误而非静默截断 | M3 |
| `hexdump` | (1) 5 字节截断文件不 panic；(2) `--annotate` 在损坏文件上优雅退出 | H1 |
| `batch` | (1) 部分失败返回 exit 2；(2) 空 glob 返回 exit 0 但 stderr 提示 | M4 |
