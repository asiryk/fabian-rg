use std::fs::File;
use std::path::Path;
use std::sync::{Arc, Mutex};

use bytesize::ByteSize;

use grep_matcher::Matcher;

use crate::{Searcher, Sink, SinkError};
use crate::searcher::parallel_searcher::work_stealing::WorkStealingQueue;
use crate::searcher::parallel_searcher::worker::{BufferedWorker, split_into_ranges};

/// Searcher that performs it's search using multithreading
#[derive(Debug)]
pub struct ParallelSearcher {
    threads: usize,
    searcher: Searcher,
}

impl ParallelSearcher {
    /// Create new parallel searcher
    pub fn new(threads: usize, searcher: Searcher) -> Self {
        ParallelSearcher { threads, searcher }
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
        let queues = WorkStealingQueue::new_for_each_thread(
            self.threads.min(ranges.len()),
            ranges
        );
        std::thread::scope(|s| {
            let handles: Vec<_> = queues.into_iter()
                .map(|queue| BufferedWorker::new(
                    &file,
                    queue,
                    Vec::with_capacity(buf_size),
                    self.searcher.clone(),
                    &matcher,
                    Arc::clone(&sink),
                ))
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

mod work_stealing {
    use std::sync::Arc;

    use crossbeam_deque::{Stealer, Worker as Deque};

    /// A work-stealing stack.
    #[derive(Debug)]
    pub struct WorkStealingQueue<T> {
        /// This thread's index.
        index: usize,
        /// The thread-local stack.
        deque: Deque<T>,
        /// The work stealers.
        stealers: Arc<[Stealer<T>]>,
    }

    impl<T> WorkStealingQueue<T> {
        /// Create a work-stealing queue for each thread.
        pub fn new_for_each_thread(threads: usize, init: Vec<T>) -> Vec<WorkStealingQueue<T>> {
            let deques: Vec<Deque<T>> =
                std::iter::repeat_with(Deque::new_fifo).take(threads).collect();
            let stealers = Arc::<[Stealer<T>]>::from(
                deques.iter().map(Deque::stealer).collect::<Vec<_>>(),
            );
            let stacks: Vec<WorkStealingQueue<T>> = deques
                .into_iter()
                .enumerate()
                .map(|(index, deque)| WorkStealingQueue {
                    index,
                    deque,
                    stealers: stealers.clone(),
                })
                .collect();
            // Distribute the initial messages.
            init.into_iter()
                .zip(stacks.iter().cycle())
                .for_each(|(m, s)| {
                    s.push(m)
                });
            stacks
        }

        /// Push a message.
        pub fn push(&self, msg: T) {
            self.deque.push(msg);
        }

        /// Pop a message.
        pub fn pop(&self) -> Option<T> {
            self.deque.pop().or_else(|| self.steal())
        }

        /// Steal a message from another queue.
        pub fn steal(&self) -> Option<T> {
            // For fairness, try to steal from index - 1, then index - 2, ... 0,
            // then wrap around to len - 1, len - 2, ... index + 1.
            let (left, right) = self.stealers.split_at(self.index);
            // Don't steal from ourselves
            let right = &right[1..];

            left.iter()
                .rev()
                .chain(right.iter().rev())
                .map(|s| s.steal_batch_and_pop(&self.deque))
                .find_map(|s| s.success())
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
    use crate::searcher::parallel_searcher::work_stealing::WorkStealingQueue;

    pub struct BufferedWorker<M: Matcher, S: Sink> {
        file: Arc<File>,
        queue: WorkStealingQueue<Range<u64>>,
        buffer: Cursor<Vec<u8>>,
        searcher: Searcher,
        matcher: M,
        sink: Arc<Mutex<S>>,
    }

    impl<M: Matcher, S: Sink> BufferedWorker<M, S> {
        pub fn new(
            file: &Arc<File>,
            queue: WorkStealingQueue<Range<u64>>,
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
            buffer.clear();
            let n = self.file.read_at(buffer, range.start).ok()?;
            Some(n)
        }

        /// Receive work.
        fn recv(&self) -> Option<Range<u64>> {
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

        use crate::searcher::parallel_searcher::worker::split_into_ranges;

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
