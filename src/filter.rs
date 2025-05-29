const VALID_CHARACTERS: &[u8] = b"/0123456789_abcdefghijklmnopqrstuvwxyz";

const RANGE: u16 = {
    let len = VALID_CHARACTERS.len();
    assert!(len * len * len < u16::MAX as usize);
    len as u16
};

const LOOKUP: [u8; 256] = {
    let mut lookup = [u8::MAX; 256];
    let mut i = 0;
    while i < VALID_CHARACTERS.len() {
        let c = VALID_CHARACTERS[i];
        lookup[c as usize] = i as u8;
        i += 1;
    }
    lookup
};

#[inline]
fn trie_to_index(trie: &[u8]) -> u16 {
    let a = LOOKUP[trie[0] as usize] as u16;
    let b = LOOKUP[trie[1] as usize] as u16;
    let c = LOOKUP[trie[2] as usize] as u16;

    if a == 0xff || b == 0xff || c == 0xff {
        return u16::MAX;
    }

    (a * RANGE * RANGE)
        + (b * RANGE)
        + c
}

pub struct FilterTrie(Box<[u8; 0x10000]>);

impl FilterTrie {
    pub fn new() -> Self {
        let mut filter = Box::new([1; 0x10000]);
        filter[0xffff] = u8::MAX;
        Self(filter)
    }

    pub fn add_keys(&mut self, keys: &[&str]) {
        for key in keys {
            if !key.as_bytes().iter().all(|b| LOOKUP[*b as usize] < u8::MAX) {
                if cfg!(debug_assertions) {
                    panic!("key contains unexpected characters:\n{key}");
                } else {
                    continue;
                }
            }
            for trie in key.as_bytes().windows(3) {
                let idx = trie_to_index(trie);
                self.0[idx as usize] = 0;
            }
        }
    }

    pub fn check_trie(&self, prefix_end: [u8; 2], tail: &[u8]) -> bool {
        let mut unk: u32 = 0;

        let idx = trie_to_index(&[prefix_end[0], prefix_end[1], tail[0]]);
        unk += self.0[idx as usize] as u32;
        if tail.len() > 1 {
            let idx = trie_to_index(&[prefix_end[1], tail[0], tail[1]]);
            unk += self.0[idx as usize] as u32;
        }
        if tail.len() > 2 {
            for trie in tail.windows(3) {
                let idx = trie_to_index(trie);
                unk += self.0[idx as usize] as u32;
            }
        }

        match (tail.len(), unk) {
            (1..=5, 0..=4) => true,
            (6..=7, 0..=3) => true,
            _ => false,
        }
    }
}

pub fn is_valid(key: &[u8]) -> bool {
    key.iter().all(|b| LOOKUP[*b as usize] != 0xff)
}
