use super::fixed_big_int::*;

// ReplayDetector is the interface of sequence replay detector.
pub(crate) trait ReplayDetector {
    // Check returns true if given sequence number is not replayed.
    // Call accept() to mark the packet is received properly.
    fn check(&mut self, seq: u64) -> bool;
    fn accept(&mut self);
}

pub(crate) struct SlidingWindowDetector {
    accepted: bool,
    seq: u64,
    latest_seq: u64,
    max_seq: u64,
    window_size: usize,
    mask: FixedBigInt,
}

impl SlidingWindowDetector {
    // New creates ReplayDetector.
    // Created ReplayDetector doesn't allow wrapping.
    // It can handle monotonically increasing sequence number up to
    // full 64bit number. It is suitable for DTLS replay protection.
    pub(crate) fn new(window_size: usize, max_seq: u64) -> Self {
        SlidingWindowDetector {
            accepted: false,
            seq: 0,
            latest_seq: 0,
            max_seq,
            window_size,
            mask: FixedBigInt::new(window_size),
        }
    }
}

impl ReplayDetector for SlidingWindowDetector {
    fn check(&mut self, seq: u64) -> bool {
        self.accepted = false;

        if seq > self.max_seq {
            // Exceeded upper limit.
            return false;
        }

        if seq <= self.latest_seq {
            if self.latest_seq >= self.window_size as u64 + seq {
                return false;
            }
            if self.mask.bit((self.latest_seq - seq) as usize) != 0 {
                // The sequence number is duplicated.
                return false;
            }
        }

        self.accepted = true;
        self.seq = seq;
        true
    }

    fn accept(&mut self) {
        if !self.accepted {
            return;
        }

        if self.seq > self.latest_seq {
            // Update the head of the window.
            self.mask.lsh((self.seq - self.latest_seq) as usize);
            self.latest_seq = self.seq;
        }
        let diff = (self.latest_seq - self.seq) % self.max_seq;
        self.mask.set_bit(diff as usize);
    }
}