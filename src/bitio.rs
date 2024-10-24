use std::io::Write;

use anyhow::Result;

pub struct BitWriter<'a> {
    data: [u8; 1],
    bit_pos: u8,
    writer: &'a mut dyn Write,
}

impl<'a> BitWriter<'a> {
    pub fn new(writer: &'a mut dyn Write) -> BitWriter {
        BitWriter {
            data: [0],
            bit_pos: 0,
            writer,
        }
    }

    pub fn write_bit(&mut self, data: u8) -> Result<()> {
        if data > 0 {
            self.data[0] |= 1 << self.bit_pos;
        }
        self.bit_pos += 1;
        if self.bit_pos >= 8 {
            self.flush()?;
        }
        return Ok(());
    }

    pub fn flush(&mut self) -> Result<()> {
        self.writer.write_all(&self.data)?;
        self.bit_pos = 0;
        self.data[0] = 0;
        return Ok(());
    }
}
