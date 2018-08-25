use std::sync::Arc;
use std::sync::Mutex;
use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::ops::Range;
use std::cmp;
use std::io;

const BUFFERS_COUNT: usize = 3;
const MINIMUM_BUFFER_SIZE: usize = 0x8000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileViewError {
    IOError
}

pub struct FileView {
    inner: Arc<Mutex<FileViewInner>>
}

struct FileViewInner {
    file: File,
    buffers: Vec<Arc<FileViewBuffer>>
}

struct FileViewBuffer {
    file_offset: u64,
    data: Vec<u8>
}

pub struct FileViewItem {
    buffer: Arc<FileViewBuffer>,
    buffer_offset: u64,
    len: u64
}

impl FileView {
    pub fn new(file: File) -> Self {
        FileView {
            inner: Arc::new(Mutex::new(FileViewInner {
                file: file,
                buffers: vec!()
            }))
        }
    }

    pub fn read(&self, range: Range<u64>) -> Result<FileViewItem, FileViewError> {
        let inner = &mut *self.inner.lock().unwrap();

        let buffer = if let Some(buffer) = inner.get(&range) {
            buffer
        } else {
            // load a new chunk from disk
            inner.file.seek(SeekFrom::Start(range.start)).unwrap();
            let mut new_buffer = vec!(0;cmp::max(MINIMUM_BUFFER_SIZE, (range.end - range.start) as usize));
            inner.file.read(&mut new_buffer)?;

            // add it to our buffers
            // println!("replacing cache ({}: {})", range.start, new_buffer.len());
            let file_view_buffer = Arc::new(FileViewBuffer {
                file_offset: range.start,
                data: new_buffer
            });
            if inner.buffers.len() < BUFFERS_COUNT {
                inner.buffers.push(file_view_buffer.clone())
            } else {
                inner.buffers[0] = file_view_buffer.clone()
            }
            file_view_buffer
        };

        Ok(FileViewItem {
            buffer_offset: range.start - buffer.file_offset,
            buffer: buffer,
            len: range.end - range.start
        })
    }
}

impl FileViewInner {
    fn get(&mut self, range: &Range<u64>) -> Option<Arc<FileViewBuffer>> {
        let hit_buffer = self.get_buffer(range);
        if let Some((i, buffer)) = hit_buffer {
            self.buffers.rotate_right(1);
            self.buffers.swap(i, 0);
            Some(buffer)
        } else {
            None
        }
    }

    fn get_buffer(&self, range: &Range<u64>) -> Option<(usize, Arc<FileViewBuffer>)> {
        for (i, buffer) in self.buffers.iter().enumerate() {
            if buffer.file_offset + (buffer.data.len() as u64) >= range.end && buffer.file_offset <= range.start {
                return Some((i, buffer.clone()))
            }
        }
        None
    }
}

impl FileViewItem {
    pub fn get(&self) -> &[u8] {
        let begin: usize = self.buffer_offset as usize;
        let end: usize = (self.buffer_offset + self.len) as usize;
        &self.buffer.data[begin..end]
    }
}

impl From<io::Error> for FileViewError {
    fn from(_: io::Error) -> Self {
        FileViewError::IOError
    }
}
