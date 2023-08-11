use serde::{Deserialize, Serialize};
use crate::components::Component;
use crate::properties::Property;
use crate::structs::{FName, FPackageVersion, FTopLevelAssetPath};

mod reader;
mod writer;

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

#[derive(Debug)]
pub struct SavFile {
    pub crc32: u32,
    pub content_size: u32,
    pub version: u32,
    pub chunks: Vec<SavChunk>,
}

#[derive(Debug)]
pub struct FCompressedChunkInfo {
    pub compressed_size: u64,
    pub uncompressed_size: u64,
}

#[derive(Debug)]
pub struct SavChunk {
    pub package_file_tag: u64,
    pub compressor: Compressor,
    pub compression_info: FCompressedChunkInfo,
    pub compressed_data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveGameArchiveHeader {
    pub save_game_file_version: u32,
    pub build_number: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveGameArchive {
    pub header: SaveGameArchiveHeader,
    pub content: SaveGameArchiveContent,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveGameArchiveContent {
    pub package_version: Option<FPackageVersion>,
    pub save_game_class_path: Option<FTopLevelAssetPath>,
    pub name_table: NameTable,
    pub object_index: Vec<UObject>,
    pub version: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NameTable {
    pub list: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UObject {
    pub object_id: u32,
    pub was_loaded: bool,
    pub object_path: String,
    pub loaded_data: Option<UObjectLoadedData>,
    pub properties: Vec<Property>,
    pub components: Option<Vec<Component>>, // Some if is actor
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UObjectLoadedData {
    pub name: FName,
    pub outer_id: u32,
}

