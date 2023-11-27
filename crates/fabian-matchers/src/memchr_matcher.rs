use std::sync::Arc;

use memchr::arch::all::rabinkarp::Finder;

use grep_matcher::{Match, Matcher, NoCaptures, NoError};

#[derive(Debug)]
pub struct MemchrMatcher {
    needle: Arc<Vec<u8>>,
    rk_finder: Finder,
}

impl MemchrMatcher {
    pub fn new(needle: &Arc<Vec<u8>>) -> Self {
        let rk_finder = Finder::new(&needle);
        MemchrMatcher { needle: Arc::clone(needle), rk_finder }
    }
}

impl Clone for MemchrMatcher {
    fn clone(&self) -> Self {
        MemchrMatcher::new(&self.needle)
    }
}

impl Matcher for MemchrMatcher {
    type Captures = NoCaptures;
    type Error = NoError;

    fn find_at(&self, haystack: &[u8], at: usize) -> Result<Option<Match>, Self::Error> {
        let haystack = &haystack[at..];
        let result = self.rk_finder.find(haystack, &self.needle);

        Ok(result.map(|r| Match::new(at + r, at + r + self.needle.len())))
    }

    fn new_captures(&self) -> Result<Self::Captures, Self::Error> {
        Ok(NoCaptures::new())
    }
}
