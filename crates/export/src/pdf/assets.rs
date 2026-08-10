//! Bounded PNG decoding and image collection for deterministic PDF exports.

use std::io::Read;

use flate2::read::ZlibDecoder;
use question_model::AssetId;

use crate::{FlowBlock, PrintExam};

pub(super) const MAX_PNG_DECODED_PIXELS: usize = 20_000_000;
const MAX_PNG_DECODED_BYTES: usize = 80 * 1024 * 1024;

#[derive(Debug, Clone)]
pub(super) struct PngImage {
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) color: u8,
    pub(super) raw: Vec<u8>,
}

#[derive(Debug, Clone, Copy)]
struct PngLayout {
    width: u32,
    height: u32,
    color: u8,
    channels: usize,
    stride: usize,
    raw_len: usize,
    inflated_len: usize,
}

impl PngLayout {
    fn checked(width: u32, height: u32, color: u8) -> Result<Self, ()> {
        if width == 0 || height == 0 {
            return Err(());
        }
        let width_usize = usize::try_from(width).map_err(|_| ())?;
        let height_usize = usize::try_from(height).map_err(|_| ())?;
        let channels = match color {
            2 => 3,
            6 => 4,
            _ => return Err(()),
        };
        let pixels = width_usize.checked_mul(height_usize).ok_or(())?;
        if pixels > MAX_PNG_DECODED_PIXELS {
            return Err(());
        }
        let stride = width_usize.checked_mul(channels).ok_or(())?;
        let raw_len = stride.checked_mul(height_usize).ok_or(())?;
        let inflated_len = stride
            .checked_add(1)
            .and_then(|row_len| row_len.checked_mul(height_usize))
            .ok_or(())?;
        if raw_len > MAX_PNG_DECODED_BYTES || inflated_len > MAX_PNG_DECODED_BYTES {
            return Err(());
        }
        Ok(Self {
            width,
            height,
            color,
            channels,
            stride,
            raw_len,
            inflated_len,
        })
    }
}

struct IdatReader<'a> {
    chunks: Vec<&'a [u8]>,
    chunk: usize,
    offset: usize,
}

impl<'a> IdatReader<'a> {
    fn new(chunks: Vec<&'a [u8]>) -> Self {
        Self {
            chunks,
            chunk: 0,
            offset: 0,
        }
    }
}

impl Read for IdatReader<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        if buffer.is_empty() {
            return Ok(0);
        }
        while let Some(chunk) = self.chunks.get(self.chunk) {
            let remaining = &chunk[self.offset..];
            if remaining.is_empty() {
                self.chunk += 1;
                self.offset = 0;
                continue;
            }
            let copied = remaining.len().min(buffer.len());
            buffer[..copied].copy_from_slice(&remaining[..copied]);
            self.offset += copied;
            return Ok(copied);
        }
        Ok(0)
    }
}

pub(super) fn collect_assets(
    exam: &PrintExam,
    flow: &[Vec<FlowBlock>],
) -> Vec<(AssetId, PngImage)> {
    let mut assets = Vec::new();
    for question in flow {
        for block in question {
            match block {
                FlowBlock::Image { asset, .. } if !assets.iter().any(|(id, _)| id == asset) => {
                    assets.push((
                        *asset,
                        decode_png(&exam.asset(*asset).expect("validated asset").bytes)
                            .expect("validated PNG"),
                    ));
                }
                _ => {}
            }
        }
    }
    assets
}

/// True when a PNG can be carried faithfully by both deterministic writers.
pub(super) fn png_is_supported(bytes: &[u8]) -> bool {
    decode_png(bytes).is_ok()
}

pub(super) fn decode_png(bytes: &[u8]) -> Result<PngImage, ()> {
    if bytes.len() < 33 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" {
        return Err(());
    }
    let mut position = 8;
    let mut layout = None;
    let mut idat_chunks = Vec::new();
    let mut compressed_len = 0_usize;
    let mut seen_idat = false;
    let mut idat_finished = false;
    let mut seen_palette = false;
    let mut seen_iend = false;
    while position < bytes.len() {
        let header_end = position.checked_add(8).ok_or(())?;
        let header = bytes.get(position..header_end).ok_or(())?;
        let length = usize::try_from(u32::from_be_bytes(header[..4].try_into().map_err(|_| ())?))
            .map_err(|_| ())?;
        let kind: &[u8; 4] = header[4..].try_into().map_err(|_| ())?;
        if !kind.iter().all(u8::is_ascii_alphabetic) || !kind[2].is_ascii_uppercase() {
            return Err(());
        }
        let data_start = header_end;
        let data_end = data_start.checked_add(length).ok_or(())?;
        let chunk_end = data_end.checked_add(4).ok_or(())?;
        let data = bytes.get(data_start..data_end).ok_or(())?;
        let expected_crc = u32::from_be_bytes(
            bytes
                .get(data_end..chunk_end)
                .ok_or(())?
                .try_into()
                .map_err(|_| ())?,
        );
        if png_crc32(kind, data) != expected_crc {
            return Err(());
        }
        match kind {
            b"IHDR" => {
                if position != 8 || layout.is_some() || length != 13 {
                    return Err(());
                }
                let width = u32::from_be_bytes(data[..4].try_into().map_err(|_| ())?);
                let height = u32::from_be_bytes(data[4..8].try_into().map_err(|_| ())?);
                let color = data[9];
                if data[8] != 8 || data[10] != 0 || data[11] != 0 || data[12] != 0 {
                    return Err(());
                }
                layout = Some(PngLayout::checked(width, height, color)?);
            }
            b"PLTE" => {
                if layout.is_none()
                    || seen_palette
                    || seen_idat
                    || length == 0
                    || length > 768
                    || !length.is_multiple_of(3)
                {
                    return Err(());
                }
                seen_palette = true;
            }
            b"IDAT" => {
                if layout.is_none() || idat_finished {
                    return Err(());
                }
                seen_idat = true;
                compressed_len = compressed_len.checked_add(length).ok_or(())?;
                idat_chunks.try_reserve(1).map_err(|_| ())?;
                idat_chunks.push(data);
            }
            b"IEND" => {
                if layout.is_none() || !seen_idat || length != 0 {
                    return Err(());
                }
                seen_iend = true;
            }
            _ => {
                if layout.is_none() || kind[0].is_ascii_uppercase() {
                    return Err(());
                }
                if seen_idat {
                    idat_finished = true;
                }
            }
        }
        position = chunk_end;
        if seen_iend {
            break;
        }
    }
    if !seen_iend || position != bytes.len() || compressed_len == 0 {
        return Err(());
    }
    let layout = layout.ok_or(())?;
    let read_limit = layout.inflated_len.checked_add(1).ok_or(())?;
    let mut decoder = ZlibDecoder::new(IdatReader::new(idat_chunks));
    let mut inflated = Vec::new();
    inflated.try_reserve_exact(read_limit).map_err(|_| ())?;
    {
        let mut limited = (&mut decoder).take(u64::try_from(read_limit).map_err(|_| ())?);
        limited.read_to_end(&mut inflated).map_err(|_| ())?;
    }
    if inflated.len() != layout.inflated_len
        || decoder.total_out() != u64::try_from(layout.inflated_len).map_err(|_| ())?
        || decoder.total_in() != u64::try_from(compressed_len).map_err(|_| ())?
    {
        return Err(());
    }
    let mut raw = Vec::new();
    raw.try_reserve_exact(layout.raw_len).map_err(|_| ())?;
    raw.resize(layout.raw_len, 0);
    for row in 0..usize::try_from(layout.height).map_err(|_| ())? {
        let row_start = row.checked_mul(layout.stride + 1).ok_or(())?;
        let filter = *inflated.get(row_start).ok_or(())?;
        let source = inflated
            .get(row_start + 1..row_start + 1 + layout.stride)
            .ok_or(())?;
        let start = row.checked_mul(layout.stride).ok_or(())?;
        for x in 0..layout.stride {
            let left = if x >= layout.channels {
                raw[start + x - layout.channels]
            } else {
                0
            };
            let up = if row > 0 {
                raw[start + x - layout.stride]
            } else {
                0
            };
            let up_left = if row > 0 && x >= layout.channels {
                raw[start + x - layout.stride - layout.channels]
            } else {
                0
            };
            raw[start + x] = match filter {
                0 => source[x],
                1 => source[x].wrapping_add(left),
                2 => source[x].wrapping_add(up),
                3 => source[x].wrapping_add(((u16::from(left) + u16::from(up)) / 2) as u8),
                4 => source[x].wrapping_add(paeth(left, up, up_left)),
                _ => return Err(()),
            };
        }
    }
    Ok(PngImage {
        width: layout.width,
        height: layout.height,
        color: layout.color,
        raw,
    })
}

pub(super) fn png_crc32(kind: &[u8; 4], data: &[u8]) -> u32 {
    let mut crc = !0_u32;
    for byte in kind.iter().chain(data) {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            crc = (crc >> 1) ^ (0xedb8_8320_u32 & 0_u32.wrapping_sub(crc & 1));
        }
    }
    !crc
}

fn paeth(a: u8, b: u8, c: u8) -> u8 {
    let p = i32::from(a) + i32::from(b) - i32::from(c);
    let pa = (p - i32::from(a)).abs();
    let pb = (p - i32::from(b)).abs();
    let pc = (p - i32::from(c)).abs();
    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}
