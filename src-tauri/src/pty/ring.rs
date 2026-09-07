use std::collections::VecDeque;

pub const RING_CAPACITY: usize = 262_144;

/// Oturum çıktısının son `cap` baytını tutar.
/// Kapasite aşıldığında baştan tam satırlar atılır; hiç satır sonu yoksa
/// ham bayt kırpmaya düşer.
pub struct RingBuffer {
    cap: usize,
    buf: VecDeque<u8>,
}

impl RingBuffer {
    pub fn with_capacity(cap: usize) -> Self {
        Self {
            cap,
            buf: VecDeque::with_capacity(cap.min(8192)),
        }
    }

    pub fn push(&mut self, data: &[u8]) {
        self.buf.extend(data.iter().copied());
        self.trim();
    }

    pub fn snapshot(&self) -> Vec<u8> {
        self.buf.iter().copied().collect()
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    fn trim(&mut self) {
        while self.buf.len() > self.cap {
            match self.buf.iter().position(|&b| b == b'\n') {
                // Satır sonuna kadar (dahil) at; ama bu tek başına yetmezse
                // döngü bir sonraki satırı da atar.
                Some(nl) => {
                    self.buf.drain(..=nl);
                }
                // Hiç satır sonu yok: ham kırp.
                None => {
                    let excess = self.buf.len() - self.cap;
                    self.buf.drain(..excess);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_everything_under_capacity() {
        let mut r = RingBuffer::with_capacity(64);
        r.push(b"hello\n");
        r.push(b"world\n");
        assert_eq!(r.snapshot(), b"hello\nworld\n");
    }

    #[test]
    fn never_exceeds_capacity() {
        let mut r = RingBuffer::with_capacity(16);
        for _ in 0..100 {
            r.push(b"0123456789\n");
        }
        assert!(r.len() <= 16, "len was {}", r.len());
    }

    #[test]
    fn evicts_whole_lines_from_the_front() {
        let mut r = RingBuffer::with_capacity(20);
        r.push(b"aaaa\nbbbb\ncccc\ndddd\n");
        r.push(b"eeee\n");
        // "aaaa\n" atılmalı; kalan tam satırlarla başlamalı
        let snap = r.snapshot();
        assert!(
            snap.starts_with(b"bbbb\n"),
            "snapshot: {:?}",
            String::from_utf8_lossy(&snap)
        );
        assert!(snap.ends_with(b"eeee\n"));
    }

    #[test]
    fn falls_back_to_raw_trim_when_no_newline_exists() {
        let mut r = RingBuffer::with_capacity(8);
        r.push(b"aaaaaaaaaaaaaaaa"); // 16 bayt, hiç newline yok
        assert_eq!(r.len(), 8);
        assert_eq!(r.snapshot(), b"aaaaaaaa");
    }

    #[test]
    fn handles_a_single_push_larger_than_capacity() {
        let mut r = RingBuffer::with_capacity(10);
        r.push(b"111\n222\n333\n444\n");
        assert!(r.len() <= 10);
        assert!(r.snapshot().ends_with(b"444\n"));
    }
}
