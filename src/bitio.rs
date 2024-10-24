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

    pub fn write_varint(&mut self, value: i16) -> Result<()> {
        let (width, data) = BitWriter::varint_convert(value);
        if width > 0 {
            for i in 0..width {
                self.write_bit(((data >> i) & 1) as u8)?;
            }
        }
        return Ok(());
    }

    pub fn flush(&mut self) -> Result<()> {
        if self.bit_pos > 0 {
            self.writer.write_all(&self.data)?;
            self.bit_pos = 0;
            self.data[0] = 0;
        }
        return Ok(());
    }

    fn adjust_int(value: i16, offset: i16) -> i16 {
        if value >= 0 {
            return value - offset;
        } else {
            return !(-value - offset);
        }
    }

    fn varint_convert(value: i16) -> (usize, i16) {
        if value <= -1024 || value >= 1024 {
            return (11, BitWriter::adjust_int(value, 1024));
        }
        if value <= -512 || value >= 512 {
            return (10, BitWriter::adjust_int(value, 512));
        }
        if value <= -256 || value >= 256 {
            return (9, BitWriter::adjust_int(value, 256));
        }
        if value <= -128 || value >= 128 {
            return (8, BitWriter::adjust_int(value, 128));
        }
        if value <= -64 || value >= 64 {
            return (7, BitWriter::adjust_int(value, 64));
        }
        if value <= -32 || value >= 32 {
            return (6, BitWriter::adjust_int(value, 32));
        }
        if value <= -16 || value >= 16 {
            return (5, BitWriter::adjust_int(value, 16));
        }
        if value <= -8 || value >= 8 {
            return (4, BitWriter::adjust_int(value, 8));
        }
        if value <= -4 || value >= 4 {
            return (3, BitWriter::adjust_int(value, 4));
        }
        if value <= -2 || value >= 2 {
            return (2, BitWriter::adjust_int(value, 2));
        }
        if value <= -1 || value >= 1 {
            return (1, BitWriter::adjust_int(value, 1));
        }
        return (0, 0);
    }

    pub fn write_vec(&mut self, values: &[i8]) -> Result<()> {
        for d in values {
            if *d < 0 {
                return Ok(());
            }
            self.write_bit(*d as u8)?;
        }
        return Ok(());
    }
}
