use std::io::{Seek, SeekFrom};
use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use crate::io::{Reader, ReaderExt};
use crate::properties::Property;

#[derive(Debug, Serialize, Deserialize)]
pub struct FTopLevelAssetPath {
    pub path: String,
    pub name: String,
}

impl FTopLevelAssetPath {
    pub fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let path = reader.read_fstring()?;
        let name = reader.read_fstring()?;

        Ok(FTopLevelAssetPath {
            path,
            name,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FPackageVersion {
    pub ue4_version: u32,
    pub ue5_version: u32,
}

impl FPackageVersion {
    fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let ue4_version = reader.read_u32::<LittleEndian>()?;
        let ue5_version = reader.read_u32::<LittleEndian>()?;

        Ok(FPackageVersion {
            ue4_version,
            ue5_version,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveGameArchiveHeader {
    pub crc32: u32,
    pub size: u32,
    pub save_game_file_version: u32,
    pub build_number: u32,
}

impl SaveGameArchiveHeader {
    fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let crc32 = reader.read_u32::<LittleEndian>()?;
        let size = reader.read_u32::<LittleEndian>()?;
        let save_game_file_version = reader.read_u32::<LittleEndian>()?;
        let build_number = reader.read_u32::<LittleEndian>()?;

        Ok(SaveGameArchiveHeader {
            crc32,
            size,
            save_game_file_version,
            build_number,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveGameArchive {
    pub header: SaveGameArchiveHeader,
    pub content: SaveGameArchiveContent,
}

impl SaveGameArchive {
    pub fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let header = SaveGameArchiveHeader::read(reader)?;
        let content = SaveGameArchiveContent::read(reader, true, true)?;

        Ok(SaveGameArchive {
            header,
            content,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SaveGameArchiveContent {
    pub package_version: Option<FPackageVersion>,
    pub save_game_class_path: Option<FTopLevelAssetPath>,
    pub name_table_offset: u64,
    pub name_table: Vec<String>,
    pub object_index_offset: u64,
    pub object_index: Vec<UObject>,
    pub version: u32,
}

impl SaveGameArchiveContent {
    pub fn read(reader: &mut Reader, has_ue_version: bool, has_top_level_asset_path: bool) -> anyhow::Result<Self> {
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
            name_table_offset,
            name_table,
            object_index_offset,
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

            let properties = object.read_data(reader, &sav_data)?;

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
}

impl SaveGameArchiveContent {
    pub fn read_name(&self, reader: &mut Reader) -> anyhow::Result<FName> {
        const HAS_NUMBER: u16 = 1 << 15;

        let mut index = reader.read_u16::<LittleEndian>()?;

        let number = if index & HAS_NUMBER != 0 {
            index &= !HAS_NUMBER;
            Some(reader.read_u32::<LittleEndian>()?)
        } else {
            None
        };

        let empty = "".to_owned();
        let name = self.name_table.get(index as usize).unwrap_or(&empty).clone();

        Ok(FName {
            value: name,
            number,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UObject {
    pub object_id: u32,
    pub was_loaded: bool,
    pub object_path: String,
    pub loaded_data: Option<UObjectLoadedData>,
    pub properties: Vec<Property>, // initialized later
    pub components: Option<Vec<Component>>, // Some if is actor
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UObjectLoadedData {
    pub unk0: u32,
    pub name: FName,
    pub outer_id: u32,
}

impl UObject {
    fn read(reader: &mut Reader, sav_data: &SaveGameArchiveContent, object_id: u32) -> anyhow::Result<UObject> {
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
            let unk0 = reader.read_u32::<LittleEndian>()?;
            let object_name = sav_data.read_name(reader)?;
            let outer_id = reader.read_u32::<LittleEndian>()?;

            Some(
                UObjectLoadedData {
                    unk0,
                    name: object_name,
                    outer_id,
                }
            )
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

    fn read_data(&self, reader: &mut Reader, sav_data: &SaveGameArchiveContent) -> anyhow::Result<Vec<Property>> {
        let object_length = reader.read_u32::<LittleEndian>()?;

        let start_pos = reader.position();
        let properties = if object_length > 0 {
            Property::read_multiple(reader, sav_data)?
        } else {
            Vec::new()
        };

        if reader.position() - start_pos != object_length as u64 {
            reader.seek(SeekFrom::Start(start_pos + object_length as u64))?;
        }

        Ok(properties)
    }

    fn read_components(&self, reader: &mut Reader, sav_data: &SaveGameArchiveContent) -> anyhow::Result<Vec<Component>> {
        let component_count = reader.read_u32::<LittleEndian>()?;
        let mut components = Vec::with_capacity(component_count as usize);

        for _ in 0..component_count {
            let component_key = reader.read_fstring()?;
            let object_length = reader.read_u32::<LittleEndian>()?;

            let start_pos = reader.position();
            let properties = Property::read_multiple(reader, &sav_data)?;

            if reader.position() - start_pos != object_length as u64 {
                reader.seek(SeekFrom::Start(start_pos + object_length as u64))?;
            }

            components.push(Component {
                component_key,
                properties,
            });
        }

        Ok(components)
    }

}

#[derive(Debug, Serialize, Deserialize)]
pub struct Component {
    pub component_key: String,
    pub properties: Vec<Property>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FName {
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none", default = "Option::default")]
    pub number: Option<u32>,
}

impl FName {
    pub fn new(name: &str) -> FName {
        FName {
            value: name.to_owned(),
            number: None,
        }
    }

    pub fn none() -> FName {
        FName::new("None")
    }
}
