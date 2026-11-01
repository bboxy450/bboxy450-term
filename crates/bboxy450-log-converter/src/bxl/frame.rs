// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-term — https://github.com/bboxy450/bboxy450-term
// License: https://github.com/bboxy450/bboxy450-term/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-term/blob/main/ATTRIBUTION.md
// Origin: P2 task #5084, ported from bboxy450-log-converter into bboxy450-term.
// Copyright 2026 Boxy (bboxy450) and contributors.
// SPDX-License-Identifier: LicenseRef-Respectful-Slop-1.0-Draft
// Project: bboxy450-log-converter — https://github.com/bboxy450/bboxy450-log-converter
// License: https://github.com/bboxy450/bboxy450-log-converter/blob/main/LICENSE
// Credit copied/adapted/translated reuse accurately; independent inspiration credit requested.
// Policy: https://github.com/bboxy450/bboxy450-log-converter/blob/main/ATTRIBUTION.md

//! Zstd framing from the published format, not from another session converter.
//! https://github.com/facebook/zstd/blob/v1.5.7/doc/zstd_compression_format.md
use super::*;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom, Write},
};
pub(crate) const HEADER_MAGIC: u32 = 0x184d2a50;
pub(crate) const SEQ_MAGIC: u32 = 0x184d2a51;
pub(crate) const SEEK_MAGIC: u32 = 0x184d2a5e;
const ZSTD_MAGIC: u32 = 0xfd2fb528;
const SEEK_FOOTER: u32 = 0x8f92eab1;
const MAX_COMPRESSED: usize = MAX_FRAME_BYTES + MAX_FRAME_BYTES / 128 + 256;
pub(crate) enum Physical {
    Skip {
        magic: u32,
        payload: Vec<u8>,
        len: u64,
    },
    Data {
        compressed: Vec<u8>,
        plain: Vec<u8>,
    },
}
fn fill(file: &mut File, buf: &mut Vec<u8>, n: usize) -> Result<bool> {
    if buf.len() + n > MAX_COMPRESSED {
        return Err(Error::Oversize);
    }
    let start = buf.len();
    buf.resize(start + n, 0);
    match file.read_exact(&mut buf[start..]) {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => Ok(false),
        Err(e) => Err(e.into()),
    }
}
pub(crate) fn read(file: &mut File, offset: u64, dict: &[u8]) -> Result<Option<Physical>> {
    file.seek(SeekFrom::Start(offset))?;
    let mut bytes = Vec::new();
    if !fill(file, &mut bytes, 4)? {
        return Ok(None);
    }
    let magic = u32::from_le_bytes(bytes[..4].try_into().unwrap());
    if (HEADER_MAGIC..=0x184d2a5f).contains(&magic) {
        if !fill(file, &mut bytes, 4)? {
            return Ok(None);
        }
        let len = u32::from_le_bytes(bytes[4..8].try_into().unwrap()) as u64;
        if len + 8 > ROTATE_BYTES {
            return Err(Error::Oversize);
        }
        let end = offset
            .checked_add(8 + len)
            .ok_or_else(|| invalid("skip size overflow"))?;
        if file.metadata()?.len() < end {
            return Ok(None);
        }
        let payload = if magic == HEADER_MAGIC || magic == SEQ_MAGIC {
            if len > MAX_FRAME_BYTES as u64 {
                return Err(Error::Oversize);
            }
            let mut payload = vec![0; len as usize];
            file.read_exact(&mut payload)?;
            payload
        } else if magic == SEEK_MAGIC {
            if len < 9 {
                return Err(invalid("seek table length"));
            }
            file.seek(SeekFrom::Start(end - 9))?;
            let mut footer = vec![0; 9];
            file.read_exact(&mut footer)?;
            let count = u32::from_le_bytes(footer[..4].try_into().unwrap()) as u64;
            if footer[4] != 0
                || u32::from_le_bytes(footer[5..].try_into().unwrap()) != SEEK_FOOTER
                || len != count * 8 + 9
            {
                return Err(invalid("seek table footer"));
            }
            footer
        } else {
            Vec::new()
        };
        return Ok(Some(Physical::Skip {
            magic,
            payload,
            len: 8 + len,
        }));
    }
    if magic != ZSTD_MAGIC {
        return Err(invalid("zstd magic"));
    }
    if !fill(file, &mut bytes, 1)? {
        return Ok(None);
    }
    let descriptor = bytes[4];
    if descriptor & 8 != 0 || descriptor & 4 == 0 {
        return Err(invalid("zstd checksum required / reserved bit"));
    }
    let single = descriptor & 32 != 0;
    let dict_len = [0, 1, 2, 4][(descriptor & 3) as usize];
    let size_len = match descriptor >> 6 {
        0 => usize::from(single),
        1 => 2,
        2 => 4,
        _ => 8,
    };
    if !fill(file, &mut bytes, usize::from(!single) + dict_len + size_len)? {
        return Ok(None);
    }
    loop {
        let start = bytes.len();
        if !fill(file, &mut bytes, 3)? {
            return Ok(None);
        }
        let block = u32::from_le_bytes([bytes[start], bytes[start + 1], bytes[start + 2], 0]);
        let kind = (block >> 1) & 3;
        if kind == 3 {
            return Err(invalid("reserved zstd block"));
        }
        let len = if kind == 1 { 1 } else { (block >> 3) as usize };
        if !fill(file, &mut bytes, len)? {
            return Ok(None);
        }
        if block & 1 != 0 {
            break;
        }
    }
    if !fill(file, &mut bytes, 4)? {
        return Ok(None);
    }
    let frame_dict = zstd::zstd_safe::get_dict_id_from_frame(&bytes).map_or(0, |n| n.get());
    let supplied = zstd::zstd_safe::get_dict_id_from_dict(dict).map_or(0, |n| n.get());
    if frame_dict != supplied {
        return Err(Error::DictionaryMissing { id: frame_dict });
    }
    let mut decoder = zstd::stream::read::Decoder::with_dictionary(bytes.as_slice(), dict)
        .map_err(|e| invalid(format!("zstd decode: {e}")))?;
    decoder.window_log_max(20)?;
    let mut plain = Vec::new();
    decoder
        .take(MAX_FRAME_BYTES as u64 + 1)
        .read_to_end(&mut plain)
        .map_err(|e| invalid(format!("zstd decode: {e}")))?;
    if plain.len() > MAX_FRAME_BYTES {
        return Err(Error::Oversize);
    }
    Ok(Some(Physical::Data {
        compressed: bytes,
        plain,
    }))
}
pub(crate) fn compress(plain: &[u8], dict: &[u8]) -> Result<Vec<u8>> {
    let mut encoder = zstd::stream::Encoder::with_dictionary(Vec::new(), 3, dict)?;
    encoder.include_checksum(true)?;
    encoder.window_log(20)?;
    encoder.set_pledged_src_size(Some(plain.len() as u64))?;
    encoder.write_all(plain)?;
    Ok(encoder.finish()?)
}
pub(crate) fn skippable(magic: u32, payload: &[u8]) -> Result<Vec<u8>> {
    let len = u32::try_from(payload.len()).map_err(|_| Error::Oversize)?;
    let mut out = magic.to_le_bytes().to_vec();
    out.extend(len.to_le_bytes());
    out.extend(payload);
    Ok(out)
}
/// Standard seekable entries include all preceding skippable frames with size 0.
/// First-event seq extensions live in a separate preceding skippable CBOR frame,
/// preserving the standard footer/entry layout for independent seekable tools.
pub(crate) fn seek_table(physical: &[(u32, u32)]) -> Result<Vec<u8>> {
    let mut payload = Vec::new();
    for (compressed, plain) in physical {
        payload.extend(compressed.to_le_bytes());
        payload.extend(plain.to_le_bytes());
    }
    payload.extend(
        u32::try_from(physical.len())
            .map_err(|_| Error::Oversize)?
            .to_le_bytes(),
    );
    payload.push(0);
    payload.extend(SEEK_FOOTER.to_le_bytes());
    skippable(SEEK_MAGIC, &payload)
}
pub(crate) fn hash(bytes: &[u8]) -> u64 {
    twox_hash::XxHash64::oneshot(0, bytes)
}
