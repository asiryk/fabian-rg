use std::sync::Arc;

use grep_matcher::{Match, Matcher, NoCaptures, NoError};
use hash::{Hash, NeedleHash};

#[derive(Debug)]
pub struct RabinKarpMatcher {
    needle: Arc<Vec<u8>>,
}

impl RabinKarpMatcher {
    pub fn new(needle: &Arc<Vec<u8>>) -> Self {
        RabinKarpMatcher { needle: Arc::clone(needle) }
    }

    /// Compare byte-by-byte (naive search) by haystack.
    fn cmp_needle_bytes(&self, haystack: &[u8]) -> bool {
        self.needle.iter().zip(haystack).all(|(a, b)| a.eq(b))
    }
}

impl Clone for RabinKarpMatcher {
    fn clone(&self) -> Self {
        RabinKarpMatcher::new(&self.needle)
    }
}

impl Matcher for RabinKarpMatcher {
    type Captures = NoCaptures;
    type Error = NoError;

    fn find_at(&self, haystack: &[u8], at: usize) -> Result<Option<Match>, Self::Error> {
        let needle = &self.needle[..];
        let mut haystack = &haystack[at..];

        if needle.is_empty() { return Ok(None); };
        if haystack.len() < needle.len() { return Ok(None); }

        let start = haystack.as_ptr() as usize;
        let mut hash = Hash::new(&haystack[..needle.len()]);
        let nhash = NeedleHash::new(&needle);
        loop {
            if nhash.get_val().eq(&hash) && self.cmp_needle_bytes(haystack) {
                let match_start = at + haystack.as_ptr() as usize - start;
                let match_end = match_start + needle.len();
                let r#match = Match::new(match_start, match_end);
                return Ok(Some(r#match));
            }
            if needle.len() >= haystack.len() {
                return Ok(None);
            }
            hash.roll(haystack[0], haystack[needle.len()], nhash.get_len_pow_2());
            haystack = &haystack[1..];
        }
    }

    fn new_captures(&self) -> Result<Self::Captures, Self::Error> {
        Ok(NoCaptures::new())
    }
}

mod hash {
    #[derive(Debug, PartialEq, Eq)]
    pub struct Hash(u32);

    impl Hash {
        /// Create a new hash from byte array slice.
        pub fn new(bytes: &[u8]) -> Self {
            let mut hash = Hash(0);
            for &b in bytes {
                hash.add(b);
            }

            hash
        }

        /// Create a new hash, which is equal to empty string.
        pub fn new_empty() -> Self {
            Hash(0)
        }

        /// Delete byte from the "beginning" of the rolling hash and add byte to the "end".
        pub fn roll(&mut self, old: u8, new: u8, factor: u32) {
            self.del(old, factor);
            self.add(new);
        }

        /// Add byte to the "end" of rolling hash.
        /// It bitwise shifts current hash (which is equal to multiplication by 2) and adds new byte.
        pub fn add(&mut self, byte: u8) {
            let w = self.0.wrapping_shl(1);
            self.0 = w.wrapping_add(byte as u32);
        }

        /// Delete byte from the "beginning" of the rolling hash.
        /// Since it is all power of 2, first we need to multiply this byte by factor (2^len),
        /// and only then subtract from rolling hash.
        pub fn del(&mut self, byte: u8, factor: u32) {
            let w = (byte as u32).wrapping_mul(factor);
            self.0 = self.0.wrapping_sub(w);
        }
    }

    pub struct NeedleHash {
        hash: Hash,
        len_pow_2: u32,
    }

    impl NeedleHash {
        pub fn new(needle: &[u8]) -> Self {
            let mut nh = NeedleHash { hash: Hash::new_empty(), len_pow_2: 1 };
            if needle.is_empty() {
                return nh;
            }
            nh.hash.add(needle[0]);
            for &b in needle.iter().skip(1) {
                nh.hash.add(b);
                nh.len_pow_2 = nh.len_pow_2.wrapping_shl(1);
            }

            nh
        }

        pub fn get_val(&self) -> &Hash {
            &self.hash
        }

        pub fn get_len_pow_2(&self) -> u32 {
            self.len_pow_2
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn add_and_del_idempotent() {
            let mut hash = Hash::new_empty();

            hash.add(5);
            hash.add(3);
            hash.add(7);
            hash.add(13);
            hash.del(5, 8);
            hash.del(3, 4);
            hash.del(7, 2);
            hash.del(13, 1);

            assert_eq!(Hash::new_empty(), hash);
        }

        #[test]
        fn roll_and_new_equal() {
            let hash1 = Hash::new("hello".as_bytes());
            let mut hash2 = Hash::new("xhell".as_bytes());

            // remove x with factor of 16
            // x  | h | e | l | l
            // 16 | 8 | 4 | 2 | 1
            hash2.roll(b'x', b'o', 16);

            assert_eq!(hash1, hash2);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use grep_matcher::Matcher;

    use crate::rabin_karp_matcher::RabinKarpMatcher;

    #[test]
    fn find_at_some() {
        let haystack = b"a b c hello next";
        let needle = Arc::new(b"hello".to_vec());
        let matcher = RabinKarpMatcher::new(&needle);

        let result = matcher.find_at(haystack, 5);
        let r#match = result.unwrap().expect("expected to find a match");

        assert_eq!(r#match.len(), needle.len());
        assert_eq!(r#match.start(), 6,
                   "should return relative id from the needle start,but not 'at'");
        assert_eq!(r#match.end(), 6 + needle.len());
    }

    #[test]
    fn find_at_none() {
        let haystack = b"hello elloh";
        let needle = Arc::new(b"hello".to_vec());
        let matcher = RabinKarpMatcher::new(&needle);

        let result = matcher.find_at(haystack, 1);

        assert_eq!(None, result.unwrap(), "should not find a match")
    }
}
