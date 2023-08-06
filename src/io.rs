use std::ffi::CString;
use std::io::{Cursor, Read};
use byteorder::{LittleEndian, ReadBytesExt};

pub type Reader = Cursor<Vec<u8>>;

pub trait Readable {
    fn read(reader: &mut Reader) -> anyhow::Result<Self> where Self: Sized;
}

pub trait ReaderExt: Read {
    fn read_length_prefixed_cstring(&mut self) -> anyhow::Result<String> {
        let length = self.read_u32::<LittleEndian>()?;

        if length == 0 {
            return Ok("".to_owned());
        }

        let mut buf = vec![0; length as usize];

        self.read_exact(&mut buf)?;

        Ok(CString::from_vec_with_nul(buf)?.to_str()?.to_owned())
    }
}

impl ReaderExt for Reader { }