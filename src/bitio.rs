use std::io::{Read, Write};

use anyhow::{bail, Result};

pub struct BitWriter<'a> {
    data: [u8; 1],
    bit_pos: u8,
    writer: &'a mut dyn Write,
}

pub struct BitReader<'a> {
    data: [u8; 1],
    bit_pos: u8,
    reader: &'a mut dyn Read,
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
        /*if width == 5 {
            println!("{} {:016b}", value, data);
        }*/
        if width > 0 {
            for i in 0..width {
                self.write_bit(((data >> (width - 1 - i)) & 1) as u8)?;
            }
        }
        //print!("{}\n", value);
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
        //print!("vec:{:?}|", values);
        for d in values {
            if *d < 0 {
                return Ok(());
            }
            self.write_bit(*d as u8)?;
        }
        return Ok(());
    }
}

const WIDTH_OFFSETS: [i16; 12] = [0, 1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024];

impl<'a> BitReader<'a> {
    pub fn new(reader: &'a mut dyn Read) -> BitReader {
        BitReader {
            data: [0],
            bit_pos: 0,
            reader,
        }
    }

    pub fn read_bit(&mut self) -> Result<u8> {
        if self.bit_pos == 0 {
            self.reader.read_exact(&mut self.data)?;
        }
        let result = (self.data[0] >> self.bit_pos) & 1;
        self.bit_pos += 1;
        if self.bit_pos >= 8 {
            self.bit_pos = 0;
        }
        return Ok(result);
    }

    pub fn decode_huffman(&mut self, decoder: &[[i16; 2]]) -> Result<u8> {
        let mut dec_pos = 0i16;
        loop {
            let bit = self.read_bit()?;
            let next = decoder[dec_pos as usize][bit as usize];
            if next <= 0 {
                return Ok((-next) as u8);
            } else {
                dec_pos += next;
            }
        }
    }

    pub fn read_varint(&mut self, width: u8) -> Result<i16> {
        if width == 0 {
            return Ok(0);
        }
        if width > 11 {
            bail!("Width is too big: {}", width);
        }

        let mut result = 0i16;

        let sign = self.read_bit()?;
        if sign > 0 {
            result = !result;
        }
        //println!("{}", sign);

        for i in 0..width - 1 {
            let bit = self.read_bit()?;
            //println!("{}", bit);
            if bit > 0 {
                result |= 1 << (width - 2 - i);
            } else {
                result &= !(1 << (width - 2 - i));
            }
        }

        //println!("{:016b}", result);

        if sign > 0 {
            result = -!result - WIDTH_OFFSETS[width as usize];
        } else {
            result = result + WIDTH_OFFSETS[width as usize];
        }

        //println!("{:016b}", result);
        //print!("{}\n", result);
        return Ok(result);
    }
}
