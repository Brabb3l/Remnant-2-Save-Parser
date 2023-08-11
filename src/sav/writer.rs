use std::cmp::min;
use std::io::{Cursor, Read, Seek, SeekFrom, Write};
use byteorder::{LittleEndian, WriteBytesExt};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use crate::io::{Writer, WriterExt};
use crate::properties::Property;
use crate::sav::{ARCHIVE_V2_HEADER_TAG, Compressor, FCompressedChunkInfo, NameTable, SaveGameArchive, SaveGameArchiveContent, SaveGameArchiveHeader, SavFile, UObject};
use crate::structs::FName;

impl Compressor {
    fn write(&self, writer: &mut Writer) -> anyhow::Result<()> {
        match self {
            Compressor::Custom(name) => {
                writer.write_u8(0)?;
                writer.write_fstring(name.clone())?;
            }
            Compressor::None => {
                writer.write_u8(1)?;
            }
            Compressor::Oodle => {
                writer.write_u8(2)?;
            }
            Compressor::Zlib => {
                writer.write_u8(3)?;
            }
            Compressor::Gzip => {
                writer.write_u8(4)?;
            }
            Compressor::LZ4 => {
                writer.write_u8(5)?;
            }
        }

        Ok(())
    }
}

impl SavFile {
    #[allow(dead_code)] // TODO: Fix write
    pub fn write(
        mut writer: &mut Writer,
        archive: &SaveGameArchive
    ) -> anyhow::Result<()> {
        let mut archive_writer = Writer::new(Vec::new(), 4);

        // write archive

        let start_pos = archive_writer.position();

        archive.write(&mut archive_writer)?;

        let size = archive_writer.position() - start_pos - 8;
        let size_with_header = size + 8;

        // update archive header with correct values

        archive_writer.seek(SeekFrom::Start(4))?;
        archive_writer.write_u32::<LittleEndian>(size_with_header as u32)?;

        // calculate crc32 from position 4 to end of file

        let mut crc32 = crc32fast::Hasher::new();

        crc32.update(&archive_writer.get_ref()[4..]);

        let crc32 = crc32.finalize();

        // fix up the archive header to only contain the size at 0x08

        archive_writer.seek(SeekFrom::Start(8))?;
        archive_writer.write_u32::<LittleEndian>(size as u32 - 4)?;

        // append sav file header

        writer.write_u32::<LittleEndian>(crc32)?;
        writer.write_u32::<LittleEndian>(size_with_header as u32)?;
        writer.write_u32::<LittleEndian>(9)?; // todo

        let mut buf = vec![0u8; 2 << 16];
        let mut to_write = size;

        let mut archive_writer = Cursor::new(archive_writer.into_inner());

        archive_writer.seek(SeekFrom::Start(8))?;

        while to_write > 0 {
            let chunk_size = min(to_write, buf.len() as u64);

            writer.write_u64::<LittleEndian>(ARCHIVE_V2_HEADER_TAG)?;
            writer.write_u64::<LittleEndian>(2 << 16)?;
            Compressor::Zlib.write(writer)?;

            let mut compression_info = FCompressedChunkInfo {
                compressed_size: 0, // placeholder
                uncompressed_size: chunk_size,
            };

            let header_start_pos = writer.position();

            compression_info.write(writer)?;
            compression_info.write(writer)?;

            let start_pos = writer.position();
            let mut encoder = ZlibEncoder::new(writer, Compression::default());

            archive_writer.read_exact(&mut buf[..chunk_size as usize])?;
            encoder.write_all(&buf[..chunk_size as usize])?;

            writer = encoder.finish()?;

            let compressed_size = writer.position() - start_pos;
            let current_pos = writer.position();

            writer.seek(SeekFrom::Start(header_start_pos))?;

            compression_info.compressed_size = compressed_size;

            compression_info.write(writer)?;
            compression_info.write(writer)?;

            writer.seek(SeekFrom::Start(current_pos))?;

            to_write -= chunk_size;
        }

        Ok(())
    }
}

impl FCompressedChunkInfo {
    fn write(&self, writer: &mut Writer) -> anyhow::Result<()> {
        writer.write_u64::<LittleEndian>(self.compressed_size)?;
        writer.write_u64::<LittleEndian>(self.uncompressed_size)?;

        Ok(())
    }
}

impl SaveGameArchiveHeader {
    pub fn write(&self, writer: &mut Writer) -> anyhow::Result<()> {
        writer.write_u32::<LittleEndian>(0)?;
        writer.write_u32::<LittleEndian>(0)?;
        writer.write_u32::<LittleEndian>(self.save_game_file_version)?;
        writer.write_u32::<LittleEndian>(self.build_number)?;

        Ok(())
    }
}

impl NameTable {
    pub fn write_name(&mut self, writer: &mut Writer, name: &FName) -> anyhow::Result<()> {
        const HAS_NUMBER: u16 = 1 << 15;

        // check if name is already in table (insert if not)
        let index = self.list.iter().position(|n| *n == name.value);
        let mut index = if let Some(index) = index {
            index as u16
        } else {
            self.list.push(name.value.clone());
            (self.list.len() - 1) as u16
        };

        if name.number.is_some() {
            index |= HAS_NUMBER;
        }

        writer.write_u16::<LittleEndian>(index)?;

        if let Some(number) = name.number {
            writer.write_u32::<LittleEndian>(number)?;
        }

        Ok(())
    }
}

impl SaveGameArchiveContent {
    pub fn write(
        &self,
        writer: &mut Writer,
    ) -> anyhow::Result<()> {
        if let Some(package_version) = &self.package_version {
            package_version.write(writer)?;
        }

        if let Some(save_game_class_path) = &self.save_game_class_path {
            save_game_class_path.write(writer)?;
        }

        let name_table_offset = writer.position();
        writer.write_u64::<LittleEndian>(0)?; // placeholder

        writer.write_u32::<LittleEndian>(self.version)?;

        let object_index_offset = writer.position();
        writer.write_u64::<LittleEndian>(0)?; // placeholder

        let mut name_table = NameTable { list: Vec::new() };

        for object in &self.object_index {
            writer.write_u32::<LittleEndian>(object.object_id)?;
            object.write_data(writer, &mut name_table)?;
        }

        let object_index_offset_start = writer.position();

        writer.seek(SeekFrom::Start(object_index_offset))?;
        writer.write_u64::<LittleEndian>(object_index_offset_start)?;
        writer.seek(SeekFrom::Start(object_index_offset_start))?;
        writer.write_u32::<LittleEndian>(self.object_index.len() as u32)?;

        for object in &self.object_index {
            object.write(writer, &mut name_table)?;
        }

        let name_table_offset_start = writer.position();

        writer.seek(SeekFrom::Start(name_table_offset))?;
        writer.write_u64::<LittleEndian>(name_table_offset_start)?;
        writer.seek(SeekFrom::Start(name_table_offset_start))?;
        writer.write_u32::<LittleEndian>(name_table.list.len() as u32)?;

        for name in &name_table.list {
            writer.write_fstring(name.clone())?;
        }

        Ok(())
    }
}

impl UObject {
    pub fn write(&self, writer: &mut Writer, name_table: &mut NameTable) -> anyhow::Result<()> {
        writer.write_u8(self.was_loaded as u8)?;
        writer.write_fstring(self.object_path.clone())?;

        if let Some(loaded_data) = &self.loaded_data {
            name_table.write_name(writer, &loaded_data.name)?;
            writer.write_u32::<LittleEndian>(loaded_data.outer_id)?;
        }

        Ok(())
    }

    pub fn write_data(&self, writer: &mut Writer, name_table: &mut NameTable) -> anyhow::Result<()> {
        let size_offset = writer.position();
        writer.write_u32::<LittleEndian>(0)?; // placeholder for size

        let start_pos = writer.position();

        if !self.properties.is_empty() {
            for property in &self.properties {
                property.write(writer, name_table)?;
            }

            Property::write_none(writer, name_table)?;

            if self.object_id == 0 && writer.object_padding == 8 {
                writer.write_u64::<LittleEndian>(0)?;
            } else {
                writer.write_u32::<LittleEndian>(0)?;
            }
        }

        let end_pos = writer.position();
        let size = end_pos - start_pos;

        writer.seek(SeekFrom::Start(size_offset))?;
        writer.write_u32::<LittleEndian>(size as u32)?;
        writer.seek(SeekFrom::Start(end_pos))?;

        self.write_components(writer, name_table)?;

        Ok(())
    }

    pub fn write_components(&self, writer: &mut Writer, name_table: &mut NameTable) -> anyhow::Result<()> {
        if let Some(components) = &self.components {
            writer.write_u8(1)?;
            writer.write_u32::<LittleEndian>(components.len() as u32)?;

            for component in components {
                writer.write_fstring(component.component_key.clone())?;

                let size_offset = writer.position();
                writer.write_u32::<LittleEndian>(0)?; // placeholder for size

                let start_pos = writer.position();

                component.component_type.write(writer, name_table)?;

                let end_pos = writer.position();
                let size = end_pos - start_pos;

                writer.seek(SeekFrom::Start(size_offset))?;
                writer.write_u32::<LittleEndian>(size as u32)?;
                writer.seek(SeekFrom::Start(end_pos))?;
            }
        } else {
            writer.write_u8(0)?;
        }

        Ok(())
    }
}

