use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};
use bytesize::ByteSize;
use grep_matcher::Matcher;
use crate::{Searcher, Sink, SinkError};
use crate::searcher::parallel_default_searcher::work_pool::WorkPool;
use crate::searcher::parallel_default_searcher::worker::{BufferedWorker, split_into_ranges};

/// Searcher that performs it's search using multithreading
#[derive(Debug)]
pub struct ParallelDefaultSearcher {
    threads: usize,
    searcher: Searcher,
}

/// docs
impl ParallelDefaultSearcher {
    /// Create new parallel searcher
    pub fn new(threads: usize, searcher: Searcher) -> Self {
        ParallelDefaultSearcher { threads, searcher }
    }

    /// Execute a search over the file with the given path and write the
    /// results to the given sink.
    pub fn search_path<P, M, S>(
        &mut self,
        matcher: M,
        path: P,
        sink: Arc<Mutex<S>>,
    ) -> Result<(), S::Error>
        where
            P: AsRef<Path>,
            M: Matcher + Send + Sync,
            S: Sink + Send, <S as Sink>::Error: Send
    {
        let path = path.as_ref();
        let file = File::open(path).map_err(S::Error::error_io)?;
        let file_len = file.metadata().map_err(S::Error::error_io)?.len();
        let buf_size = file_len.min(ByteSize::mib(10).0) as usize;
        let file = Arc::new(file);

        let ranges = split_into_ranges(file_len, buf_size as u64);
        let queues = WorkPool::split_into_chunks(
            self.threads.min(ranges.len()),
            ranges,
        );
        std::thread::scope(|s| {
            let handles: Vec<_> = queues.into_iter()
                .map(|queue| {
                    BufferedWorker::new(
                        &file,
                        queue,
                        std::iter::repeat(0).take(buf_size).collect(),
                        self.searcher.clone(),
                        &matcher,
                        Arc::clone(&sink),
                    )
                })
                .map(|worker| s.spawn(|| worker.run()))
                .collect();

            for handle in handles {
                let _ = handle.join().unwrap();
            }
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;
    use std::fs::File;
    use std::os::unix::fs::FileExt;
    use std::path::Path;

    #[test]
    fn it_works() -> Result<(), Box<dyn Error>> {
        let path = Path::new("../../tmp/Windows.log");
        let a = File::open(path)?;
        let mut buf: Vec<u8> = vec![0; 4096];
        let c = a.read_at(&mut buf, 0)?;
        log::trace!("[ripgrep] searcher started for file: {:?}", &path);

        Ok(())
    }
}

mod work_pool {
    use std::collections::VecDeque;

    pub struct WorkPool<T> {
        work: VecDeque<T>,
    }

    impl<T> WorkPool<T> {
        pub fn split_into_chunks(threads: usize, init: Vec<T>) -> Vec<WorkPool<T>> where T: Clone {
            // Calculate the size of each chunk
            let chunk_size = init.len() / threads + if init.len() % threads > 0 { 1 } else { 0 };

            // Split the vector into chunks and collect them into a new vector
            init
                .chunks(chunk_size)
                .map(|chunk| VecDeque::from(chunk.to_vec()))
                .map(|v| WorkPool { work: v })
                .collect()
        }

        pub fn pop(&mut self) -> Option<T> {
            self.work.pop_front()
        }
    }
}

mod worker {
    use std::fs::File;
    use std::io::Cursor;
    use std::ops::Range;
    use std::os::unix::prelude::*;
    use std::sync::{Arc, Mutex};

    use bstr::ByteSlice;

    use grep_matcher::Matcher;

    use crate::{Searcher, Sink};
    use crate::searcher::parallel_default_searcher::work_pool::WorkPool;

    pub struct BufferedWorker<M: Matcher, S: Sink> {
        file: Arc<File>,
        queue: WorkPool<Range<u64>>,
        buffer: Cursor<Vec<u8>>,
        searcher: Searcher,
        matcher: M,
        sink: Arc<Mutex<S>>,
    }

    impl<M: Matcher, S: Sink> BufferedWorker<M, S> {
        pub fn new(
            file: &Arc<File>,
            queue: WorkPool<Range<u64>>,
            buffer: Vec<u8>,
            searcher: Searcher,
            matcher: M,
            sink: Arc<Mutex<S>>,
        ) -> Self {
            BufferedWorker {
                file: Arc::clone(file),
                queue,
                buffer: Cursor::new(buffer),
                searcher,
                matcher,
                sink,
            }
        }

        pub fn run(mut self) -> Result<(), S::Error> {
            while let Some(_) = self.fill_buffer() {
                let rdr = self.buffer.get_ref().as_bytes();
                let res = self.searcher.search_reader(&self.matcher, rdr, Arc::clone(&self.sink));
                res?
            }

            Ok(())
        }

        fn fill_buffer(&mut self) -> Option<usize> {
            let range = self.recv()?;
            let buffer = self.buffer.get_mut();
            buffer.fill(0);
            let n = self.file.read_at(buffer, range.start).ok()?;
            Some(n)
        }

        /// Receive work.
        fn recv(&mut self) -> Option<Range<u64>> {
            self.queue.pop()
        }
    }

    pub fn split_into_ranges(number: u64, step: u64) -> Vec<Range<u64>> {
        if number <= step {
            return vec![0..number];
        }

        let mut result = Vec::new();
        let mut start = 0;

        while start < number {
            let end = std::cmp::min(start + step, number);
            result.push(start..end);
            start = end;
        }

        result
    }

    #[cfg(test)]
    mod tests {
        use bytesize::ByteSize;

        use crate::searcher::parallel_default_searcher::worker::split_into_ranges;

        #[test]
        fn test_split_into_ranges() {
            let size = 28_012_696_901;
            let step = ByteSize::mib(1).0;

            let result = split_into_ranges(size, step);
            let last = result.last().unwrap();

            assert_eq!(size, last.end);
        }
    }
}
