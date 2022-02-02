use ic_cdk::api::stable::{stable_grow, stable_read, stable_write, StableMemoryError};
use ic_kit::candid::{Deserialize, CandidType};

pub trait Memory<E> {
    fn capacity(&self) -> u32;
    fn size(&self) -> usize;
    fn grow(&mut self, pages: u32) -> Result<(), E>;
    fn read(&self, offset: usize, dst: &mut [u8]) -> Result<usize, E>;
    fn write(&mut self, src: &[u8]) -> Result<usize, E>;
}

#[derive(Deserialize, CandidType, Default, Clone)]
pub struct StableMemory {
    /// current offset in stable memory
    pub(crate) offset: usize,
    /// current pages count in stable memory
    capacity: u32,
}

#[derive(Deserialize, CandidType, Default, Clone)]
pub struct Position {
    pub(crate) offset: usize,
    pub(crate) len: usize,
}

impl Position {
    fn new(offset: usize, len: usize)  -> Self {
        Self {
            offset,
            len
        }
    }
}

#[cfg(not(test))]
impl Memory<StableMemoryError> for StableMemory {
    /// get current pages count
    fn capacity(&self) -> u32 {
        self.capacity
    }

    /// get current memory size in bytes
    fn size(&self) -> usize {
        (self.capacity as usize) << 16
    }

    /// attempts to grow the memory by adding new pages
    fn grow(&mut self, pages: u32) -> Result<(), StableMemoryError> {
        let old_page_count = stable_grow(pages)?;
        self.capacity = old_page_count + pages;
        Ok(())
    }

    /// read bytes from offset to fill the buf, return bytes read
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize, StableMemoryError> {
        if offset + buf.len() > self.offset {
            return Err(StableMemoryError())
        }
        stable_read(offset as u32, buf);
        Ok(buf.len())
    }

    /// write bytes to stable memory, return bytes written
    fn write(&mut self, buf: &[u8]) -> Result<usize, StableMemoryError> {
        if self.offset + buf.len() >  self.size() {
            self.grow((buf.len() >> 16) as u32 + 1)?;
        }
        stable_write(self.offset as u32, buf);
        self.offset += buf.len();
        Ok(buf.len())
    }
}

#[cfg(test)]
impl Memory<StableMemoryError> for StableMemory {
    /// get current pages count
    fn capacity(&self) -> u32 {
        0
    }

    /// get current memory size in bytes
    fn size(&self) -> usize {
        0
    }

    /// attempts to grow the memory by adding new pages
    fn grow(&mut self, pages: u32) -> Result<(), StableMemoryError> {
        Ok(())
    }

    /// read bytes from offset to fill the buf, return bytes read
    fn read(&self, offset: usize, buf: &mut [u8]) -> Result<usize, StableMemoryError> {
        Ok(0)
    }

    /// write bytes to stable memory, return bytes written
    fn write(&mut self, buf: &[u8]) -> Result<usize, StableMemoryError> {
        Ok(0)
    }
}
