use std::fs::File;
use std::path::Path;

use grep_matcher::Matcher;

use crate::{Sink, SinkError};

/// Searcher that performs it's search using multithreading
#[derive(Debug)]
pub struct ParallelSearcher {}

impl ParallelSearcher {
    /// Create new parallel searcher
    pub fn new() -> Self {
        ParallelSearcher {}
    }

    /// Execute a search over the file with the given path and write the
    /// results to the given sink.
    ///
    /// If memory maps are enabled and the searcher heuristically believes
    /// memory maps will help the search run faster, then this will use
    /// memory maps. For this reason, callers should prefer using this method
    /// or `search_file` over the more generic `search_reader` when possible.
    pub fn search_path<P, M, S>(
        &mut self,
        matcher: M,
        path: P,
        write_to: S,
    ) -> Result<(), S::Error>
        where
            P: AsRef<Path>,
            M: Matcher,
            S: Sink,
    {
        let path = path.as_ref();
        let file = File::open(path).map_err(S::Error::error_io)?;
        log::trace!("[ripgrep] searcher started for file: {:?}", &path);

        todo!("it's not implemented yet...");
        Ok(())
    }
}
