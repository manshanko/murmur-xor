pub struct KeyTail([u8; 8]);

impl KeyTail {
    pub fn new(hash: u64, len: usize) -> Self {
        assert!(len < 8);
        let mut tail = hash.to_le_bytes();
        tail[7] = len as u8;
        Self(tail)
    }

    pub fn as_bytes(&self) -> &[u8] {
        let len = self.0[7];
        &self.0[..len as usize]
    }

    pub fn as_str(&self) -> &str {
        std::str::from_utf8(self.as_bytes()).unwrap()
    }
}

#[derive(PartialEq, Eq, Hash)]
struct KeyHash(u64);

impl KeyHash {
    // Uses 8th bit of every byte since we expect ASCII keys.
    const MASKS: [u64; 7] = [
        0b1111111111111111111111111111111111111111111111111111111110000000,
        0b1111111111111111111111111111111111111111111111111000000010000000,
        0b1111111111111111111111111111111111111111100000001000000010000000,
        0b1111111111111111111111111111111110000000100000001000000010000000,
        0b1111111111111111111111111000000010000000100000001000000010000000,
        0b1111111111111111100000001000000010000000100000001000000010000000,
        0b1111111110000000100000001000000010000000100000001000000010000000,
    ];

    fn from_hash(hash: u64) -> [Self; 7] {
        let part = crate::hash::mmh64a_undo_end(hash);
        let mut bucket = 0;
        Self::MASKS.map(|mask| {
            bucket += 1;
            Self((part & mask) | bucket)
        })
    }

    fn from_prefix(prefix: &[u8]) -> [(u64, Self); 7] {
        let parts = crate::hash::mmh64a_prefix7(prefix);
        assert_eq!(parts.len(), Self::MASKS.len());
        let mut bucket = 0;
        Self::MASKS.map(|mask| {
            let part = parts[bucket as usize];
            bucket += 1;
            (part, Self((part & mask) | bucket))
        })
    }
}

pub struct KeyLookup(crate::hash::MurmurHashMap<KeyHash, Vec<u64>>);

impl KeyLookup {
    pub fn new(hashes: &[u64]) -> Self {
        let mut lookup = crate::hash::MurmurHashMap::default();
        lookup.reserve(hashes.len() * 7);
        for hash in hashes {
            for key in KeyHash::from_hash(*hash) {
                let entry = lookup.entry(key).or_insert(Vec::new());
                entry.push(*hash);
            }
        }
        for (_, list) in &mut lookup {
            list.sort();
        }
        Self(lookup)
    }

    pub fn remove(&mut self, hash: u64) {
        for key in KeyHash::from_hash(hash) {
            let entry = self.0.get_mut(&key).unwrap();
            match entry.binary_search(&hash) {
                Ok(i) => {
                    entry.remove(i);
                }
                Err(_) => unreachable!(),
            }
        }
    }

    pub fn find_neighbors(&self, prefix: &[u8]) -> impl Iterator<Item = (u64, KeyTail)> {
        let mut parts = KeyHash::from_prefix(prefix)
            .into_iter();
        let mut len = 0;
        let mut group: Option<std::iter::Map<_, _>> = None;
        std::iter::from_fn(move || {
            if let Some(group) = &mut group {
                if let Some(next) = group.next() {
                    return Some(next);
                }
            }

            let (part, similar) = loop {
                len += 1;
                let (part, key_hash) = parts.next()?;
                if let Some(out) = self.0.get(&key_hash) {
                    break (part, out);
                }
            };

            group = Some(similar.iter().map(move |sim| {
                (*sim, KeyTail::new(part ^ crate::hash::mmh64a_undo_end(*sim), len))
            }));
            group.as_mut().unwrap().next()
        })
    }
}

#[cfg(test)]
mod test {
    use super::*;

    const KEY: &str = "known/key";

    const NEIGHBORS: [&str; 5] = [
        "known/key_a",
        "known/key_b",
        "known/key_c",
        "known/key_seven",
        "known/key_six6",
    ];

    #[test]
    fn key_lookup() {
        let hashes = NEIGHBORS.map(|s| crate::hash::mmh64a(s.as_bytes()));
        let lookup = KeyLookup::new(&hashes);

        let key = KEY;
        let mut map = std::collections::HashMap::new();
        for (hash, tail) in lookup.find_neighbors(key.as_bytes()) {
            let neighbor = format!("{}{}",
                &key[..key.len() - key.len() % 8],
                tail.as_str());
            map.insert(neighbor, hash);
        }

        assert!(map.len() == NEIGHBORS.len());
        for neighbor in NEIGHBORS {
            let hash = map.get(neighbor).unwrap();
            assert_eq!(*hash, crate::hash::mmh64a(neighbor.as_bytes()));
        }
    }
}
