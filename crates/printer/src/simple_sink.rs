use std::io;

use grep_searcher::{Searcher, Sink, SinkMatch};

///
pub struct SimpleSink {}

impl SimpleSink {
    ///
    pub fn new() -> Self {
        SimpleSink {}
    }
}

impl Sink for SimpleSink {
    type Error = io::Error;

    fn matched(&mut self, _searcher: &Searcher, mat: &SinkMatch<'_>) -> Result<bool, Self::Error> {
        let offset = mat.absolute_byte_offset();
        let range = mat.bytes_range_in_buffer();
        let result = String::from_utf8_lossy(mat.bytes());
        println!("offset = {:?}; range = {:?};\nresult = {}\n", offset, range, result);
        Ok(true)
    }
}
