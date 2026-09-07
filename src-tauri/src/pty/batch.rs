pub const FLUSH_INTERVAL_MS: u64 = 8;
/// Lower bound, not a cap: the collector tests this only after appending a
/// whole PTY read (up to 64 KB), so a single flush can carry ~96 KB.
pub const FLUSH_BYTES: usize = 32_768;

/// Biriken baytların IPC'ye gönderilip gönderilmeyeceği.
///
/// Tauri'nin varsayılan olay yolu her mesajı JSON'a çevirir; `npm install`
/// gibi bir çıktı selinde bu tek başına CPU'yu yer. Toplama, saniyede
/// binlerce mesajı ~125 mesaja indirir.
pub fn should_flush(pending_len: usize, waited_ms: u64) -> bool {
    if pending_len == 0 {
        return false;
    }
    pending_len >= FLUSH_BYTES || waited_ms >= FLUSH_INTERVAL_MS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn does_not_flush_an_empty_buffer() {
        assert!(!should_flush(0, 0));
        assert!(!should_flush(0, 50));
    }

    #[test]
    fn flushes_once_the_interval_has_elapsed() {
        assert!(!should_flush(10, FLUSH_INTERVAL_MS - 1));
        assert!(should_flush(10, FLUSH_INTERVAL_MS));
    }

    #[test]
    fn flushes_early_when_the_byte_threshold_is_reached() {
        assert!(should_flush(FLUSH_BYTES, 0));
        assert!(!should_flush(FLUSH_BYTES - 1, 0));
    }
}
