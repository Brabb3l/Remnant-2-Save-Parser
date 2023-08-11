use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use std::ffi::CString;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};

pub struct Reader {
    cursor: Cursor<Vec<u8>>,
    pub object_padding: u32,
}

impl Reader {
    pub fn new(data: Vec<u8>, object_padding: u32) -> Self {
        Self {
            cursor: Cursor::new(data),
            object_padding,
        }
    }

    pub fn get_ref(&self) -> &Vec<u8> {
        self.cursor.get_ref()
    }

    pub fn position(&self) -> u64 {
        self.cursor.position()
    }
}

impl Read for Reader {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.cursor.read(buf)
    }
}

impl Seek for Reader {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.cursor.seek(pos)
    }
}

pub struct Writer {
    cursor: Cursor<Vec<u8>>,
    pub object_padding: u32,
}

impl Writer {
    pub fn new(buf: Vec<u8>, object_padding: u32) -> Self {
        Self {
            cursor: Cursor::new(buf),
            object_padding,
        }
    }

    pub fn get_ref(&self) -> &Vec<u8> {
        self.cursor.get_ref()
    }

    pub fn position(&self) -> u64 {
        self.cursor.position()
    }

    pub fn into_inner(self) -> Vec<u8> {
        self.cursor.into_inner()
    }
}

impl Write for Writer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.cursor.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.cursor.flush()
    }
}

impl Seek for Writer {
    fn seek(&mut self, pos: SeekFrom) -> std::io::Result<u64> {
        self.cursor.seek(pos)
    }
}

pub trait ReaderExt: Read {
    fn read_fstring(&mut self) -> anyhow::Result<String> {
        let length = self.read_u32::<LittleEndian>()?;

        if length == 0 {
            return Ok("".to_owned());
        }

        let mut buf = vec![0; length as usize];

        self.read_exact(&mut buf)?;

        Ok(CString::from_vec_with_nul(buf)?.to_str()?.to_owned())
    }
}

pub trait WriterExt: Write {
    fn write_fstring(&mut self, value: String) -> anyhow::Result<()> {
        if value.is_empty() {
            self.write_u32::<LittleEndian>(0)?;

            return Ok(());
        }

        let c_string = CString::new(value)?;
        let bytes = c_string.as_bytes_with_nul();

        self.write_u32::<LittleEndian>(bytes.len() as u32)?;
        self.write_all(bytes)?;

        Ok(())
    }
}

impl ReaderExt for Reader {}
impl WriterExt for Writer {}
