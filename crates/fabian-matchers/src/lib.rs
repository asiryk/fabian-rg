/*!
This crate provides implementations of ripgrep Matcher trait with different algorithms.
 */

pub use naive_matcher::NaiveMatcher;
pub use rabin_karp_matcher::RabinKarpMatcher;

mod rabin_karp_matcher;
mod naive_matcher;
