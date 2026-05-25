use std::path::Path;

use colored::Colorize;

use crate::error::{read_bounded, CliError, DEFAULT_MAX_FILE_SIZE};

pub fn run(
    file: &str,
    max_bytes: Option<usize>,
    skip_header: bool,
    annotate: bool,
) -> Result<(), CliError> {
    let path = Path::new(file);
    let bytes = read_bounded(path, DEFAULT_MAX_FILE_SIZE)?;

    if !fit::is_fit(&bytes) {
        return Err(CliError::NotFit(file.to_string()));
    }

    let start = if skip_header {
        let header = fit::FileHeader::parse(&bytes).map_err(CliError::from)?;
        header.header_size as usize
    } else {
        0
    };

    let end = max_bytes
        .map(|n| (start + n).min(bytes.len()))
        .unwrap_or(bytes.len());

    let data = &bytes[start..end];

    if annotate {
        hexdump_annotated(data, start, &bytes);
    } else {
        hexdump_plain(data, start);
    }

    Ok(())
}

fn hexdump_plain(data: &[u8], offset_base: usize) {
    for (i, chunk) in data.chunks(16).enumerate() {
        let offset = offset_base + i * 16;

        // Hex bytes
        let hex: Vec<String> = chunk.iter().map(|b| format!("{b:02x}")).collect();
        let hex_left: String = hex[..8.min(hex.len())].join(" ");
        let hex_right: String = if hex.len() > 8 {
            hex[8..].join(" ")
        } else {
            String::new()
        };

        // ASCII representation
        let ascii: String = chunk
            .iter()
            .map(|&b| {
                if (0x20..=0x7E).contains(&b) {
                    b as char
                } else {
                    '.'
                }
            })
            .collect();

        println!(
            "{:08x}:  {:<24}  {:<24}  |{}|",
            offset, hex_left, hex_right, ascii
        );
    }
}

fn hexdump_annotated(data: &[u8], offset_base: usize, full_bytes: &[u8]) {
    hexdump_plain(data, offset_base);

    println!();

    // Try to annotate message boundaries using the full file
    let header = match fit::FileHeader::parse(full_bytes) {
        Ok(h) => h,
        Err(_) => return,
    };

    let data_start = header.header_size as usize;
    let data_end = header.total_file_size() - 2; // exclude CRC
    let file_data = &full_bytes[data_start..data_end];

    println!("{}", "Annotations:".bold());

    let mut pos = 0usize;
    let mut msg_idx = 0usize;

    while pos < file_data.len() {
        if pos >= file_data.len() {
            break;
        }

        let header_byte = file_data[pos];
        let is_definition = (header_byte & 0x40) != 0;
        let local_num = header_byte & 0x0F;

        if is_definition {
            let def_size = estimate_definition_size(&file_data[pos..]);
            let abs_offset = data_start + pos;
            println!(
                "  [{:08x}] Definition #{} (local={}) ~{} bytes",
                abs_offset, msg_idx, local_num, def_size
            );
            pos += def_size;
        } else {
            // Data message — we need to have seen the definition to know size
            let abs_offset = data_start + pos;
            println!(
                "  [{:08x}] Data #{} (local={})",
                abs_offset, msg_idx, local_num
            );
            // Skip just the header byte + estimate 8 bytes as fallback
            pos += 1 + 8;
        }
        msg_idx += 1;
    }
}

/// Estimate definition message size from raw bytes.
/// Record header(1) + reserved(1) + arch(1) + mesg_num(2) + field_count(1) + fields(N*3)
fn estimate_definition_size(data: &[u8]) -> usize {
    if data.len() < 6 {
        return data.len();
    }

    let has_dev_data = (data[0] & 0x20) != 0;
    let field_count = data[5] as usize;
    let mut size = 1 + 1 + 1 + 2 + 1 + field_count * 3; // header + reserved + arch + mesg_num + count + fields

    if has_dev_data && size < data.len() {
        let dev_field_count = data[size] as usize;
        size += 1 + dev_field_count * 3;
    }

    size.min(data.len())
}
