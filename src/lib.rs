use std::borrow::Borrow;
use std::sync::Arc;
use std::sync::Mutex;
use std::fs::File;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::ops::Range;
use std::io;
use std::collections::HashMap;
use std::fmt;

const DEBUG: bool = false;
const CHUNK_SIZE: usize = 0x8000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileViewError {
    IOError,
    OutOfBoundsError,
    RangeTooBigError
}

#[derive(Debug, Clone)]
pub struct FileView {
    inner: Arc<Mutex<FileViewInner>>,
    pub length: u64
}

#[derive(Debug)]
struct FileViewInner {
    file: File,
    buffers: HashMap<u64, FileViewChunkWrapper>
}

#[derive(Debug, Clone)]
struct FileViewChunkWrapper {
    readers: u64,
    inner: Arc<FileViewChunk>
}

struct FileViewChunk {
    offset: u64,
    data: Vec<u8>
}

#[derive(Clone)]
pub struct FileViewMapping {
    inner: Arc<FileViewMappingInner>
}

struct FileViewMappingInner {
    file_view: FileView,
    start_offset: usize,
    end_offset: usize,
    length: usize,
    buffers: Vec<Arc<FileViewChunk>>
}

pub struct FileViewMappingChunks<'a> {
    mapping: &'a FileViewMapping,
    index: usize
}

impl<'a> std::iter::Iterator for FileViewMappingChunks<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        let index = self.index;
        self.index += 1;
        if index < self.mapping.inner.buffers.len() {
            let begin = if index == 0 {
                self.mapping.inner.start_offset
            } else {
                0
            };
            let end = if index == self.mapping.inner.buffers.len()-1 {
                self.mapping.inner.end_offset
            } else {
                CHUNK_SIZE
            };
            Some(&self.mapping.inner.buffers[index].data[begin..end])
        } else {
            None
        }
    }
}

impl FileViewMapping {
    pub fn chunks(&self) -> FileViewMappingChunks {
        FileViewMappingChunks {
            mapping: &self,
            index: 0
        }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut v = Vec::with_capacity(self.inner.length);
        for chunk in self.chunks() {
            v.extend_from_slice(chunk)
        }
        v
    }
}

impl Drop for FileViewChunk {
    fn drop(&mut self) {
        if DEBUG {
            println!("dropping {:?}", &self);
        }
    }
}

impl FileView {
    pub fn new(file: File) -> Result<Self, FileViewError> {
        Ok(FileView {
            length: file.metadata()?.len(),
            inner: Arc::new(Mutex::new(FileViewInner {
                file: file,
                buffers: HashMap::new()
            }))
        })
    }

    fn handle_dropped_mapping(&self, mapping: &FileViewMappingInner) {
        let inner = &mut *self.inner.lock().unwrap();
        let mut stale_buffer_ids = vec!();
        for chunk in &mapping.buffers {
            let wrapper = inner.buffers.get_mut(&chunk.offset).unwrap();
            wrapper.readers -= 1;
            if wrapper.readers == 0 {
                stale_buffer_ids.push(chunk.offset)
            }
        }
        for stale_buffer_id in stale_buffer_ids {
            inner.buffers.remove(&stale_buffer_id).unwrap();
        }
    }

    pub fn read<R: Borrow<Range<u64>>>(&self, range: R) -> Result<FileViewMapping, FileViewError> {
        let inner = &mut *self.inner.lock().unwrap();
        let range = range.borrow();
        if DEBUG {
            println!("read({:x}-{:x}) (self.length={:x})", range.start, range.end, self.length)
        }
        if range.end > self.length {
            return Err(FileViewError::OutOfBoundsError);
        }
        let u64_len = range.end - range.start;
        if u64_len > std::usize::MAX as u64 {
            return Err(FileViewError::RangeTooBigError)
        }
        let len = u64_len as usize;
        let start_offset = (range.start % CHUNK_SIZE as u64) as usize;
        let mut end_offset;
        let base_address = range.start - range.start % CHUNK_SIZE as u64;
        let mut buffers = vec!();
        let mut address = base_address;
        inner.file.seek(SeekFrom::Start(base_address))?;
        loop {
            if inner.buffers.contains_key(&address) {
                inner.file.seek(SeekFrom::Current(CHUNK_SIZE as i64))?;
                let chunk = inner.buffers.get_mut(&address).unwrap();
                chunk.readers += 1;
                buffers.push(chunk.inner.clone())
            } else {
                let mut buffer = vec!(0; CHUNK_SIZE);
                inner.file.read(&mut buffer)?;
                let chunk = Arc::new(FileViewChunk {
                    offset: address,
                    data: buffer
                });
                inner.buffers.insert(address, FileViewChunkWrapper {
                    readers: 1,
                    inner: chunk.clone()
                });
                buffers.push(chunk);
            }
            end_offset = (range.end - address) as usize;
            address += CHUNK_SIZE as u64;
            if address >= range.end {
                break;
            }
        }

        Ok(FileViewMapping {
            inner: Arc::new(FileViewMappingInner {
                file_view: self.clone(),
                start_offset: start_offset,
                end_offset: end_offset,
                length: len,
                buffers: buffers
            })
        })
    }

    pub fn read_raw<R: Borrow<Range<u64>>>(&self, range: R) -> Result<Vec<u8>, FileViewError> {
        let inner = &mut *self.inner.lock().unwrap();
        let range = range.borrow();
        if DEBUG {
            println!("read_raw({:x}-{:x}) (self.length={:x})", range.start, range.end, self.length)
        }
        if range.end > self.length {
            return Err(FileViewError::OutOfBoundsError);
        }
        let u64_len = range.end - range.start;
        if u64_len > std::usize::MAX as u64 {
            return Err(FileViewError::RangeTooBigError)
        }
        let len = u64_len as usize;
        let mut buffer = vec!(0; len);
        inner.file.seek(SeekFrom::Start(range.start))?;
        inner.file.read_exact(&mut buffer)?;
        Ok(buffer)
    }
}

impl Drop for FileViewMappingInner {
    fn drop(&mut self) {
        if DEBUG {
            println!("dropping {:?}", &self);
        }
        self.file_view.handle_dropped_mapping(&self);
    }
}

impl fmt::Debug for FileViewMappingInner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FileViewMappingInner {{ start_offset: {:x}, length: {:x} }}", self.start_offset, self.length)
    }
}

impl fmt::Debug for FileViewChunk {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "FileViewChunk {{ begin: {:x}, end: {:x} }}", self.offset, self.offset + CHUNK_SIZE as u64)
    }
}

impl From<io::Error> for FileViewError {
    fn from(_: io::Error) -> Self {
        FileViewError::IOError
    }
}
