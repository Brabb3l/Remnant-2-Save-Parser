use std::io::{Read, Seek, SeekFrom};
use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use crate::io::{Readable, Reader, ReaderExt};
use crate::sav_data::{FGuid, SavNameTable};

pub trait PropertyReader {
    fn read(&mut self, reader: &mut Reader, name_table: &SavNameTable, size: u32) -> anyhow::Result<PropertyData>;
    fn read_head(&mut self, reader: &mut Reader, name_table: &SavNameTable) -> anyhow::Result<()>;
    fn read_raw(&mut self, reader: &mut Reader, name_table: &SavNameTable) -> anyhow::Result<PropertyData>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub index: u32,
    pub type_name: String,
    pub size: u32,
    pub data: PropertyData,
}

impl Property {
    fn read(reader: &mut Reader, name_table: &SavNameTable) -> anyhow::Result<Option<Self>> {
        let name = name_table.read_name(reader)?;

        if name == "None" {
            return Ok(None);
        }

        let type_name = name_table.read_name(reader)?;
        let size = reader.read_u32::<LittleEndian>()?;
        let index = reader.read_u32::<LittleEndian>()?;

        let mut property_parser = PropertyParser::from_name(reader, &type_name, false)?;
        let data = property_parser.read(reader, name_table, size)?;

        let property = Property {
            name,
            index,
            type_name,
            size,
            data,
        };

        Ok(Some(property))
    }

    pub fn read_multiple(reader: &mut Reader, name_table: &SavNameTable) -> anyhow::Result<Vec<Property>> {
        let mut properties = Vec::new();

        loop {
            let property = Property::read(reader, name_table)?;

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
        enum_name: String,
        value: BytePropertyValue,
    },
    Bool(bool),
    Enum {
        enum_name: String,
        value: String,
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
        key_type: String,
        value_type: String,
        elements: Vec<(PropertyData, PropertyData)>,
    },
    Array {
        inner_type: String,
        elements: Vec<PropertyData>,
    },
    Object {
        class_name: Option<String>,
    },
    SoftObject {
        class_name: String,
    },
    Name(String),
    Struct {
        struct_name: String,
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
                    struct_name: String::new(),
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
    Enum(String),
    Byte(u8),
}

pub struct BytePropertyParser;

impl PropertyReader for BytePropertyParser {
    fn read(&mut self, reader: &mut Reader, name_table: &SavNameTable, _size: u32) -> anyhow::Result<PropertyData> {
        let enum_name = name_table.read_name(reader)?;

        reader.read_u8()?;

        let value = if enum_name == "None" {
            BytePropertyValue::Byte(reader.read_u8()?)
        } else {
            BytePropertyValue::Enum(name_table.read_name(reader)?)
        };

        Ok(PropertyData::Byte {
            enum_name,
            value,
        })
    }

    fn read_head(&mut self, _reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<PropertyData> {
        let value = reader.read_u8()?;

        Ok(PropertyData::Byte {
            enum_name: "None".to_owned(),
            value: BytePropertyValue::Byte(value),
        })
    }
}

pub struct BoolPropertyParser;

impl PropertyReader for BoolPropertyParser {
    fn read(&mut self, reader: &mut Reader, name_table: &SavNameTable, _size: u32) -> anyhow::Result<PropertyData> {
        let value = Self::read_raw(self, reader, name_table)?;

        reader.read_u8()?;

        Ok(value)
    }

    fn read_head(&mut self, _reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<PropertyData> {
        let value = reader.read_u8()?;

        Ok(PropertyData::Bool(value != 0))
    }

}

pub struct EnumPropertyParser;

impl PropertyReader for EnumPropertyParser {
    fn read(&mut self, reader: &mut Reader, name_table: &SavNameTable, _size: u32) -> anyhow::Result<PropertyData> {
        let enum_name = name_table.read_name(reader)?;

        reader.read_u8()?;

        let value = name_table.read_name(reader)?;

        Ok(PropertyData::Enum {
            enum_name,
            value,
        })
    }

    fn read_head(&mut self, _reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, _reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<PropertyData> {
        todo!("EnumPropertyParser::read_raw")
    }
}

macro_rules! impl_primitive_parser {
    (
        $name:ident, $read_method:ident, $prop_data_name:ident
    ) => {
        pub struct $name;

        impl PropertyReader for $name {
            fn read(&mut self, reader: &mut Reader, name_table: &SavNameTable, _size: u32) -> anyhow::Result<PropertyData> {
                reader.read_u8()?;

                let value = Self::read_raw(self, reader, name_table)?;

                Ok(value)
            }

            fn read_head(&mut self, _reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<()> {
                Ok(())
            }

            fn read_raw(&mut self, reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<PropertyData> {
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
    fn read(&mut self, reader: &mut Reader, name_table: &SavNameTable, _size: u32) -> anyhow::Result<PropertyData> {
        let key_type = name_table.read_name(reader)?;
        let value_type = name_table.read_name(reader)?;

        let mut key_parser = PropertyParser::from_name(reader, key_type.as_str(), true)?;
        let mut value_parser = PropertyParser::from_name(reader, value_type.as_str(), false)?;

        reader.read_u8()?;
        reader.read_u32::<LittleEndian>()?;

        let element_count = reader.read_u32::<LittleEndian>()?;
        let mut elements = Vec::with_capacity(element_count as usize);

        for _ in 0..element_count {
            let key = key_parser.read_raw(reader, name_table)?;
            let value = value_parser.read_raw(reader, name_table)?;

            elements.push((key, value));
        }

        Ok(PropertyData::Map {
            key_type,
            value_type,
            elements,
        })
    }

    fn read_head(&mut self, _reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, _reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<PropertyData> {
        todo!("MapPropertyParser::read_raw")
    }
}

struct ArrayPropertyParser;

impl PropertyReader for ArrayPropertyParser {
    fn read(&mut self, reader: &mut Reader, name_table: &SavNameTable, _size: u32) -> anyhow::Result<PropertyData> {
        let inner_type = name_table.read_name(reader)?;
        let mut inner_parser = PropertyParser::from_name(reader, inner_type.as_str(), false)?;

        reader.read_u8()?;

        let element_count = reader.read_u32::<LittleEndian>()?;
        let mut elements = Vec::with_capacity(element_count as usize);

         inner_parser.read_head(reader, name_table)?;

        for _ in 0..element_count {
            let value = inner_parser.read_raw(reader, name_table)?;

            elements.push(value);
        }

        Ok(PropertyData::Array {
            inner_type,
            elements,
        })
    }

    fn read_head(&mut self, _reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, _reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<PropertyData> {
        todo!("ArrayPropertyParser::read_raw")
    }
}

struct ObjectPropertyParser;

impl PropertyReader for ObjectPropertyParser {
    fn read(&mut self, reader: &mut Reader, name_table: &SavNameTable, _size: u32) -> anyhow::Result<PropertyData> {
        reader.read_u8()?;

        let value = Self::read_raw(self, reader, name_table)?;

        Ok(value)
    }

    fn read_head(&mut self, _reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, reader: &mut Reader, name_table: &SavNameTable) -> anyhow::Result<PropertyData> {
        let class_name_index = reader.read_i32::<LittleEndian>()?;
        let class_name = if class_name_index == -1 {
            None
        } else {
            Some(name_table.classes[class_name_index as usize].name.clone())
        };

        Ok(PropertyData::Object {
            class_name,
        })
    }
}

struct SoftObjectPropertyParser;

impl PropertyReader for SoftObjectPropertyParser {
    fn read(&mut self, reader: &mut Reader, _name_table: &SavNameTable, _size: u32) -> anyhow::Result<PropertyData> {
        reader.read_u8()?;

        let class_name = reader.read_length_prefixed_cstring()?;

        Ok(PropertyData::SoftObject {
            class_name,
        })
    }

    fn read_head(&mut self, _reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, _reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<PropertyData> {
        todo!("SoftObjectPropertyParser::read_raw")
    }
}

pub struct NamePropertyParser;

impl PropertyReader for NamePropertyParser {
    fn read(&mut self, reader: &mut Reader, name_table: &SavNameTable, _size: u32) -> anyhow::Result<PropertyData> {
        reader.read_u8()?;

        let value = Self::read_raw(self, reader, name_table)?;

        Ok(value)
    }

    fn read_head(&mut self, _reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, reader: &mut Reader, name_table: &SavNameTable) -> anyhow::Result<PropertyData> {
        let value = name_table.read_name(reader)?;

        Ok(PropertyData::Name(value))
    }
}

struct StructPropertyParser {
    size: u32,
    struct_name: String,
    guid: FGuid,
}

impl PropertyReader for StructPropertyParser {
    fn read(&mut self, reader: &mut Reader, name_table: &SavNameTable, size: u32) -> anyhow::Result<PropertyData> {
        self.struct_name = name_table.read_name(reader)?;
        self.guid = FGuid::read(reader)?;
        self.size = size;

        reader.read_u8()?;

        let data = self.read_struct_data(reader, name_table, self.size)?;

        Ok(PropertyData::Struct {
            struct_name: self.struct_name.clone(),
            guid: self.guid.clone(),
            data,
        })
    }

    fn read_head(&mut self, reader: &mut Reader, name_table: &SavNameTable) -> anyhow::Result<()> {
        let _name = name_table.read_name(reader)?;
        let _type_name = name_table.read_name(reader)?;
        self.size = reader.read_u32::<LittleEndian>()?;
        let _index = reader.read_u32::<LittleEndian>()?;
        self.struct_name = name_table.read_name(reader)?;
        self.guid = FGuid::read(reader)?;

        reader.read_u8()?;

        Ok(())
    }

    fn read_raw(&mut self, reader: &mut Reader, name_table: &SavNameTable) -> anyhow::Result<PropertyData> {
        let data = self.read_struct_data(reader, name_table, self.size)?;

        Ok(PropertyData::Struct {
            struct_name: self.struct_name.clone(),
            guid: self.guid.clone(),
            data,
        })
    }
}

impl StructPropertyParser {
    fn read_struct_data(&self, reader: &mut Reader, name_table: &SavNameTable, size: u32) -> anyhow::Result<StructData> {
        let data = match self.struct_name.as_str() {
            "SoftClassPath" => {
                let value = reader.read_length_prefixed_cstring()?;

                StructData::SoftClassPath {
                    value,
                }
            }
            "SoftObjectPath" => {
                let value = reader.read_length_prefixed_cstring()?;

                StructData::SoftObjectPath {
                    value,
                }
            }
            "PersistenceBlob" => {
                let mut value = vec![0; size as usize];

                reader.read_exact(&mut value)?;

                let reader = &mut Reader::new(value);

                let size = reader.read_u32::<LittleEndian>()?;
                let unk0 = reader.read_u32::<LittleEndian>()?;

                if unk0 == 4 { // disambiguate between save and profile version like this for now
                    // parse save version

                    let unk1 = reader.read_u32::<LittleEndian>()?; // some size or offset?
                    let unk2 = reader.read_u32::<LittleEndian>()?; // some size or offset?
                    let unk3 = reader.read_u32::<LittleEndian>()?;
                    let names_offset = reader.read_u32::<LittleEndian>()?;
                    let unk4 = reader.read_u32::<LittleEndian>()?;
                    let unk5 = reader.read_u32::<LittleEndian>()?;
                    let class_names_offset = reader.read_u32::<LittleEndian>()?;
                    let unk6 = reader.read_u32::<LittleEndian>()?;
                    let unk7 = reader.read_u32::<LittleEndian>()?;
                    let first_obj_size = reader.read_u32::<LittleEndian>()?;

                    let mut name_table = SavNameTable::read(reader, names_offset + 16, class_names_offset + 16)?;
                    let first_obj_properties = Property::read_multiple(reader, &name_table)?;

                    reader.read_u64::<LittleEndian>()?;
                    reader.read_u8()?;

                    let object_count = reader.read_u32::<LittleEndian>()?;
                    let mut objects = Vec::new();

                    objects.push(
                        PersistenceBlobObject {
                            name: "".to_owned(),
                            size: first_obj_size,
                            properties: first_obj_properties,
                        }
                    );

                    for _ in 0..object_count {
                        let object = PersistenceBlobObject::read_object(reader, &name_table)?;

                        objects.push(object);
                    }

                    name_table.read_additional_class_data(reader)?;

                    StructData::PersistenceBlob2 {
                        size,
                        unk0,
                        unk1,
                        unk2,
                        unk3,
                        names_offset,
                        unk4,
                        unk5,
                        class_names_offset,
                        unk6,
                        unk7,
                        objects,
                        name_table,
                    }
                } else {
                    // parse profile version

                    let unk1 = reader.read_u32::<LittleEndian>()?;
                    let names_offset = reader.read_u32::<LittleEndian>()?;
                    let unk2 = reader.read_u32::<LittleEndian>()?;
                    let unk3 = reader.read_u32::<LittleEndian>()?;
                    let class_names_offset = reader.read_u32::<LittleEndian>()?;
                    let unk4 = reader.read_u32::<LittleEndian>()?;

                    let mut name_table = SavNameTable::read(reader, names_offset + 4, class_names_offset + 4)?;

                    let first_object = PersistenceBlobObject::read_object(reader, &name_table)?;
                    let flag = reader.read_u8()?;
                    let object_count = reader.read_u32::<LittleEndian>()?;

                    let mut objects = Vec::new();

                    objects.push(first_object);

                    for _ in 0..object_count {
                        let object = PersistenceBlobObject::read_object(reader, &name_table)?;

                        objects.push(object);
                    }

                    name_table.read_additional_class_data(reader)?;

                    StructData::PersistenceBlob {
                        size,
                        unk0,
                        unk1,
                        names_offset,
                        unk2,
                        unk3,
                        class_names_offset,
                        unk4,
                        flag,
                        object_count,
                        objects,
                        name_table,
                    }
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
            "Vector" => {
                let x = reader.read_f64::<LittleEndian>()?;
                let y = reader.read_f64::<LittleEndian>()?;
                let z = reader.read_f64::<LittleEndian>()?;

                StructData::Vector {
                    x,
                    y,
                    z,
                }
            }
            _ => {
                let properties = Property::read_multiple(reader, name_table)?;

                StructData::Dynamic {
                    properties,
                }
            }
        };

        Ok(data)
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
        size: u32,
        unk0: u32,
        unk1: u32,
        names_offset: u32,
        unk2: u32,
        unk3: u32,
        class_names_offset: u32,
        unk4: u32,
        flag: u8,
        object_count: u32,
        objects: Vec<PersistenceBlobObject>,
        name_table: SavNameTable,
    },
    PersistenceBlob2 {
        size: u32,
        unk0: u32,
        unk1: u32,
        unk2: u32,
        unk3: u32,
        names_offset: u32,
        unk4: u32,
        unk5: u32,
        class_names_offset: u32,
        unk6: u32,
        unk7: u32,
        objects: Vec<PersistenceBlobObject>,
        name_table: SavNameTable,
    },
    Guid {
        value: FGuid,
    },
    Timespan {
        value: u64,
    },
    Vector {
        x: f64,
        y: f64,
        z: f64,
    },
    Dynamic {
        properties: Vec<Property>,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistenceBlobObject {
    name: String,
    size: u32,
    properties: Vec<Property>,
}

impl PersistenceBlobObject {
    fn read_object(reader: &mut Reader, name_table: &SavNameTable) -> anyhow::Result<PersistenceBlobObject> {
        let name = reader.read_length_prefixed_cstring()?;
        let size = reader.read_u32::<LittleEndian>()?;
        let start = reader.position();
        let properties = Property::read_multiple(reader, name_table)?;

        reader.seek(SeekFrom::Start(start + size as u64))?;

        Ok(PersistenceBlobObject {
            name,
            size,
            properties,
        })
    }
}

struct StrPropertyParser;

impl PropertyReader for StrPropertyParser {
    fn read(&mut self, reader: &mut Reader, name_table: &SavNameTable, _size: u32) -> anyhow::Result<PropertyData> {
        reader.read_u8()?;

        let value = Self::read_raw(self, reader, name_table)?;

        Ok(value)
    }

    fn read_head(&mut self, _reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<PropertyData> {
        let value = reader.read_length_prefixed_cstring()?;

        Ok(PropertyData::Str(value))
    }
}

struct MapStructPropertyParser;

impl PropertyReader for MapStructPropertyParser {
    fn read(&mut self, _reader: &mut Reader, _name_table: &SavNameTable, _size: u32) -> anyhow::Result<PropertyData> {
        panic!("Unsupported operation");
    }

    fn read_head(&mut self, _reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<PropertyData> {
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
    fn read(&mut self, reader: &mut Reader, _name_table: &SavNameTable, _size: u32) -> anyhow::Result<PropertyData> {
        reader.read_u8()?;

        let value = self.read_raw(reader, _name_table)?;

        Ok(value)
    }

    fn read_head(&mut self, _reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<()> {
        Ok(())
    }

    fn read_raw(&mut self, reader: &mut Reader, _name_table: &SavNameTable) -> anyhow::Result<PropertyData> {
        let flags = reader.read_u32::<LittleEndian>()?;
        let history_type = reader.read_u8()?;

        let data = match history_type {
            0 => { // Base
                let namespace = reader.read_length_prefixed_cstring()?;
                let key = reader.read_length_prefixed_cstring()?;
                let source_string = reader.read_length_prefixed_cstring()?;

                TextPropertyData::Base {
                    namespace,
                    key,
                    source_string,
                }
            }
            255 => { // None
                let has_culture_invariant_string = reader.read_u32::<LittleEndian>()? != 0;

                let culture_invariant_string = if has_culture_invariant_string {
                    Some(reader.read_length_prefixed_cstring()?)
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
