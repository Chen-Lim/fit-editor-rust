# Testing Strategy

## 测试策略文档

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
