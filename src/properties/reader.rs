use crate::io::{Reader, ReaderExt};
use crate::properties::{ArrayProperty, ByteProperty, EnumProperty, HeadData, MapProperty, Property, PropertyData, REMNANT_SAVE_GAME, REMNANT_SAVE_GAME_PROFILE, StructProperty, TextProperty};
use crate::structs::{
    DateTime, DynamicStruct, FGuid, FName, FVector, PersistenceBlob, PersistenceContainer,
    StructData, Timespan,
};
use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use std::io::Read;
use crate::sav::SaveGameArchiveContent;

pub trait PropertyReader {
    fn read(
        &mut self,
        reader: &mut Reader,
        save_archive: &SaveGameArchiveContent,
        size: u32,
    ) -> anyhow::Result<PropertyData>;

    fn read_head(
        &mut self,
        reader: &mut Reader,
        save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<HeadData>;

    fn read_raw(
        &mut self,
        reader: &mut Reader,
        save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<PropertyData>;
}

impl Property {
    fn read(
        reader: &mut Reader,
        save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<Option<Self>> {
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

    pub fn read_multiple(
        reader: &mut Reader,
        save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<Vec<Property>> {
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

pub struct PropertyParser;

impl PropertyParser {
    pub fn from_name(
        reader: &Reader,
        name: &str,
        alt: bool,
    ) -> anyhow::Result<Box<dyn PropertyReader>> {
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
            "StructProperty" => {
                if alt {
                    Box::new(MapStructPropertyParser)
                } else {
                    Box::new(StructPropertyParser {
                        size: 0,
                        struct_name: FName::none(),
                        guid: FGuid::default(),
                    })
                }
            }
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

#[derive(Debug, Serialize, Deserialize)]
pub enum TextPropertyData {
    Base {
        namespace: String,
        key: String,
        source_string: String,
    },
    None {
        culture_invariant_string: Option<String>,
    },
}

pub struct BytePropertyParser;
pub struct BoolPropertyParser;
pub struct EnumPropertyParser;
pub struct MapPropertyParser;
pub struct ArrayPropertyParser;
pub struct ObjectPropertyParser;
pub struct SoftObjectPropertyParser;
pub struct NamePropertyParser;
pub struct StructPropertyParser {
    size: u32,
    struct_name: FName,
    guid: FGuid,
}
pub struct StrPropertyParser;
pub struct TextPropertyParser;
pub struct MapStructPropertyParser;

impl PropertyReader for BytePropertyParser {
    fn read(
        &mut self,
        reader: &mut Reader,
        save_archive: &SaveGameArchiveContent,
        _size: u32,
    ) -> anyhow::Result<PropertyData> {
        let enum_name = save_archive.read_name(reader)?;

        reader.read_u8()?;

        let value = if enum_name.value == "None" {
            BytePropertyValue::Byte(reader.read_u8()?)
        } else {
            BytePropertyValue::Enum(save_archive.read_name(reader)?)
        };

        Ok(PropertyData::Byte(ByteProperty { enum_name, value }))
    }

    fn read_head(
        &mut self,
        _reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<HeadData> {
        Ok(HeadData::None)
    }

    fn read_raw(
        &mut self,
        reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<PropertyData> {
        let value = reader.read_u8()?;

        Ok(PropertyData::Byte(ByteProperty {
            enum_name: FName::none(),
            value: BytePropertyValue::Byte(value),
        }))
    }
}

impl PropertyReader for BoolPropertyParser {
    fn read(
        &mut self,
        reader: &mut Reader,
        save_archive: &SaveGameArchiveContent,
        _size: u32,
    ) -> anyhow::Result<PropertyData> {
        let value = Self::read_raw(self, reader, save_archive)?;

        reader.read_u8()?;

        Ok(value)
    }

    fn read_head(
        &mut self,
        _reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<HeadData> {
        Ok(HeadData::None)
    }

    fn read_raw(
        &mut self,
        reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<PropertyData> {
        let value = reader.read_u8()?;

        Ok(PropertyData::Bool(value != 0))
    }
}

impl PropertyReader for EnumPropertyParser {
    fn read(
        &mut self,
        reader: &mut Reader,
        save_archive: &SaveGameArchiveContent,
        _size: u32,
    ) -> anyhow::Result<PropertyData> {
        let enum_name = save_archive.read_name(reader)?;

        reader.read_u8()?;

        let value = save_archive.read_name(reader)?;

        Ok(PropertyData::Enum(EnumProperty { enum_name, value }))
    }

    fn read_head(
        &mut self,
        _reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<HeadData> {
        Ok(HeadData::None)
    }

    fn read_raw(
        &mut self,
        _reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<PropertyData> {
        todo!("EnumPropertyParser::read_raw")
    }
}

impl PropertyReader for MapPropertyParser {
    fn read(
        &mut self,
        reader: &mut Reader,
        save_archive: &SaveGameArchiveContent,
        _size: u32,
    ) -> anyhow::Result<PropertyData> {
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

        Ok(PropertyData::Map(MapProperty {
            key_type,
            value_type,
            elements,
        }))
    }

    fn read_head(
        &mut self,
        _reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<HeadData> {
        Ok(HeadData::None)
    }

    fn read_raw(
        &mut self,
        _reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<PropertyData> {
        todo!("MapPropertyParser::read_raw")
    }
}

impl PropertyReader for ArrayPropertyParser {
    fn read(
        &mut self,
        reader: &mut Reader,
        save_archive: &SaveGameArchiveContent,
        _size: u32,
    ) -> anyhow::Result<PropertyData> {
        let inner_type = save_archive.read_name(reader)?;
        let mut inner_parser = PropertyParser::from_name(reader, inner_type.value.as_str(), false)?;

        reader.read_u8()?;

        let element_count = reader.read_u32::<LittleEndian>()?;
        let mut elements = Vec::with_capacity(element_count as usize);

        let head_data = inner_parser.read_head(reader, save_archive)?;

        for _ in 0..element_count {
            let value = inner_parser.read_raw(reader, save_archive)?;

            elements.push(value);
        }

        Ok(PropertyData::Array(ArrayProperty {
            inner_type,
            head_data,
            elements,
        }))
    }

    fn read_head(
        &mut self,
        _reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<HeadData> {
        Ok(HeadData::None)
    }

    fn read_raw(
        &mut self,
        _reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<PropertyData> {
        todo!("ArrayPropertyParser::read_raw")
    }
}

impl PropertyReader for ObjectPropertyParser {
    fn read(
        &mut self,
        reader: &mut Reader,
        save_archive: &SaveGameArchiveContent,
        _size: u32,
    ) -> anyhow::Result<PropertyData> {
        reader.read_u8()?;

        let value = Self::read_raw(self, reader, save_archive)?;

        Ok(value)
    }

    fn read_head(
        &mut self,
        _reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<HeadData> {
        Ok(HeadData::None)
    }

    fn read_raw(
        &mut self,
        reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<PropertyData> {
        let class_name_index = reader.read_i32::<LittleEndian>()?;

        Ok(PropertyData::Object(class_name_index))
    }
}

impl PropertyReader for SoftObjectPropertyParser {
    fn read(
        &mut self,
        reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
        _size: u32,
    ) -> anyhow::Result<PropertyData> {
        reader.read_u8()?;

        let class_name = reader.read_fstring()?;

        Ok(PropertyData::SoftObject(class_name))
    }

    fn read_head(
        &mut self,
        _reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<HeadData> {
        Ok(HeadData::None)
    }

    fn read_raw(
        &mut self,
        _reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<PropertyData> {
        todo!("SoftObjectPropertyParser::read_raw")
    }
}

impl PropertyReader for NamePropertyParser {
    fn read(
        &mut self,
        reader: &mut Reader,
        save_archive: &SaveGameArchiveContent,
        _size: u32,
    ) -> anyhow::Result<PropertyData> {
        reader.read_u8()?;

        let value = Self::read_raw(self, reader, save_archive)?;

        Ok(value)
    }

    fn read_head(
        &mut self,
        _reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<HeadData> {
        Ok(HeadData::None)
    }

    fn read_raw(
        &mut self,
        reader: &mut Reader,
        save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<PropertyData> {
        let value = save_archive.read_name(reader)?;

        Ok(PropertyData::Name(value))
    }
}

impl PropertyReader for StructPropertyParser {
    fn read(
        &mut self,
        reader: &mut Reader,
        save_archive: &SaveGameArchiveContent,
        size: u32,
    ) -> anyhow::Result<PropertyData> {
        self.struct_name = save_archive.read_name(reader)?;
        self.guid = FGuid::read(reader)?;
        self.size = size;

        reader.read_u8()?;

        let data = self.read_struct_data(reader, save_archive, self.size)?;

        Ok(PropertyData::Struct(StructProperty {
            struct_name: self.struct_name.clone(),
            guid: self.guid.clone(),
            data,
        }))
    }

    fn read_head(
        &mut self,
        reader: &mut Reader,
        save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<HeadData> {
        let name = save_archive.read_name(reader)?;
        let type_name = save_archive.read_name(reader)?;
        self.size = reader.read_u32::<LittleEndian>()?;
        let index = reader.read_u32::<LittleEndian>()?;
        self.struct_name = save_archive.read_name(reader)?;
        self.guid = FGuid::read(reader)?;

        reader.read_u8()?;

        Ok(HeadData::Struct {
            name,
            type_name,
            index,
            struct_name: self.struct_name.clone(),
            guid: self.guid.clone(),
        })
    }

    fn read_raw(
        &mut self,
        reader: &mut Reader,
        save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<PropertyData> {
        let data = self.read_struct_data(reader, save_archive, self.size)?;

        Ok(PropertyData::Struct(StructProperty {
            struct_name: self.struct_name.clone(),
            guid: self.guid.clone(),
            data,
        }))
    }
}

impl StructPropertyParser {
    fn read_struct_data(
        &self,
        reader: &mut Reader,
        save_archive: &SaveGameArchiveContent,
        _size: u32,
    ) -> anyhow::Result<StructData> {
        let data = match self.struct_name.value.as_str() {
            "SoftClassPath" => StructData::SoftClassPath(reader.read_fstring()?),
            "SoftObjectPath" => StructData::SoftObjectPath(reader.read_fstring()?),
            "PersistenceBlob" => {
                let size = reader.read_u32::<LittleEndian>()?;
                let mut data = vec![0; size as usize];

                reader.read_exact(&mut data)?;

                let mut reader = Reader::new(data, 8);

                if let Some(save_game_class_path) = &save_archive.save_game_class_path {
                    match save_game_class_path.path.as_str() {
                        REMNANT_SAVE_GAME_PROFILE => {
                            StructData::PersistenceBlob(PersistenceBlob::read(&mut reader)?)
                        }
                        REMNANT_SAVE_GAME => {
                            StructData::PersistenceContainer(PersistenceContainer::read(&mut reader)?)
                        }
                        _ => {
                            bail!("Unknown SaveGameClassPath: {}", save_game_class_path.path);
                        }
                    }
                } else {
                    bail!("SaveGameClassPath not found");
                }
            }
            "Guid" => StructData::Guid(FGuid::read(reader)?),
            "Timespan" => StructData::Timespan(Timespan::read(reader)?),
            "DateTime" => StructData::DateTime(DateTime::read(reader)?),
            "Vector" => StructData::Vector(FVector::read(reader)?),
            _ => StructData::Dynamic(DynamicStruct::read(reader, save_archive)?),
        };

        Ok(data)
    }
}

impl PropertyReader for StrPropertyParser {
    fn read(
        &mut self,
        reader: &mut Reader,
        save_archive: &SaveGameArchiveContent,
        _size: u32,
    ) -> anyhow::Result<PropertyData> {
        reader.read_u8()?;

        let value = Self::read_raw(self, reader, save_archive)?;

        Ok(value)
    }

    fn read_head(
        &mut self,
        _reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<HeadData> {
        Ok(HeadData::None)
    }

    fn read_raw(
        &mut self,
        reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<PropertyData> {
        let value = reader.read_fstring()?;

        Ok(PropertyData::Str(value))
    }
}

impl PropertyReader for MapStructPropertyParser {
    fn read(
        &mut self,
        _reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
        _size: u32,
    ) -> anyhow::Result<PropertyData> {
        panic!("MapStructPropertyParser::read");
    }

    fn read_head(
        &mut self,
        _reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<HeadData> {
        Ok(HeadData::None)
    }

    fn read_raw(
        &mut self,
        reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<PropertyData> {
        let value = FGuid::read(reader)?;

        Ok(PropertyData::StructReference(value))
    }
}

impl PropertyReader for TextPropertyParser {
    fn read(
        &mut self,
        reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
        _size: u32,
    ) -> anyhow::Result<PropertyData> {
        reader.read_u8()?;

        let value = self.read_raw(reader, _save_archive)?;

        Ok(value)
    }

    fn read_head(
        &mut self,
        _reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<HeadData> {
        Ok(HeadData::None)
    }

    fn read_raw(
        &mut self,
        reader: &mut Reader,
        _save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<PropertyData> {
        let flags = reader.read_u32::<LittleEndian>()?;
        let history_type = reader.read_u8()?;

        let data = match history_type {
            0 => {
                // Base
                let namespace = reader.read_fstring()?;
                let key = reader.read_fstring()?;
                let source_string = reader.read_fstring()?;

                TextPropertyData::Base {
                    namespace,
                    key,
                    source_string,
                }
            }
            255 => {
                // None
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

        Ok(PropertyData::Text(TextProperty { flags, data }))
    }
}

macro_rules! impl_primitive_parser {
    (
        $name:ident, $read_method:ident, $prop_data_name:ident
    ) => {
        pub struct $name;

        impl PropertyReader for $name {
            fn read(
                &mut self,
                reader: &mut Reader,
                save_archive: &SaveGameArchiveContent,
                _size: u32,
            ) -> anyhow::Result<PropertyData> {
                reader.read_u8()?;

                let value = Self::read_raw(self, reader, save_archive)?;

                Ok(value)
            }

            fn read_head(
                &mut self,
                _reader: &mut Reader,
                _save_archive: &SaveGameArchiveContent,
            ) -> anyhow::Result<HeadData> {
                Ok(HeadData::None)
            }

            fn read_raw(
                &mut self,
                reader: &mut Reader,
                _save_archive: &SaveGameArchiveContent,
            ) -> anyhow::Result<PropertyData> {
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
