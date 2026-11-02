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

//! Canonical transport records. Event/Session envelopes retain unknown nested fields.
use super::*;
use std::io::Cursor as IoCursor;
pub(crate) fn encode(value: Value) -> Result<Vec<u8>> {
    Ok(cbor::canonical_bytes(value)?)
}
pub(crate) fn decode(bytes: &[u8]) -> Result<Value> {
    let mut input = IoCursor::new(bytes);
    let value: Value = ciborium::de::from_reader(&mut input).map_err(|e| invalid(e.to_string()))?;
    if input.position() != bytes.len() as u64 {
        return Err(invalid("trailing CBOR"));
    }
    // Validate uniqueness recursively even for transport-owned future fields.
    encode(value.clone())?;
    Ok(value)
}
pub(crate) fn uint(v: &Value) -> Result<u64> {
    match v {
        Value::Integer(i) => u64::try_from(*i).map_err(|_| invalid("unsigned integer")),
        _ => Err(invalid("integer")),
    }
}
pub(crate) fn field(v: &Value, key: u64) -> Result<&Value> {
    match v {
        Value::Map(m) => m
            .iter()
            .find(|(k, _)| *k == Value::from(key))
            .map(|(_, v)| v)
            .ok_or_else(|| invalid(format!("missing key {key}"))),
        _ => Err(invalid("record map")),
    }
}
pub(crate) fn raw(value: &Value) -> Result<Vec<u8>> {
    encode(value.clone())
}
impl Header {
    pub(crate) fn encode(&self) -> Result<Vec<u8>> {
        let mut fields = vec![
            (0.into(), "BXL".into()),
            (1.into(), self.major.into()),
            (2.into(), self.minor.into()),
            (3.into(), self.session_id.0.to_string().into()),
            (4.into(), self.segment_no.into()),
            (5.into(), self.dict_id.into()),
            (6.into(), self.writer_id.0.to_string().into()),
            (
                7.into(),
                self.created
                    .as_ref()
                    .map_or(Value::Null, |s| s.clone().into()),
            ),
        ];
        fields.extend(self.unknown.clone());
        encode(Value::Map(fields))
    }
    pub(crate) fn decode(bytes: &[u8]) -> Result<Self> {
        let v = decode(bytes)?;
        if field(&v, 0)?.as_text() != Some("BXL") {
            return Err(invalid("header magic"));
        }
        let major = uint(field(&v, 1)?)?;
        if major != 1 {
            return Err(Error::Version {
                found: major,
                supported: 1,
            });
        }
        let id = |key| -> Result<ir::SessionId> {
            Ok(ir::SessionId(
                field(&v, key)?
                    .as_text()
                    .ok_or_else(|| invalid("ULID"))?
                    .parse()
                    .map_err(|_| invalid("ULID"))?,
            ))
        };
        let unknown = match &v {
            Value::Map(m) => m
                .iter()
                .filter(|(k, _)| uint(k).map_or(true, |n| n > 7))
                .cloned()
                .collect(),
            _ => unreachable!(),
        };
        Ok(Self {
            major,
            minor: uint(field(&v, 2)?)?,
            session_id: id(3)?,
            segment_no: uint(field(&v, 4)?)?,
            dict_id: uint(field(&v, 5)?)?
                .try_into()
                .map_err(|_| invalid("dict id"))?,
            writer_id: id(6)?,
            created: match field(&v, 7)? {
                Value::Null => None,
                Value::Text(s) => Some(s.clone()),
                _ => return Err(invalid("created timestamp")),
            },
            unknown,
        })
    }
}
impl Record {
    pub(crate) fn value(&self) -> Result<Value> {
        match self {
            Self::Event(e) => decode(&e.encode()?),
            Self::Session(s) => decode(&s.encode()?),
            Self::Blob(b) => {
                if blake3::hash(&b.bytes).as_bytes() != &b.blake3 {
                    return Err(invalid("blob hash"));
                }
                let mut fields = vec![
                    (0.into(), Value::Bytes(b.blake3.to_vec())),
                    (1.into(), Value::Bytes(b.bytes.clone())),
                ];
                fields.extend(b.unknown.clone());
                Ok(Value::Map(fields))
            }
            Self::Unknown(v) => Ok(v.clone()),
        }
    }
    pub(crate) fn parse(v: Value) -> Result<Self> {
        match field(&v, 0) {
            Ok(Value::Integer(_)) => Ok(Self::Event(cbor::decode(&raw(&v)?)?)),
            Ok(Value::Text(_)) => Ok(Self::Session(cbor::decode(&raw(&v)?)?)),
            Ok(Value::Bytes(hash)) if hash.len() == 32 => {
                let bytes = field(&v, 1)?
                    .as_bytes()
                    .ok_or_else(|| invalid("blob bytes"))?
                    .to_vec();
                let mut blob = Blob::new(bytes);
                if let Value::Map(fields) = &v {
                    blob.unknown = fields
                        .iter()
                        .filter(|(key, _)| !matches!(uint(key), Ok(0 | 1)))
                        .cloned()
                        .collect();
                }
                if blob.blake3.as_slice() != hash {
                    return Err(invalid("blob hash"));
                }
                Ok(Self::Blob(blob))
            }
            _ => Ok(Self::Unknown(v)),
        }
    }
}
impl Frame {
    pub(crate) fn encode(&self) -> Result<Vec<u8>> {
        let mut values = vec![
            self.seq.into(),
            self.first_event_seq.into(),
            (self.records.len() as u64).into(),
            self.source_cursor.clone().unwrap_or(Value::Null),
        ];
        for record in &self.records {
            values.push(record.value()?);
        }
        let bytes = encode(Value::Array(values))?;
        if bytes.len() > MAX_FRAME_BYTES {
            return Err(Error::Oversize);
        }
        Ok(bytes)
    }
    pub(crate) fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_FRAME_BYTES {
            return Err(Error::Oversize);
        }
        let Value::Array(values) = decode(bytes)? else {
            return Err(invalid("frame array"));
        };
        if values.len() < 4 || uint(&values[2])? != values.len() as u64 - 4 {
            return Err(invalid("record count"));
        }
        let mut next = uint(&values[1])?;
        let mut events = 0;
        let mut records = Vec::new();
        for v in values.iter().skip(4) {
            let record = Record::parse(v.clone())?;
            if let Some(seq) = record.seq() {
                if seq != next {
                    return Err(invalid("event sequence"));
                }
                next = next
                    .checked_add(1)
                    .ok_or_else(|| invalid("event sequence overflow"))?;
                events += 1;
            }
            records.push(record);
        }
        if events > MAX_EVENTS {
            return Err(invalid("event count"));
        }
        Ok(Self {
            seq: uint(&values[0])?,
            first_event_seq: uint(&values[1])?,
            source_cursor: if values[3] == Value::Null {
                None
            } else {
                Some(values[3].clone())
            },
            records,
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Entry {
    pub offset: u64,
    pub frame_seq: u64,
    pub first_event_seq: u64,
}
#[derive(Clone, Debug, Default)]
pub(crate) struct Index {
    pub segment_len: u64,
    pub last_frame_seq: Option<u64>,
    pub last_frame_xxh64: u64,
    pub entries: Vec<Entry>,
}
impl Index {
    pub fn bytes(&self) -> Result<Vec<u8>> {
        encode(Value::Map(vec![
            (0.into(), self.segment_len.into()),
            (
                1.into(),
                self.last_frame_seq.map_or(Value::Null, Value::from),
            ),
            (2.into(), self.last_frame_xxh64.into()),
            (
                3.into(),
                Value::Array(
                    self.entries
                        .iter()
                        .map(|e| {
                            Value::Array(vec![
                                e.offset.into(),
                                e.frame_seq.into(),
                                e.first_event_seq.into(),
                            ])
                        })
                        .collect(),
                ),
            ),
        ]))
    }
}
