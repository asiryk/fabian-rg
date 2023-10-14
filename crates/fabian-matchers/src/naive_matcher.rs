use std::sync::Arc;

use grep_matcher::{Match, Matcher, NoCaptures, NoError};

#[derive(Debug)]
pub struct NaiveMatcher {
    needle: Arc<Vec<u8>>,
}

impl NaiveMatcher {
    pub fn new(needle: &Arc<Vec<u8>>) -> Self {
        NaiveMatcher { needle: Arc::clone(needle) }
    }
}

impl Clone for NaiveMatcher {
    fn clone(&self) -> Self {
        NaiveMatcher::new(&self.needle)
    }
}

impl Matcher for NaiveMatcher {
    type Captures = NoCaptures;
    type Error = NoError;

    fn find_at(&self, haystack: &[u8], at: usize) -> Result<Option<Match>, Self::Error> {
        let needle = &self.needle[..];
        let haystack = &haystack[at..];

        for i in 0..(haystack.len() - needle.len() + 1) {
            let hay = haystack[i..(i + needle.len())].iter();

            if self.needle.iter().zip(hay).all(|(a, b)| a.eq(b)) {
                let match_start = i + at;
                let match_end = match_start + needle.len();
                let r#match = Match::new(match_start, match_end);

                return Ok(Some(r#match));
            }
        }

        Ok(None)
    }

    fn new_captures(&self) -> Result<Self::Captures, Self::Error> {
        Ok(NoCaptures::new())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use grep_matcher::Matcher;

    use crate::NaiveMatcher;

    #[test]
    fn find_at_some() {
        let haystack = b"a b c hello next";
        let needle = Arc::new(b"hello".to_vec());
        let matcher = NaiveMatcher::new(&needle);

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
        let matcher = NaiveMatcher::new(&needle);

        let result = matcher.find_at(haystack, 1);

        assert_eq!(None, result.unwrap(), "should not find a match")
    }
}
