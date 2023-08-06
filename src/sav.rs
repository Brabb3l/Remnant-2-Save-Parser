use std::io::Read;
use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt};
use flate2::read::ZlibDecoder;
use crate::io::{Readable, Reader};

#[derive(Debug)]
pub struct SavFile {
    pub unknown: u64,
    pub version: u32,
    pub chunks: Vec<SavChunk>,
}

impl SavFile {
    pub fn get_uncompressed_size(&self) -> u64 {
        self.chunks.iter()
            .map(|chunk| chunk.uncompressed_size)
            .sum()
    }
}

impl SavFile {
    pub fn get_content(&self) -> anyhow::Result<Vec<u8>> {
        let mut uncompressed_data = Vec::with_capacity(self.get_uncompressed_size() as usize);

        for chunk in &self.chunks {
            ZlibDecoder::new(chunk.compressed_data.as_slice())
                .read_to_end(&mut uncompressed_data)?;
        }

        Ok(uncompressed_data)
    }
}

impl Readable for SavFile {
    fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let size = reader.get_ref().len() as u64;
        
        let unknown = reader.read_u64::<LittleEndian>()?;
        let version = reader.read_u32::<LittleEndian>()?;
        
        let mut chunks = Vec::new();

        while reader.position() < size {
            chunks.push(SavChunk::read(reader)?);
        }

        let sav_file = SavFile {
            unknown,
            version,
            chunks,
        };

        Ok(sav_file)
    }
}

#[derive(Debug)]
pub struct SavChunk {
    pub magic: u32,
    pub unknown: u32,
    pub uncompressed_block_size: u64,
    pub unknown2: u8,
    pub compressed_size: u64,
    pub uncompressed_size: u64,
    pub compressed_size_2: u64,
    pub uncompressed_size_2: u64,
    pub compressed_data: Vec<u8>,
}

impl Readable for SavChunk {
    fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let magic = reader.read_u32::<LittleEndian>()?;

        if magic != 0x9E2A83C1 {
            bail!("Invalid magic number: 0x{:X}", magic);
        }

        let unknown = reader.read_u32::<LittleEndian>()?;
        let uncompressed_block_size = reader.read_u64::<LittleEndian>()?;
        let unknown2 = reader.read_u8()?;
        let compressed_size = reader.read_u64::<LittleEndian>()?;
        let uncompressed_size = reader.read_u64::<LittleEndian>()?;
        let compressed_size_2 = reader.read_u64::<LittleEndian>()?;
        let uncompressed_size_2 = reader.read_u64::<LittleEndian>()?;

        let mut data = vec![0u8; compressed_size as usize];

        reader.read_exact(&mut data)?;

        let sav_chunk = SavChunk {
            magic,
            unknown,
            uncompressed_block_size,
            unknown2,
            compressed_size,
            uncompressed_size,
            compressed_size_2,
            uncompressed_size_2,
            compressed_data: data,
        };

        Ok(sav_chunk)
    }
}
