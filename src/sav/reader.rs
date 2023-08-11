use std::io::{Cursor, Read, Seek, SeekFrom};
use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use flate2::bufread::ZlibDecoder;
use crate::components::{Component, ComponentType};
use crate::io::{Reader, ReaderExt, Writer};
use crate::properties::Property;
use crate::sav::{ARCHIVE_V2_HEADER_TAG, Compressor, FCompressedChunkInfo, NameTable, SavChunk, SaveGameArchive, SaveGameArchiveContent, SaveGameArchiveHeader, SavFile, UObject, UObjectLoadedData};
use crate::structs::{FName, FPackageVersion, FTopLevelAssetPath};

impl Compressor {
    fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let compressor = match reader.read_u8()? {
            0 => Compressor::Custom(reader.read_fstring()?),
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

        cursor.seek(SeekFrom::Start(8))?;
        cursor.write_u32::<LittleEndian>(self.version)?;

        // crc32 check

        cursor.seek(SeekFrom::Start(4))?;

        let mut crc32 = crc32fast::Hasher::new();

        crc32.update(&cursor.get_ref()[4..]);

        if crc32.finalize() != self.crc32 {
            bail!("CRC32 mismatch");
        }

        Ok(cursor.into_inner())
    }

    pub fn get_archive(&self) -> anyhow::Result<SaveGameArchive> {
        let content = self.get_content()?;
        let mut reader = Reader::new(content, 4);

        SaveGameArchive::read(&mut reader)
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

impl SaveGameArchiveHeader {
    fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let _crc32 = reader.read_u32::<LittleEndian>()?;
        let _size = reader.read_u32::<LittleEndian>()?;
        let save_game_file_version = reader.read_u32::<LittleEndian>()?;
        let build_number = reader.read_u32::<LittleEndian>()?;

        Ok(SaveGameArchiveHeader {
            save_game_file_version,
            build_number,
        })
    }
}

impl NameTable {
    pub fn read_name(&self, reader: &mut Reader) -> anyhow::Result<FName> {
        const HAS_NUMBER: u16 = 1 << 15;

        let mut index = reader.read_u16::<LittleEndian>()?;

        let number = if index & HAS_NUMBER != 0 {
            index &= !HAS_NUMBER;
            Some(reader.read_u32::<LittleEndian>()?)
        } else {
            None
        };

        let name = self.list.get(index as usize).unwrap().clone();

        Ok(FName {
            value: name,
            number,
        })
    }
}

impl SaveGameArchiveContent {
    pub fn read(
        reader: &mut Reader,
        has_ue_version: bool,
        has_top_level_asset_path: bool,
    ) -> anyhow::Result<Self> {
        let package_version = if has_ue_version {
            Some(FPackageVersion::read(reader)?)
        } else {
            None
        };

        let save_game_class_path = if has_top_level_asset_path {
            Some(FTopLevelAssetPath::read(reader)?)
        } else {
            None
        };

        let name_table_offset = reader.read_u64::<LittleEndian>()?;
        let start_pos = reader.position();

        reader.seek(SeekFrom::Start(name_table_offset))?;

        let name_table_size = reader.read_u32::<LittleEndian>()?;
        let mut name_table = Vec::with_capacity(name_table_size as usize);

        for _ in 0..name_table_size {
            name_table.push(reader.read_fstring()?);
        }

        reader.seek(SeekFrom::Start(start_pos))?;

        let version = reader.read_u32::<LittleEndian>()?;
        let object_index_offset = reader.read_u64::<LittleEndian>()?;

        let start_pos = reader.position();

        reader.seek(SeekFrom::Start(object_index_offset))?;

        let object_count = reader.read_u32::<LittleEndian>()?;
        let object_index = Vec::with_capacity(object_count as usize);

        let mut sav_data = SaveGameArchiveContent {
            package_version,
            save_game_class_path,
            name_table: NameTable { list: name_table },
            object_index,
            version,
        };

        for i in 0..object_count {
            let object = UObject::read(reader, &sav_data, i)?;

            sav_data.object_index.push(object);
        }

        reader.seek(SeekFrom::Start(start_pos))?;

        for _ in 0..object_count {
            let object_id = reader.read_u32::<LittleEndian>()?;
            let object = &sav_data.object_index[object_id as usize];

            let properties = object.read_data(reader, &sav_data, object_id)?;

            let is_actor = reader.read_u8()? != 0;
            let components = if is_actor {
                Some(object.read_components(reader, &sav_data)?)
            } else {
                None
            };

            let object = &mut sav_data.object_index[object_id as usize];

            object.properties = properties;
            object.components = components;
        }

        Ok(sav_data)
    }

    pub fn read_name(
        &self,
        reader: &mut Reader,
    ) -> anyhow::Result<FName> {
        self.name_table.read_name(reader)
    }
}

impl UObject {
    pub fn read(
        reader: &mut Reader,
        sav_data: &SaveGameArchiveContent,
        object_id: u32,
    ) -> anyhow::Result<UObject> {
        let was_loaded = reader.read_u8()? != 0;
        let object_path = if was_loaded && object_id == 0 {
            if let Some(path) = &sav_data.save_game_class_path {
                path.path.clone()
            } else {
                reader.read_fstring()?
            }
        } else {
            reader.read_fstring()?
        };

        let loaded_data = if !was_loaded {
            let object_name = sav_data.read_name(reader)?;
            let outer_id = reader.read_u32::<LittleEndian>()?;

            Some(UObjectLoadedData {
                name: object_name,
                outer_id,
            })
        } else {
            None
        };

        Ok(UObject {
            object_id,
            was_loaded,
            object_path,
            loaded_data,
            properties: Vec::new(),
            components: None,
        })
    }

    pub fn read_data(
        &self,
        reader: &mut Reader,
        sav_data: &SaveGameArchiveContent,
        id: u32,
    ) -> anyhow::Result<Vec<Property>> {
        let object_length = reader.read_u32::<LittleEndian>()?;

        let start_pos = reader.position();
        let properties = if object_length > 0 {
            let properties = Property::read_multiple(reader, sav_data)?;

            if reader.object_padding == 8 && id == 0 {
                assert_eq!(reader.read_u64::<LittleEndian>()?, 0);
            } else {
                assert_eq!(reader.read_u32::<LittleEndian>()?, 0);
            }

            properties
        } else {
            Vec::new()
        };

        if reader.position() - start_pos != object_length as u64 {
            // TODO: There are some bytes that are not read, but I don't know what they are yet

            println!(
                "[WARN] Object {} has {} bytes, but only {} bytes were read",
                self.object_id,
                object_length,
                reader.position() - start_pos,
            );

            reader.seek(SeekFrom::Start(start_pos + object_length as u64))?;
        }

        Ok(properties)
    }

    pub fn read_components(
        &self,
        reader: &mut Reader,
        sav_data: &SaveGameArchiveContent,
    ) -> anyhow::Result<Vec<Component>> {
        let component_count = reader.read_u32::<LittleEndian>()?;
        let mut components = Vec::with_capacity(component_count as usize);

        for _ in 0..component_count {
            let component_key = reader.read_fstring()?;
            let object_length = reader.read_u32::<LittleEndian>()?;

            let start_pos = reader.position();

            let component = ComponentType::read(reader, sav_data, &component_key)?;

            if reader.position() - start_pos != object_length as u64 {
                bail!(
                    "Component {} has {} bytes, but only {} bytes were read",
                    component_key,
                    object_length,
                    reader.position() - start_pos
                );
            }

            components.push(Component {
                component_key,
                component_type: component,
            });
        }

        Ok(components)
    }
}

impl SaveGameArchive {
    pub fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let header = SaveGameArchiveHeader::read(reader)?;
        let content = SaveGameArchiveContent::read(reader, true, true)?;

        Ok(SaveGameArchive { header, content })
    }

    pub fn write(&self, writer: &mut Writer) -> anyhow::Result<()> {
        self.header.write(writer)?;
        self.content.write(writer)?;

        Ok(())
    }
}
