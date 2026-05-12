# FIT File Format Reference

## FIT 文件格式速查

### 1. 文件结构

```
┌──────────────────────────┐
│     File Header          │  12 或 14 字节
│  (header_size, protocol, │
│   profile, data_size,    │
│   signature ".FIT",      │
│   [header_crc])          │
├──────────────────────────┤
│     Data Records         │  header.data_size 字节
│  ┌────────────────────┐  │
│  │ Record Header (1B) │  │  定义消息 或 数据消息
│  │ Definition/Data    │  │
│  └────────────────────┘  │
│  ... (N records) ...     │
├──────────────────────────┤
│     File CRC             │  2 字节 (LE u16)
└──────────────────────────┘
```

### 2. File Header

#### 12-byte Header
```
Offset  Size  Field
0       1     header_size (0x0C = 12)
1       1     protocol_version (e.g. 0x20 = v2.0)
2       2     profile_version (LE u16, e.g. 21200)
4       4     data_size (LE u32)
8       4     signature (ASCII ".FIT")
```

#### 14-byte Header (额外)
```
Offset  Size  Field
0-11    12    同上
12      2     header_crc (LE u16, 0 = 跳过校验)
```

### 3. Record 结构

#### Definition Message
```
┌──────────────┬───────────┬──────────────────────────┐
│ Record Header│ Reserved  │ Field Definitions         │
│ 1 byte       │ 1 byte    │ N × 3 bytes              │
│ bit7=1 (def) │ (0x00)    │ (def_num, size, base_type)│
│ bit5=dev_data│           │                          │
│ bits 0-3=local│          │                          │
├──────────────┼───────────┼──────────────────────────┤
│ arch (1B)    │ mesg_num  │ field_count (1B)         │
│ 0=LE, 1=BE   │ (2B, LE/BE)│                         │
└──────────────┴───────────┴──────────────────────────┘

Developer Fields (if bit5=1):
  dev_field_count (1B)
  N × 3 bytes: (def_num, size, dev_data_index)
```

#### Data Message
```
┌──────────────┬─────────────────────┐
│ Record Header│ Field Data          │
│ 1 byte       │ (按 Definition      │
│ bit7=0       │  定义的顺序和大小)   │
│ bits 0-3=local│                    │
└──────────────┴─────────────────────┘
```

#### Compressed Timestamp Message
```
┌──────────────┬─────────────────────┐
│ Record Header│ Non-timestamp Fields│
│ 1 byte       │ (跳过 timestamp     │
│ bit7=1       │  字段，从 header    │
│ bits 6-5=local│ 的 bits 4-0 重建)  │
│ bits 4-0=offset│                   │
└──────────────┴─────────────────────┘
```

### 4. Base Types

| Code | 类型 | 大小 | 无效值 | 说明 |
|------|------|------|--------|------|
| 0x00 | Enum | 1 | 0xFF | 枚举 |
| 0x01 | SInt8 | 1 | 0x7F | 有符号 8-bit |
| 0x02 | UInt8 | 1 | 0xFF | 无符号 8-bit |
| 0x03 | SInt16 | 2 | 0x7FFF | 有符号 16-bit (LE) |
| 0x04 | UInt16 | 2 | 0xFFFF | 无符号 16-bit |
| 0x05 | SInt32 | 4 | 0x7FFFFFFF | 有符号 32-bit |
| 0x06 | UInt32 | 4 | 0xFFFFFFFF | 无符号 32-bit |
| 0x07 | String | N | 0x00 | UTF-8 字符串 |
| 0x08 | Float32 | 4 | 0xFFFFFFFF | IEEE 754 |
| 0x09 | Float64 | 8 | 0xFFFFFFFFFFFFFFFF | IEEE 754 |
| 0x0A | UInt8z | 1 | 0x00 | 零为无效 |
| 0x0B | UInt16z | 2 | 0x0000 | 零为无效 |
| 0x0C | UInt32z | 4 | 0x0000 | 零为无效 |
| 0x0D | Byte | 1 | 0xFF | 原始字节 |
| 0x0E | SInt64 | 8 | 0x7FFFFFFFFFFFFFFF | 有符号 64-bit |
| 0x0F | UInt64 | 8 | 0xFFFFFFFFFFFFFFFF | 无符号 64-bit |
| 0x10 | UInt64z | 8 | 0x0000 | 零为无效 |

### 5. 时间戳体系

**FIT Epoch:** 1989-12-31 00:00:00 UTC (Unix: 631,065,600 秒)

```
FIT timestamp = seconds since 1989-12-31 00:00:00 UTC
Unix timestamp = FIT timestamp + 631,065,600

范围: 1989-12-31 ~ 2125-12-31 (u32)
```

**压缩时间戳:** 5-bit 偏移量 (0-31)，基于上一个完整时间戳重建:
- `offset >= last_5bits`: 同 epoch 窗口 → `ts = (last & ~0x1F) | offset`
- `offset < last_5bits`: 窗口前进 → `ts = (last & ~0x1F) + 0x20 | offset`

### 6. CRC 算法

```
CRC-16 (polynomial 0x8005, init 0x0000, no reflect, no xorout)

fn crc16_step(crc: u16, byte: u8) -> u16 {
    let mut tmp = (byte as u16) << 8;
    for _ in 0..8 {
        if ((tmp ^ crc) & 0x8000) != 0 {
            crc = (crc << 1) ^ 0x8005;
        } else {
            crc <<= 1;
        }
        tmp <<= 1;
    }
    crc
}
```

### 7. 常见消息字段参考

#### file_id (mesg_num=0)
| fdn | name | type | 说明 |
|-----|------|------|------|
| 0 | type | enum | file 类型 (activity/course/settings...) |
| 1 | manufacturer | enum | 制造商 |
| 2 | product | uint16 | 产品 ID |
| 3 | serial_number | uint32z | 设备序列号 |
| 4 | time_created | date_time | 创建时间 |
| 5 | number | uint16 | 文件编号 |
| 8 | product_name | string | 产品名称 |

#### session (mesg_num=18)
| fdn | name | type | scale | units |
|-----|------|------|-------|-------|
| 0 | sport | enum | | |
| 5 | start_time | date_time | | |
| 7 | total_elapsed_time | uint32 | 1000 | s |
| 9 | total_distance | uint32 | 100 | m |
| 11 | avg_speed | uint16 | 1000 | m/s |
| 16 | avg_heart_rate | uint8 | | bpm |
| 17 | max_heart_rate | uint8 | | bpm |
| 253 | timestamp | date_time | | |

#### record (mesg_num=20)
| fdn | name | type | scale | units |
|-----|------|------|-------|-------|
| 0 | position_lat | sint32 | | semicircles |
| 1 | position_long | sint32 | | semicircles |
| 2 | altitude | uint16 | 5, 500 | m |
| 3 | heart_rate | uint8 | | bpm |
| 6 | speed | uint16 | 1000 | m/s |
| 13 | temperature | sint8 | | °C |
| 253 | timestamp | date_time | | |

#### lap (mesg_num=19)
| fdn | name | type | scale | units |
|-----|------|------|-------|-------|
| 2 | start_time | date_time | | |
| 7 | total_elapsed_time | uint32 | 1000 | s |
| 9 | total_distance | uint32 | 100 | m |
| 11 | avg_speed | uint16 | 1000 | m/s |
| 16 | avg_heart_rate | uint8 | | bpm |
| 253 | timestamp | date_time | | |

#### activity (mesg_num=34)
| fdn | name | type | scale | units |
|-----|------|------|-------|-------|
| 0 | total_timer_time | uint32 | 1000 | s |
| 1 | num_sessions | uint16 | | |
| 2 | type | enum | | |
| 3 | event | enum | | |
| 5 | local_timestamp | date_time | | |
| 253 | timestamp | date_time | | |
