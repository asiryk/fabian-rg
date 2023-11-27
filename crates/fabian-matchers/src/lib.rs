/*!
This crate provides implementations of ripgrep Matcher trait with different algorithms.
 */

use std::sync::Arc;

use grep_matcher::{Match, Matcher, NoCaptures, NoError};
use memchr_matcher::MemchrMatcher;
use naive_matcher::NaiveMatcher;
use rabin_karp_matcher::RabinKarpMatcher;

mod rabin_karp_matcher;
mod naive_matcher;
mod memchr_matcher;

#[derive(Debug, Clone)]
enum InnerMatcher {
    Naive(NaiveMatcher),
    RabinKarp(RabinKarpMatcher),
    Memchr(MemchrMatcher),
}

#[derive(Debug, Clone)]
pub struct FabianMatcher {
    inner: InnerMatcher,
}

impl FabianMatcher {
    pub fn naive(needle: &Arc<Vec<u8>>) -> Self {
        FabianMatcher { inner: InnerMatcher::Naive(NaiveMatcher::new(needle)) }
    }

    pub fn rabin_karp(needle: &Arc<Vec<u8>>) -> Self {
        FabianMatcher { inner: InnerMatcher::RabinKarp(RabinKarpMatcher::new(needle)) }
    }

    pub fn memchr(needle: &Arc<Vec<u8>>) -> Self {
        FabianMatcher { inner: InnerMatcher::Memchr(MemchrMatcher::new(needle)) }
    }
}

impl Matcher for FabianMatcher {
    type Captures = NoCaptures;
    type Error = NoError;

    fn find_at(&self, haystack: &[u8], at: usize) -> Result<Option<Match>, Self::Error> {
        match &self.inner {
            InnerMatcher::RabinKarp(matcher) => matcher.find_at(haystack, at),
            InnerMatcher::Naive(matcher) => matcher.find_at(haystack, at),
            InnerMatcher::Memchr(matcher) => matcher.find_at(haystack, at),
        }
    }

    fn new_captures(&self) -> Result<Self::Captures, Self::Error> {
        match &self.inner {
            InnerMatcher::RabinKarp(matcher) => matcher.new_captures(),
            InnerMatcher::Naive(matcher) => matcher.new_captures(),
            InnerMatcher::Memchr(matcher) => matcher.new_captures(),
        }
    }
}
