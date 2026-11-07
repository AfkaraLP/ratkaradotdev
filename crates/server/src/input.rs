use ratkara_core::Key;

pub fn parse_keys(data: &[u8]) -> Vec<Key> {
    let mut keys = Vec::new();
    let mut rest = data;
    while let Some((&byte, tail)) = rest.split_first() {
        rest = tail;
        let key = match byte {
            0x1b => match rest {
                [b'[' | b'O', tail @ ..] => {
                    let end = tail
                        .iter()
                        .position(|b| (0x40..=0x7e).contains(b))
                        .unwrap_or(tail.len().saturating_sub(1));
                    let (params, fin) = (&tail[..end], tail.get(end).copied());
                    rest = tail.get(end + 1..).unwrap_or_default();
                    match (params, fin) {
                        (_, Some(b'A')) => Key::Up,
                        (_, Some(b'B')) => Key::Down,
                        (_, Some(b'C')) => Key::Right,
                        (_, Some(b'D')) => Key::Left,
                        (_, Some(b'H')) | (b"1" | b"7", Some(b'~')) => Key::Home,
                        (_, Some(b'F')) | (b"4" | b"8", Some(b'~')) => Key::End,
                        (b"5", Some(b'~')) => Key::PageUp,
                        (b"6", Some(b'~')) => Key::PageDown,
                        (_, Some(b'Z')) => Key::BackTab,
                        _ => continue,
                    }
                }
                _ => Key::Esc,
            },
            b'\r' | b'\n' => Key::Enter,
            b'\t' => Key::Tab,
            0x7f | 0x08 => Key::Backspace,
            0x03 | 0x04 => Key::CtrlC,
            _ => {
                let len = match byte {
                    0xf0.. => 4,
                    0xe0.. => 3,
                    0xc0.. => 2,
                    _ => 1,
                };
                let end = (len - 1).min(rest.len());
                let mut bytes = vec![byte];
                bytes.extend_from_slice(&rest[..end]);
                rest = &rest[end..];
                match std::str::from_utf8(&bytes)
                    .ok()
                    .and_then(|s| s.chars().next())
                {
                    Some(c) if !c.is_control() => Key::Char(c),
                    _ => continue,
                }
            }
        };
        keys.push(key);
    }
    keys
}
