use grep_matcher::Matcher;

use crate::Sink;

/// Searcher that performs it's search using multithreading
#[derive(Debug)]
pub struct ParallelSearcher {}

impl ParallelSearcher {
    /// Create new parallel searcher
    pub fn new() -> Self {
        ParallelSearcher {}
    }

    /// Perform search
    #[allow(unused, unused_variables)]
    pub fn search_reader<M, R, S>(
        &mut self,
        matcher: M,
        read_from: R,
        write_to: S,
    ) -> Result<(), S::Error>
        where
            M: Matcher,
            R: std::io::Read,
            S: Sink,
    {
        todo!("parallel searcher is not implemented")
    }
}
