use std::io::{Cursor, Read};
use byteorder::{LittleEndian, ReadBytesExt};

use super::events::*;
use super::types::{ChannelInfo, FlpMetadata};

#[derive(Debug)]
pub enum ParseError {
    InvalidMagic,
    TruncatedHeader,
    IoError(std::io::Error),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::InvalidMagic => write!(f, "Not a valid FLP file (wrong magic bytes)"),
            ParseError::TruncatedHeader => write!(f, "FLP header is truncated"),
            ParseError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl From<std::io::Error> for ParseError {
    fn from(e: std::io::Error) -> Self {
        ParseError::IoError(e)
    }
}

/// Decode a byte slice as a string.
/// Detects UTF-16 LE (alternating null bytes or BOM), falls back to UTF-8.
/// Strips trailing null bytes.
fn decode_string(bytes: &[u8]) -> String {
    // Check for UTF-16 LE BOM (FF FE) or alternating nulls pattern
    let is_utf16 = bytes.starts_with(&[0xFF, 0xFE])
        || (bytes.len() >= 4
            && bytes.len() % 2 == 0
            && bytes[1] == 0
            && bytes[3] == 0);

    if is_utf16 {
        let start = if bytes.starts_with(&[0xFF, 0xFE]) { 2 } else { 0 };
        let u16_units: Vec<u16> = bytes[start..]
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        let mut s = String::from_utf16_lossy(&u16_units);
        // Strip trailing nulls
        while s.ends_with('\0') {
            s.pop();
        }
        return s;
    }

    // UTF-8 with replacement characters for invalid bytes, strip trailing nulls
    let s = String::from_utf8_lossy(bytes).into_owned();
    s.trim_end_matches('\0').to_string()
}

/// Parse an FLP file from raw bytes.
/// Returns FlpMetadata on success or ParseError for fatal errors.
/// For partial/truncated streams, returns Ok with warnings rather than Err.
pub fn parse_flp(bytes: &[u8]) -> Result<FlpMetadata, ParseError> {
    if bytes.len() < 4 {
        return Err(ParseError::InvalidMagic);
    }
    if &bytes[0..4] != b"FLhd" {
        return Err(ParseError::InvalidMagic);
    }

    let mut cursor = Cursor::new(bytes);
    cursor.set_position(4);

    // Header chunk size (always 6 for standard FLP)
    let _header_size = cursor
        .read_u32::<LittleEndian>()
        .map_err(|_| ParseError::TruncatedHeader)?;

    // Format version (u16 LE)
    let _format = cursor
        .read_u16::<LittleEndian>()
        .map_err(|_| ParseError::TruncatedHeader)?;

    // Number of channels (u16 LE)
    let channel_count = cursor
        .read_u16::<LittleEndian>()
        .map_err(|_| ParseError::TruncatedHeader)?;

    // PPQ (u16 LE)
    let _ppq = cursor
        .read_u16::<LittleEndian>()
        .map_err(|_| ParseError::TruncatedHeader)?;

    // Find the FLdt chunk
    // After the header chunk we expect: "FLdt" + 4-byte size + events
    let mut magic = [0u8; 4];
    if cursor.read_exact(&mut magic).is_err() {
        // No event data at all — return header-only metadata
        return Ok(FlpMetadata {
            channel_count,
            ..Default::default()
        });
    }

    if &magic != b"FLdt" {
        // Unrecognized chunk — return header-only metadata with warning
        let mut meta = FlpMetadata {
            channel_count,
            ..Default::default()
        };
        meta.warnings.push("FLdt chunk not found".to_string());
        return Ok(meta);
    }

    let _data_size = cursor.read_u32::<LittleEndian>().unwrap_or(0);

    // State for building channel info
    let mut meta = FlpMetadata {
        channel_count,
        ..Default::default()
    };

    let mut legacy_bpm: Option<f32> = None;
    let mut modern_bpm: Option<f32> = None;

    let mut current_name: Option<String> = None;
    let mut current_plugin: Option<String> = None;
    let mut current_chan_type: u8 = 0;
    let mut in_channel = false;

    // Flush the current in-progress channel into generators list
    let flush_channel =
        |name: &mut Option<String>,
         plugin: &mut Option<String>,
         chan_type: &mut u8,
         in_ch: &mut bool,
         generators: &mut Vec<ChannelInfo>| {
            if *in_ch || name.is_some() {
                let info = ChannelInfo {
                    name: name.take().unwrap_or_default(),
                    plugin_name: plugin.take(),
                    channel_type: *chan_type,
                };
                generators.push(info);
                *chan_type = 0;
                *in_ch = false;
            }
        };

    loop {
        // Read event ID
        let event_id = match cursor.read_u8() {
            Ok(id) => id,
            Err(_) => break, // end of stream
        };

        match event_id {
            // BYTE events (0-63): 1 byte value
            0..=63 => {
                let value = match cursor.read_u8() {
                    Ok(v) => v,
                    Err(_) => {
                        meta.warnings.push(format!(
                            "Truncated at BYTE event {} — partial data returned",
                            event_id
                        ));
                        break;
                    }
                };
                if event_id == FLP_CHAN_TYPE {
                    current_chan_type = value;
                }
            }

            // WORD events (64-127): 2 bytes LE
            64..=127 => {
                let value = match cursor.read_u16::<LittleEndian>() {
                    Ok(v) => v,
                    Err(_) => {
                        meta.warnings.push(format!(
                            "Truncated at WORD event {} — partial data returned",
                            event_id
                        ));
                        break;
                    }
                };
                match event_id {
                    x if x == FLP_NEW_CHAN => {
                        flush_channel(
                            &mut current_name,
                            &mut current_plugin,
                            &mut current_chan_type,
                            &mut in_channel,
                            &mut meta.generators,
                        );
                        in_channel = true;
                    }
                    x if x == FLP_NEW_PAT => {
                        meta.pattern_count += 1;
                    }
                    x if x == FLP_TEMPO_LEGACY => {
                        let bpm = value as f32;
                        if bpm < 1.0 || bpm > 999.0 {
                            meta.warnings.push(format!(
                                "Legacy BPM {} out of sane range (1-999) — ignoring",
                                bpm
                            ));
                        } else {
                            legacy_bpm = Some(bpm);
                        }
                    }
                    _ => {} // skip unknown WORD events
                }
            }

            // DWORD events (128-191): 4 bytes LE
            128..=191 => {
                let value = match cursor.read_u32::<LittleEndian>() {
                    Ok(v) => v,
                    Err(_) => {
                        meta.warnings.push(format!(
                            "Truncated at DWORD event {} — partial data returned",
                            event_id
                        ));
                        break;
                    }
                };
                if event_id == FLP_TEMPO {
                    let bpm = value as f32 / 1000.0;
                    if bpm < 1.0 || bpm > 999.0 {
                        meta.warnings.push(format!(
                            "Modern BPM {} out of sane range (1-999) — ignoring",
                            bpm
                        ));
                    } else {
                        modern_bpm = Some(bpm);
                    }
                }
                // skip all other DWORD events
            }

            // TEXT/VARIABLE events (192-255): varint length + payload bytes
            192..=255 => {
                let len = match read_varint(&mut cursor) {
                    Ok(l) => l as usize,
                    Err(_) => {
                        meta.warnings.push(format!(
                            "Truncated varint at TEXT event {} — partial data returned",
                            event_id
                        ));
                        break;
                    }
                };
                let mut payload = vec![0u8; len];
                if cursor.read_exact(&mut payload).is_err() {
                    meta.warnings.push(format!(
                        "Truncated payload at TEXT event {} — partial data returned",
                        event_id
                    ));
                    break;
                }
                let text = decode_string(&payload);
                match event_id {
                    x if x == FLP_TEXT_CHAN_NAME => {
                        current_name = Some(text);
                    }
                    x if x == FLP_TEXT_PLUGIN_NAME => {
                        current_plugin = Some(text);
                    }
                    x if x == FLP_VERSION => {
                        meta.fl_version = Some(text);
                    }
                    _ => {} // skip unknown TEXT events
                }
            }
        }
    }

    // Flush last in-progress channel
    flush_channel(
        &mut current_name,
        &mut current_plugin,
        &mut current_chan_type,
        &mut in_channel,
        &mut meta.generators,
    );

    // Resolve BPM: modern takes priority over legacy
    meta.bpm = modern_bpm.or(legacy_bpm);

    if meta.bpm.is_none() {
        meta.warnings.push("No BPM event found in file".to_string());
    }

    Ok(meta)
}

// ============================================================
// Unit tests using synthetic binary payloads
// ============================================================
#[cfg(test)]
mod tests {
    use super::*;

    // Build a minimal valid FLP header: "FLhd" + size(6) + format(0) + channels + ppq
    fn make_header(channels: u16, ppq: u16) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"FLhd");
        v.extend_from_slice(&6u32.to_le_bytes()); // header size always 6
        v.extend_from_slice(&0u16.to_le_bytes()); // format = 0
        v.extend_from_slice(&channels.to_le_bytes());
        v.extend_from_slice(&ppq.to_le_bytes());
        v
    }

    // Append the FLdt chunk prefix + all event bytes
    fn make_fldt(events: &[u8]) -> Vec<u8> {
        let mut v = Vec::new();
        v.extend_from_slice(b"FLdt");
        v.extend_from_slice(&(events.len() as u32).to_le_bytes());
        v.extend_from_slice(events);
        v
    }

    fn build_flp(channels: u16, ppq: u16, events: &[u8]) -> Vec<u8> {
        let mut v = make_header(channels, ppq);
        v.extend(make_fldt(events));
        v
    }

    // Encode a DWORD event (128-191)
    fn dword_event(id: u8, value: u32) -> Vec<u8> {
        let mut v = vec![id];
        v.extend_from_slice(&value.to_le_bytes());
        v
    }

    // Encode a WORD event (64-127)
    fn word_event(id: u8, value: u16) -> Vec<u8> {
        let mut v = vec![id];
        v.extend_from_slice(&value.to_le_bytes());
        v
    }

    // Encode a BYTE event (0-63)
    fn byte_event(id: u8, value: u8) -> Vec<u8> {
        vec![id, value]
    }

    // Encode a TEXT event (192-255) with varint-prefixed UTF-8 payload
    fn text_event(id: u8, text: &str) -> Vec<u8> {
        let payload = text.as_bytes();
        let len = payload.len();
        let mut v = vec![id];
        // Encode varint length
        let mut remaining = len;
        loop {
            let byte = (remaining & 0x7F) as u8;
            remaining >>= 7;
            if remaining > 0 {
                v.push(byte | 0x80);
            } else {
                v.push(byte);
                break;
            }
        }
        v.extend_from_slice(payload);
        v
    }

    // Encode a TEXT event with raw bytes (for UTF-16 tests)
    fn raw_text_event(id: u8, bytes: &[u8]) -> Vec<u8> {
        let len = bytes.len();
        let mut v = vec![id];
        let mut remaining = len;
        loop {
            let byte = (remaining & 0x7F) as u8;
            remaining >>= 7;
            if remaining > 0 {
                v.push(byte | 0x80);
            } else {
                v.push(byte);
                break;
            }
        }
        v.extend_from_slice(bytes);
        v
    }

    #[test]
    fn test_valid_header_parses() {
        let data = build_flp(4, 96, &[]);
        let meta = parse_flp(&data).expect("should parse valid header");
        assert_eq!(meta.channel_count, 4);
    }

    #[test]
    fn test_invalid_magic_returns_error() {
        let data = b"NOTFLP\x00\x00\x00\x00\x00\x00";
        let err = parse_flp(data).expect_err("should fail on wrong magic");
        assert!(matches!(err, ParseError::InvalidMagic));
    }

    #[test]
    fn test_empty_bytes_returns_invalid_magic() {
        let err = parse_flp(&[]).expect_err("empty bytes should fail");
        assert!(matches!(err, ParseError::InvalidMagic));
    }

    #[test]
    fn test_modern_bpm_event_156() {
        // BPM = 128.0 -> stored as 128000 in event 156
        let mut events = Vec::new();
        events.extend(dword_event(FLP_TEMPO, 128_000));
        let data = build_flp(1, 96, &events);
        let meta = parse_flp(&data).expect("should parse");
        assert!((meta.bpm.unwrap() - 128.0).abs() < 0.001, "BPM should be 128.0");
    }

    #[test]
    fn test_legacy_bpm_event_66() {
        let mut events = Vec::new();
        events.extend(word_event(FLP_TEMPO_LEGACY, 140));
        let data = build_flp(1, 96, &events);
        let meta = parse_flp(&data).expect("should parse");
        assert!((meta.bpm.unwrap() - 140.0).abs() < 0.001, "BPM should be 140.0");
    }

    #[test]
    fn test_modern_bpm_overrides_legacy() {
        // Both events present — modern (156) should win
        let mut events = Vec::new();
        events.extend(word_event(FLP_TEMPO_LEGACY, 120)); // legacy: 120 BPM
        events.extend(dword_event(FLP_TEMPO, 175_000));   // modern: 175 BPM
        let data = build_flp(1, 96, &events);
        let meta = parse_flp(&data).expect("should parse");
        assert!((meta.bpm.unwrap() - 175.0).abs() < 0.001, "Modern BPM should win");
    }

    #[test]
    fn test_no_bpm_produces_none_and_warning() {
        let data = build_flp(1, 96, &[]);
        let meta = parse_flp(&data).expect("should parse");
        assert!(meta.bpm.is_none(), "No BPM event -> bpm should be None");
        assert!(
            meta.warnings.iter().any(|w| w.contains("No BPM")),
            "Should warn about missing BPM"
        );
    }

    #[test]
    fn test_bpm_out_of_range_produces_warning() {
        // BPM = 0 from legacy event (0 < 1.0 threshold)
        let mut events = Vec::new();
        events.extend(word_event(FLP_TEMPO_LEGACY, 0));
        let data = build_flp(1, 96, &events);
        let meta = parse_flp(&data).expect("should parse");
        assert!(meta.bpm.is_none(), "Out-of-range BPM should be None");
        // Should have the out-of-range warning AND the no-BPM warning
        assert!(
            meta.warnings.iter().any(|w| w.contains("out of sane range")),
            "Should warn about out-of-range BPM"
        );
    }

    #[test]
    fn test_channel_name_and_plugin() {
        let mut events = Vec::new();
        // Start a channel, give it a name and plugin
        events.extend(word_event(FLP_NEW_CHAN, 0));
        events.extend(text_event(FLP_TEXT_CHAN_NAME, "Kick"));
        events.extend(text_event(FLP_TEXT_PLUGIN_NAME, "FPC"));
        // Start another channel to flush first one
        events.extend(word_event(FLP_NEW_CHAN, 1));
        events.extend(text_event(FLP_TEXT_CHAN_NAME, "Bass"));

        let data = build_flp(2, 96, &events);
        let meta = parse_flp(&data).expect("should parse");

        assert_eq!(meta.generators.len(), 2);
        assert_eq!(meta.generators[0].name, "Kick");
        assert_eq!(meta.generators[0].plugin_name.as_deref(), Some("FPC"));
        assert_eq!(meta.generators[1].name, "Bass");
        assert!(meta.generators[1].plugin_name.is_none());
    }

    #[test]
    fn test_pattern_count() {
        let mut events = Vec::new();
        // Three FLP_NewPat events
        events.extend(word_event(FLP_NEW_PAT, 1));
        events.extend(word_event(FLP_NEW_PAT, 2));
        events.extend(word_event(FLP_NEW_PAT, 3));
        let data = build_flp(1, 96, &events);
        let meta = parse_flp(&data).expect("should parse");
        assert_eq!(meta.pattern_count, 3);
    }

    #[test]
    fn test_unknown_event_ids_skipped() {
        let mut events = Vec::new();
        // Unknown DWORD event (e.g. 180) — should be skipped silently
        events.extend(dword_event(180, 0xDEADBEEF));
        // Then a real BPM event to verify parsing continued
        events.extend(dword_event(FLP_TEMPO, 90_000));
        let data = build_flp(1, 96, &events);
        let meta = parse_flp(&data).expect("should parse despite unknown event");
        assert!((meta.bpm.unwrap() - 90.0).abs() < 0.001);
    }

    #[test]
    fn test_fl_studio_version_extraction() {
        let mut events = Vec::new();
        events.extend(text_event(FLP_VERSION, "21.0.3.3517"));
        let data = build_flp(1, 96, &events);
        let meta = parse_flp(&data).expect("should parse");
        assert_eq!(meta.fl_version.as_deref(), Some("21.0.3.3517"));
    }

    #[test]
    fn test_truncated_file_returns_partial_with_warning() {
        // Valid header + FLdt magic + size, but then a DWORD event that's cut off
        let mut data = make_header(1, 96);
        data.extend_from_slice(b"FLdt");
        data.extend_from_slice(&100u32.to_le_bytes()); // claims 100 bytes of events
        // Only write the event ID, not the 4-byte DWORD value -> truncated
        data.push(FLP_TEMPO); // DWORD event ID 156

        let meta = parse_flp(&data).expect("truncated file should return partial result, not Err");
        // BPM will be None since event was truncated
        assert!(meta.bpm.is_none());
        // Should have a truncation warning
        assert!(
            meta.warnings.iter().any(|w| w.contains("Truncated")),
            "Should warn about truncation"
        );
    }

    #[test]
    fn test_utf16_string_decoding() {
        // Encode "Hi" as UTF-16 LE: H=0x48,0x00 i=0x69,0x00
        let utf16_bytes: Vec<u8> = vec![0x48, 0x00, 0x69, 0x00];
        let mut events = Vec::new();
        events.extend(raw_text_event(FLP_VERSION, &utf16_bytes));
        let data = build_flp(1, 96, &events);
        let meta = parse_flp(&data).expect("should parse UTF-16 string");
        assert_eq!(meta.fl_version.as_deref(), Some("Hi"));
    }

    #[test]
    fn test_channel_type_extraction() {
        let mut events = Vec::new();
        events.extend(word_event(FLP_NEW_CHAN, 0));
        events.extend(byte_event(FLP_CHAN_TYPE, 3)); // type 3
        events.extend(text_event(FLP_TEXT_CHAN_NAME, "Snare"));
        // Flush by ending stream
        let data = build_flp(1, 96, &events);
        let meta = parse_flp(&data).expect("should parse");
        assert_eq!(meta.generators.len(), 1);
        assert_eq!(meta.generators[0].channel_type, 3);
        assert_eq!(meta.generators[0].name, "Snare");
    }
}
