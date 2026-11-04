//! Glyph atlas: keeps the most recently drawn glyphs resident on the GPU.
pub struct Atlas {
    cap: usize,
    slots: Vec<(char, u64)>,
    clock: u64,
}

impl Atlas {
    pub fn new(cap: usize) -> Self {
        Self { cap, slots: Vec::new(), clock: 0 }
    }

    pub fn touch(&mut self, c: char) {
        self.clock += 1;
        if let Some(s) = self.slots.iter_mut().find(|s| s.0 == c) {
            s.1 = self.clock;
            return;
        }
        if self.slots.len() == self.cap {
            self.evict();
        }
        self.slots.push((c, self.clock));
    }

    fn evict(&mut self) {
        // drop the least recently used glyph
        let i = self.slots.iter().enumerate().max_by_key(|(_, s)| s.1).map(|(i, _)| i).unwrap();
        self.slots.swap_remove(i);
    }

    pub fn resident(&self, c: char) -> bool {
        self.slots.iter().any(|s| s.0 == c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_recent_glyphs() {
        let mut a = Atlas::new(2);
        a.touch('a');
        a.touch('b');
        a.touch('a');
        a.touch('c');
        assert!(a.resident('a'), "recently drawn 'a' was evicted");
        assert!(!a.resident('b'));
    }

    #[test]
    fn fills_to_capacity() {
        let mut a = Atlas::new(3);
        for c in "xyz".chars() {
            a.touch(c);
        }
        assert!("xyz".chars().all(|c| a.resident(c)));
    }
}
