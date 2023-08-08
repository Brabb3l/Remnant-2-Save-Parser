use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use crate::io::{Reader, ReaderExt};
use crate::sav_data::{FName, FTopLevelAssetPath, SaveGameArchiveContent};
use crate::structs::{FGuid, FTransform, FVector};

const REMNANT_SAVE_GAME_PROFILE: &str = "/Game/_Core/Blueprints/Base/BP_RemnantSaveGameProfile";
const REMNANT_SAVE_GAME: &str = "/Game/_Core/Blueprints/Base/BP_RemnantSaveGame";

pub trait PropertyReader {
    fn read(&mut self, reader: &mut Reader, save_archive: &SaveGameArchiveContent, size: u32) -> anyhow::Result<PropertyData>;
    fn read_head(&mut self, reader: &mut Reader, save_archive: &SaveGameArchiveContent) -> anyhow::Result<()>;
    fn read_raw(&mut self, reader: &mut Reader, save_archive: &SaveGameArchiveContent) -> anyhow::Result<PropertyData>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Property {
    pub name: FName,
    pub index: u32,
    pub type_name: FName,
    pub size: u32,
    pub data: PropertyData,
}

impl Property {
    fn read(reader: &mut Reader, save_archive: &SaveGameArchiveContent) -> anyhow::Result<Option<Self>> {
        let name = save_archive.read_name(reader)?;

        if name.value == "None" {
            return Ok(None);
        }

        let type_name = save_archive.read_name(reader)?;
        let size = reader.read_u32::<LittleEndian>()?;
        let index = reader.read_u32::<LittleEndian>()?;

        let mut property_parser = PropertyParser::from_name(reader, &type_name.value, false)?;
        let data = property_parser.read(reader, save_archive, size)?;

        let property = Property {
            name,
            index,
            type_name,
            size,
            data,
        };

        Ok(Some(property))
    }

    pub fn read_multiple(reader: &mut Reader, save_archive: &SaveGameArchiveContent) -> anyhow::Result<Vec<Property>> {
        let mut properties = Vec::new();

        loop {
            let property = Property::read(reader, save_archive)?;

            if let Some(property) = property {
                properties.push(property);
            } else {
                break;
            }
        }

        Ok(properties)
    }
}


#[derive(Debug, Serialize, Deserialize)]
pub enum PropertyData {
    Byte {
        enum_name: FName,
        value: BytePropertyValue,
    },
    Bool(bool),
    Enum {
        enum_name: FName,
        value: FName,
    },
    Int16(i16),
    Int32(i32),
    Int64(i64),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    Float(f32),
    Double(f64),
    Map {
        key_type: FName,
        value_type: FName,
        elements: Vec<(PropertyData, PropertyData)>,
    },
    Array {
        inner_type: FName,
        elements: Vec<PropertyData>,
    },
    Object {
        class_name: Option<String>,
    },
    SoftObject {
        class_name: String,
    },
    Name(FName),
    Struct {
        struct_name: FName,
        guid: FGuid,
        data: StructData,
    },
    Str(String),
    StructReference { // for map keys
        guid: FGuid,
    },
    Text(u32, TextPropertyData),
}

pub struct PropertyParser;

impl PropertyParser {
    pub fn from_name(reader: &Reader, name: &str, alt: bool) -> anyhow::Result<Box<dyn PropertyReader>> {
        let parser: Box<dyn PropertyReader> = match name {
            "ByteProperty" => Box::new(BytePropertyParser),
            "BoolProperty" => Box::new(BoolPropertyParser),
            "EnumProperty" => Box::new(EnumPropertyParser),
            "Int16Property" => Box::new(Int16PropertyParser),
            "IntProperty" => Box::new(IntPropertyParser),
            "Int64Property" => Box::new(Int64PropertyParser),
            "UInt16Property" => Box::new(UInt16PropertyParser),
            "UInt32Property" => Box::new(UInt32PropertyParser),
            "UInt64Property" => Box::new(UInt64PropertyParser),
            "FloatProperty" => Box::new(FloatPropertyParser),
            "DoubleProperty" => Box::new(DoublePropertyParser),
            "MapProperty" => Box::new(MapPropertyParser),
            "ArrayProperty" => Box::new(ArrayPropertyParser),
            "NameProperty" => Box::new(NamePropertyParser),
            "ObjectProperty" => Box::new(ObjectPropertyParser),
            "SoftObjectProperty" => Box::new(SoftObjectPropertyParser),
            "StructProperty" => if alt {
                Box::new(MapStructPropertyParser)
            } else {
                Box::new(StructPropertyParser {
                    size: 0,
                    struct_name: FName::none(),
                    guid: FGuid::new(),
                })
            },
            "StrProperty" => Box::new(StrPropertyParser),
            "TextProperty" => Box::new(TextPropertyParser),
            _ => bail!("Unknown property type: {} at {}", name, reader.position()),
        };

        Ok(parser)
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum BytePropertyValue {
    Enum(FName),
    Byte(u8),
}

pub struct BytePropertyParser;

impl PropertyReader for BytePropertyParser {
    fn read(&mut self, reader: &mut Reader, save_archive: &SaveGameArchiveContent, _size: u32) -> anyhow::Result<PropertyData> {
        let enum_name = save_archive.read_name(reader)?;

        reader.read_u8()?;

        let value = if enum_name.value == "None" {
            BytePropertyValue::Byte(reader.read_u8()?)
        } else {
            BytePropertyValue::Enum(save_archive.read_name(reader)?)
        };

        Ok(PropertyData::Byte {
            enum_name,
            value,
        })
    }

    fn read_head(&mut self, _reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<PropertyData> {
        let value = reader.read_u8()?;

        Ok(PropertyData::Byte {
            enum_name: FName::none(),
            value: BytePropertyValue::Byte(value),
        })
    }
}

pub struct BoolPropertyParser;

impl PropertyReader for BoolPropertyParser {
    fn read(&mut self, reader: &mut Reader, save_archive: &SaveGameArchiveContent, _size: u32) -> anyhow::Result<PropertyData> {
        let value = Self::read_raw(self, reader, save_archive)?;

        reader.read_u8()?;

        Ok(value)
    }

    fn read_head(&mut self, _reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<PropertyData> {
        let value = reader.read_u8()?;

        Ok(PropertyData::Bool(value != 0))
    }

}

pub struct EnumPropertyParser;

impl PropertyReader for EnumPropertyParser {
    fn read(&mut self, reader: &mut Reader, save_archive: &SaveGameArchiveContent, _size: u32) -> anyhow::Result<PropertyData> {
        let enum_name = save_archive.read_name(reader)?;

        reader.read_u8()?;

        let value = save_archive.read_name(reader)?;

        Ok(PropertyData::Enum {
            enum_name,
            value,
        })
    }

    fn read_head(&mut self, _reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, _reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<PropertyData> {
        todo!("EnumPropertyParser::read_raw")
    }
}

macro_rules! impl_primitive_parser {
    (
        $name:ident, $read_method:ident, $prop_data_name:ident
    ) => {
        pub struct $name;

        impl PropertyReader for $name {
            fn read(&mut self, reader: &mut Reader, save_archive: &SaveGameArchiveContent, _size: u32) -> anyhow::Result<PropertyData> {
                reader.read_u8()?;

                let value = Self::read_raw(self, reader, save_archive)?;

                Ok(value)
            }

            fn read_head(&mut self, _reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<()> {
                Ok(())
            }

            fn read_raw(&mut self, reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<PropertyData> {
                let value = reader.$read_method::<LittleEndian>()?;

                Ok(PropertyData::$prop_data_name(value))
            }
        }
    };
}

impl_primitive_parser!(Int16PropertyParser, read_i16, Int16);
impl_primitive_parser!(IntPropertyParser, read_i32, Int32);
impl_primitive_parser!(Int64PropertyParser, read_i64, Int64);
impl_primitive_parser!(UInt16PropertyParser, read_u16, UInt16);
impl_primitive_parser!(UInt32PropertyParser, read_u32, UInt32);
impl_primitive_parser!(UInt64PropertyParser, read_u64, UInt64);
impl_primitive_parser!(FloatPropertyParser, read_f32, Float);
impl_primitive_parser!(DoublePropertyParser, read_f64, Double);

pub struct MapPropertyParser;

impl PropertyReader for MapPropertyParser {
    fn read(&mut self, reader: &mut Reader, save_archive: &SaveGameArchiveContent, _size: u32) -> anyhow::Result<PropertyData> {
        let key_type = save_archive.read_name(reader)?;
        let value_type = save_archive.read_name(reader)?;

        let mut key_parser = PropertyParser::from_name(reader, key_type.value.as_str(), true)?;
        let mut value_parser = PropertyParser::from_name(reader, value_type.value.as_str(), false)?;

        reader.read_u8()?;
        reader.read_u32::<LittleEndian>()?;

        let element_count = reader.read_u32::<LittleEndian>()?;
        let mut elements = Vec::with_capacity(element_count as usize);

        for _ in 0..element_count {
            let key = key_parser.read_raw(reader, save_archive)?;
            let value = value_parser.read_raw(reader, save_archive)?;

            elements.push((key, value));
        }

        Ok(PropertyData::Map {
            key_type,
            value_type,
            elements,
        })
    }

    fn read_head(&mut self, _reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, _reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<PropertyData> {
        todo!("MapPropertyParser::read_raw")
    }
}

struct ArrayPropertyParser;

impl PropertyReader for ArrayPropertyParser {
    fn read(&mut self, reader: &mut Reader, save_archive: &SaveGameArchiveContent, _size: u32) -> anyhow::Result<PropertyData> {
        let inner_type = save_archive.read_name(reader)?;
        let mut inner_parser = PropertyParser::from_name(reader, inner_type.value.as_str(), false)?;

        reader.read_u8()?;

        let element_count = reader.read_u32::<LittleEndian>()?;
        let mut elements = Vec::with_capacity(element_count as usize);

         inner_parser.read_head(reader, save_archive)?;

        for _ in 0..element_count {
            let value = inner_parser.read_raw(reader, save_archive)?;

            elements.push(value);
        }

        Ok(PropertyData::Array {
            inner_type,
            elements,
        })
    }

    fn read_head(&mut self, _reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, _reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<PropertyData> {
        todo!("ArrayPropertyParser::read_raw")
    }
}

struct ObjectPropertyParser;

impl PropertyReader for ObjectPropertyParser {
    fn read(&mut self, reader: &mut Reader, save_archive: &SaveGameArchiveContent, _size: u32) -> anyhow::Result<PropertyData> {
        reader.read_u8()?;

        let value = Self::read_raw(self, reader, save_archive)?;

        Ok(value)
    }

    fn read_head(&mut self, _reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, reader: &mut Reader, save_archive: &SaveGameArchiveContent) -> anyhow::Result<PropertyData> {
        let class_name_index = reader.read_i32::<LittleEndian>()?;
        let class_name = if class_name_index == -1 {
            None
        } else {
            Some(save_archive.object_index[class_name_index as usize].object_path.clone())
        };

        Ok(PropertyData::Object {
            class_name,
        })
    }
}

struct SoftObjectPropertyParser;

impl PropertyReader for SoftObjectPropertyParser {
    fn read(&mut self, reader: &mut Reader, _save_archive: &SaveGameArchiveContent, _size: u32) -> anyhow::Result<PropertyData> {
        reader.read_u8()?;

        let class_name = reader.read_fstring()?;

        Ok(PropertyData::SoftObject {
            class_name,
        })
    }

    fn read_head(&mut self, _reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, _reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<PropertyData> {
        todo!("SoftObjectPropertyParser::read_raw")
    }
}

pub struct NamePropertyParser;

impl PropertyReader for NamePropertyParser {
    fn read(&mut self, reader: &mut Reader, save_archive: &SaveGameArchiveContent, _size: u32) -> anyhow::Result<PropertyData> {
        reader.read_u8()?;

        let value = Self::read_raw(self, reader, save_archive)?;

        Ok(value)
    }

    fn read_head(&mut self, _reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, reader: &mut Reader, save_archive: &SaveGameArchiveContent) -> anyhow::Result<PropertyData> {
        let value = save_archive.read_name(reader)?;

        Ok(PropertyData::Name(value))
    }
}

struct StructPropertyParser {
    size: u32,
    struct_name: FName,
    guid: FGuid,
}

impl PropertyReader for StructPropertyParser {
    fn read(&mut self, reader: &mut Reader, save_archive: &SaveGameArchiveContent, size: u32) -> anyhow::Result<PropertyData> {
        self.struct_name = save_archive.read_name(reader)?;
        self.guid = FGuid::read(reader)?;
        self.size = size;

        reader.read_u8()?;

        let data = self.read_struct_data(reader, save_archive, self.size)?;

        Ok(PropertyData::Struct {
            struct_name: self.struct_name.clone(),
            guid: self.guid.clone(),
            data,
        })
    }

    fn read_head(&mut self, reader: &mut Reader, save_archive: &SaveGameArchiveContent) -> anyhow::Result<()> {
        let _name = save_archive.read_name(reader)?;
        let _type_name = save_archive.read_name(reader)?;
        self.size = reader.read_u32::<LittleEndian>()?;
        let _index = reader.read_u32::<LittleEndian>()?;
        self.struct_name = save_archive.read_name(reader)?;
        self.guid = FGuid::read(reader)?;

        reader.read_u8()?;

        Ok(())
    }

    fn read_raw(&mut self, reader: &mut Reader, save_archive: &SaveGameArchiveContent) -> anyhow::Result<PropertyData> {
        let data = self.read_struct_data(reader, save_archive, self.size)?;

        Ok(PropertyData::Struct {
            struct_name: self.struct_name.clone(),
            guid: self.guid.clone(),
            data,
        })
    }
}

impl StructPropertyParser {
    fn read_struct_data(&self, reader: &mut Reader, save_archive: &SaveGameArchiveContent, _size: u32) -> anyhow::Result<StructData> {
        let data = match self.struct_name.value.as_str() {
            "SoftClassPath" => {
                let value = reader.read_fstring()?;

                StructData::SoftClassPath {
                    value,
                }
            }
            "SoftObjectPath" => {
                let value = reader.read_fstring()?;

                StructData::SoftObjectPath {
                    value,
                }
            }
            "PersistenceBlob" => {
                let size = reader.read_u32::<LittleEndian>()?;
                let mut data = vec![0; size as usize];

                reader.read_exact(&mut data)?;

                let reader = &mut Reader::new(data);

                if let Some(save_game_class_path) = &save_archive.save_game_class_path {
                    match save_game_class_path.path.as_str() {
                        REMNANT_SAVE_GAME_PROFILE => {
                            let archive = SaveGameArchiveContent::read(reader, true, false)?;

                            StructData::PersistenceBlob {
                                archive,
                            }
                        }
                        REMNANT_SAVE_GAME => {
                            let version = reader.read_u32::<LittleEndian>()?;
                            let index_offset = reader.read_u32::<LittleEndian>()?;
                            let dynamic_offset = reader.read_u32::<LittleEndian>()?;

                            reader.seek(SeekFrom::Start(index_offset as u64))?;

                            let info_count = reader.read_u32::<LittleEndian>()?;
                            let mut actor_info = Vec::with_capacity(info_count as usize);

                            for _ in 0..info_count {
                                let info = FInfo::read(reader)?;

                                actor_info.push(info);
                            }

                            let destroyed_count = reader.read_u32::<LittleEndian>()?;
                            let mut destroyed = Vec::with_capacity(destroyed_count as usize);

                            for _ in 0..destroyed_count {
                                let unique_id = reader.read_u64::<LittleEndian>()?;

                                destroyed.push(unique_id);
                            }

                            let mut actors = HashMap::with_capacity(info_count as usize);

                            for info in actor_info {
                                let mut bytes = vec![0; info.size as usize];

                                reader.seek(SeekFrom::Start(info.offset as u64))?;
                                reader.read_exact(&mut bytes)?;

                                let mut sub_reader = Reader::new(bytes);
                                let actor = Actor::read(&mut sub_reader)?;

                                actors.insert(
                                    info.unique_id,
                                    actor,
                                );
                            }

                            reader.seek(SeekFrom::Start(dynamic_offset as u64))?;

                            let dynamic_actor_count = reader.read_u32::<LittleEndian>()?;

                            for _ in 0..dynamic_actor_count {
                                let dynamic_actor = DynamicActor::read(reader)?;
                                let actor = actors.get_mut(&dynamic_actor.unique_id).unwrap();

                                actor.dynamic_data = Some(dynamic_actor);
                            }

                            StructData::PersistenceContainer {
                                version,
                                destroyed,
                                actors,
                            }
                        }
                        _ => {
                            bail!("Unknown SaveGameClassPath: {}", save_game_class_path.path);
                        }
                    }
                } else {
                    bail!("SaveGameClassPath not found");
                }
            }
            "Guid" => {
                let value = FGuid::read(reader)?;

                StructData::Guid {
                    value,
                }
            }
            "Timespan" => {
                let value = reader.read_u64::<LittleEndian>()?;

                StructData::Timespan {
                    value,
                }
            }
            "DateTime" => {
                let value = reader.read_u64::<LittleEndian>()?;

                StructData::DateTime {
                    value,
                }
            }
            "Vector" => {
                StructData::Vector(
                    FVector::read(reader)?
                )
            }
            _ => {
                let properties = Property::read_multiple(reader, save_archive)?;

                StructData::Dynamic {
                    properties,
                }
            }
        };

        Ok(data)
    }
}

#[derive(Debug)]
struct FInfo {
    unique_id: u64,
    offset: u32,
    size: u32,
}

impl FInfo {
    fn read(reader: &mut Reader) -> anyhow::Result<FInfo> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let offset = reader.read_u32::<LittleEndian>()?;
        let size = reader.read_u32::<LittleEndian>()?;

        Ok(FInfo {
            unique_id,
            offset,
            size,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DynamicActor {
    pub unique_id: u64,
    pub transform: FTransform,
    pub class_path: FTopLevelAssetPath,
}

impl DynamicActor {
    pub fn read(reader: &mut Reader) -> anyhow::Result<DynamicActor> {
        let unique_id = reader.read_u64::<LittleEndian>()?;
        let transform = FTransform::read(reader)?;
        let class_path = FTopLevelAssetPath::read(reader)?;

        Ok(DynamicActor {
            unique_id,
            transform,
            class_path,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Actor {
    pub transform: Option<FTransform>,
    pub archive: SaveGameArchiveContent,
    pub dynamic_data: Option<DynamicActor>,
}

impl Actor {
    pub fn read(reader: &mut Reader) -> anyhow::Result<Actor> {
        let has_transform = reader.read_u32::<LittleEndian>()?;
        let transform = if has_transform != 0 {
            let transform = FTransform::read(reader)?;

            Some(transform)
        } else {
            None
        };

        let archive = SaveGameArchiveContent::read(reader, false, false)?;

        Ok(Actor {
            transform,
            archive,
            dynamic_data: None,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum StructData {
    SoftClassPath {
        value: String,
    },
    SoftObjectPath {
        value: String,
    },
    PersistenceBlob {
        archive: SaveGameArchiveContent,
    },
    PersistenceContainer {
        version: u32,
        destroyed: Vec<u64>,
        actors: HashMap<u64, Actor>,
    },
    Guid {
        value: FGuid,
    },
    Timespan {
        value: u64,
    },
    DateTime {
        value: u64,
    },
    Vector(FVector),
    Dynamic {
        properties: Vec<Property>,
    },
    Raw { // Only used for debugging
        data: Vec<u8>,
    }
}

struct StrPropertyParser;

impl PropertyReader for StrPropertyParser {
    fn read(&mut self, reader: &mut Reader, save_archive: &SaveGameArchiveContent, _size: u32) -> anyhow::Result<PropertyData> {
        reader.read_u8()?;

        let value = Self::read_raw(self, reader, save_archive)?;

        Ok(value)
    }

    fn read_head(&mut self, _reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<PropertyData> {
        let value = reader.read_fstring()?;

        Ok(PropertyData::Str(value))
    }
}

struct MapStructPropertyParser;

impl PropertyReader for MapStructPropertyParser {
    fn read(&mut self, _reader: &mut Reader, _save_archive: &SaveGameArchiveContent, _size: u32) -> anyhow::Result<PropertyData> {
        panic!("Unsupported operation");
    }

    fn read_head(&mut self, _reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<PropertyData> {
        let value = FGuid::read(reader)?;

        Ok(PropertyData::StructReference{
            guid: value,
        })
    }
}

struct TextPropertyParser;

#[derive(Debug, Serialize, Deserialize)]
pub enum TextPropertyData {
    Base {
        namespace: String,
        key: String,
        source_string: String,
    },
    None {
        culture_invariant_string: Option<String>,
    }
}

impl PropertyReader for TextPropertyParser {
    fn read(&mut self, reader: &mut Reader, _save_archive: &SaveGameArchiveContent, _size: u32) -> anyhow::Result<PropertyData> {
        reader.read_u8()?;

        let value = self.read_raw(reader, _save_archive)?;

        Ok(value)
    }

    fn read_head(&mut self, _reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, reader: &mut Reader, _save_archive: &SaveGameArchiveContent) -> anyhow::Result<PropertyData> {
        let flags = reader.read_u32::<LittleEndian>()?;
        let history_type = reader.read_u8()?;

        let data = match history_type {
            0 => { // Base
                let namespace = reader.read_fstring()?;
                let key = reader.read_fstring()?;
                let source_string = reader.read_fstring()?;

                TextPropertyData::Base {
                    namespace,
                    key,
                    source_string,
                }
            }
            255 => { // None
                let has_culture_invariant_string = reader.read_u32::<LittleEndian>()? != 0;

                let culture_invariant_string = if has_culture_invariant_string {
                    Some(reader.read_fstring()?)
                } else {
                    None
                };

                TextPropertyData::None {
                    culture_invariant_string,
                }
            }
            _ => bail!("Unsupported history type: {}", history_type),
        };

        Ok(PropertyData::Text(
            flags,
            data,
        ))
    }
}

