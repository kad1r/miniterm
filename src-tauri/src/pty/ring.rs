use std::collections::VecDeque;

pub const RING_CAPACITY: usize = 262_144;

/// Holds the last `cap` bytes of a session's output.
/// Past capacity, whole lines are dropped from the front; with no newline at
/// all it falls back to trimming raw bytes.
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
                // Drop up to and including the newline; if that alone is not
                // enough, the loop drops the next line too.
                Some(nl) => {
                    self.buf.drain(..=nl);
                }
                // No newline at all: trim raw bytes.
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

    // Global Constraint lock: RING_CAPACITY is 256 KB per session. Raising it
    // increases peak RSS proportionally (one buffer per terminal). Any change
    // must be deliberate — update this assertion with it.
    #[test]
    fn ring_capacity_is_256_kb() {
        assert_eq!(RING_CAPACITY, 262_144);
    }

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
        // "aaaa\n" must be dropped; the rest must start on a whole line
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
        r.push(b"aaaaaaaaaaaaaaaa"); // 16 bytes, no newline
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
