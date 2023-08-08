use std::io::{Cursor, Read, Seek};
use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use flate2::read::ZlibDecoder;
use crate::io::Reader;

const ARCHIVE_V2_HEADER_TAG: u64 = 0x22222222_9E2A83C1;

#[derive(Debug)]
pub enum Compressor {
    Custom(String),
    None,
    Oodle,
    Zlib,
    Gzip,
    LZ4,
}

impl Compressor {
    fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let compressor = match reader.read_u8()? {
            0 => bail!("Custom compressor"),
            1 => Compressor::None,
            2 => Compressor::Oodle,
            3 => Compressor::Zlib,
            4 => Compressor::Gzip,
            5 => Compressor::LZ4,
            _ => bail!("Unknown compressor"),
        };

        Ok(compressor)
    }
}

#[derive(Debug)]
pub struct SavFile {
    // crc32 of the uncompressed data with content_size and version prepended and
    // the first 4 bytes of the uncompressed data need to be removed
    pub crc32: u32,
    pub content_size: u32, // uncompressed size + 8
    pub version: u32,
    pub chunks: Vec<SavChunk>,
}

impl SavFile {
    pub fn get_content(&self) -> anyhow::Result<Vec<u8>> {
        let mut uncompressed_data = Vec::with_capacity(self.content_size as usize);

        uncompressed_data.write_u32::<LittleEndian>(self.crc32)?;
        uncompressed_data.write_u32::<LittleEndian>(self.content_size)?;

        for chunk in &self.chunks {
            ZlibDecoder::new(chunk.compressed_data.as_slice())
                .read_to_end(&mut uncompressed_data)?;
        }

        let mut cursor = Cursor::new(uncompressed_data);

        cursor.seek(std::io::SeekFrom::Start(8))?;
        cursor.write_u32::<LittleEndian>(self.version)?;

        Ok(cursor.into_inner())
    }
}

impl SavFile {
    pub fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let size = reader.get_ref().len() as u64;

        let crc32 = reader.read_u32::<LittleEndian>()?;
        let content_size = reader.read_u32::<LittleEndian>()?;
        let version = reader.read_u32::<LittleEndian>()?;
        
        let mut chunks = Vec::new();

        while reader.position() < size {
            chunks.push(SavChunk::read(reader)?);
        }

        let sav_file = SavFile {
            crc32,
            content_size,
            version,
            chunks,
        };

        Ok(sav_file)
    }
}

#[derive(Debug)]
pub struct FCompressedChunkInfo {
    pub compressed_size: u64,
    pub uncompressed_size: u64,
}

impl FCompressedChunkInfo {
    fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let compressed_size = reader.read_u64::<LittleEndian>()?;
        let uncompressed_size = reader.read_u64::<LittleEndian>()?;

        let compression_info = FCompressedChunkInfo {
            compressed_size,
            uncompressed_size,
        };

        Ok(compression_info)
    }
}

#[derive(Debug)]
pub struct SavChunk {
    pub package_file_tag: u64,
    pub compressor: Compressor,
    pub compression_info: FCompressedChunkInfo,
    pub compressed_data: Vec<u8>,
}

impl SavChunk {
    fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let package_file_tag = reader.read_u64::<LittleEndian>()?;

        if package_file_tag != ARCHIVE_V2_HEADER_TAG {
            bail!("Unsupported package file tag: {}", package_file_tag);
        }

        let _uncompressed_size = reader.read_u64::<LittleEndian>()?; // can be ignored
        let compressor = Compressor::read(reader)?;
        let compression_info = FCompressedChunkInfo::read(reader)?;
        let _compression_info_2 = FCompressedChunkInfo::read(reader)?; // can be ignored

        let mut data = vec![0u8; compression_info.compressed_size as usize];

        reader.read_exact(&mut data)?;

        let sav_chunk = SavChunk {
            package_file_tag,
            compressor,
            compression_info,
            compressed_data: data,
        };

        Ok(sav_chunk)
    }
}
