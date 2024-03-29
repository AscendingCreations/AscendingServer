use bytey::ByteBuffer;
use std::str;

pub trait ByteBufferExt {
    fn new_packet() -> bytey::Result<ByteBuffer>;
    fn new_packet_with(len: usize) -> bytey::Result<ByteBuffer>;
    fn write_str(&mut self, str: &str) -> bytey::Result<&mut ByteBuffer>;
    fn read_str(&mut self) -> bytey::Result<String>;
    fn finish(&mut self) -> bytey::Result<&mut ByteBuffer>;
}

impl ByteBufferExt for ByteBuffer {
    fn new_packet() -> bytey::Result<ByteBuffer> {
        ByteBuffer::new_packet_with(8)
    }

    fn new_packet_with(len: usize) -> bytey::Result<ByteBuffer> {
        let mut buffer = ByteBuffer::with_capacity(len + 8)?;
        buffer.write(0u64)?;
        Ok(buffer)
    }

    #[inline]
    fn write_str(&mut self, str: &str) -> bytey::Result<&mut Self> {
        let bytestr = str.as_bytes();
        self.write(bytestr.len() as u64)?;
        self.write_slice(bytestr)
    }

    #[inline]
    fn read_str(&mut self) -> bytey::Result<String> {
        let size = self.read::<u64>()? as usize;

        if size == 0 {
            return Ok(String::new());
        }

        match str::from_utf8(self.read_slice(size)?) {
            Ok(string) => Ok(String::from(string)),
            Err(_) => Ok(String::new()),
        }
    }

    #[inline]
    fn finish(&mut self) -> bytey::Result<&mut ByteBuffer> {
        self.move_cursor(0)?;
        self.write((self.length() - 8) as u64)?;
        self.move_cursor(0)
    }
}
