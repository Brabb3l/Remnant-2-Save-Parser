use std::io::{Seek, SeekFrom, Write};
use crate::io::{Writer, WriterExt};
use crate::properties::reader::{BytePropertyValue, TextPropertyData};
use crate::properties::{ArrayProperty, ByteProperty, EnumProperty, HeadData, MapProperty, Property, PropertyData, StructProperty, TextProperty};
use crate::structs::{FGuid, FName, StructData};
use anyhow::bail;
use byteorder::{LittleEndian, WriteBytesExt};
use crate::sav::NameTable;

pub trait PropertyWriter<T> {
    fn write(
        writer: &mut Writer,
        data: &T,
        name_table: &mut NameTable,
    ) -> anyhow::Result<u32>;

    fn write_raw(
        writer: &mut Writer,
        data: &T,
        name_table: &mut NameTable,
    ) -> anyhow::Result<u32>;
}

impl Property {
    pub fn write(
        &self,
        writer: &mut Writer,
        name_table: &mut NameTable,
    ) -> anyhow::Result<()> {
        name_table.write_name(writer, &self.name)?;
        name_table.write_name(writer, &self.type_name)?;
        writer.write_u32::<LittleEndian>(0)?; // placeholder for size
        writer.write_u32::<LittleEndian>(self.index)?;

        let start_pos = writer.position();
        let size = PropertyComposer::write(writer, &self.data, name_table)?;
        let end_pos = writer.position();
        
        writer.seek(SeekFrom::Start(start_pos - 8))?;
        writer.write_u32::<LittleEndian>(size)?;
        writer.seek(SeekFrom::Start(end_pos))?;

        Ok(())
    }

    pub fn write_none(
        writer: &mut Writer,
        name_table: &mut NameTable,
    ) -> anyhow::Result<()> {
        name_table.write_name(writer, &FName::from("None"))?;

        Ok(())
    }
}

pub struct PropertyComposer;

impl PropertyComposer {
    pub fn write(
        writer: &mut Writer,
        property_data: &PropertyData,
        name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        let size = match property_data {
            PropertyData::Byte(property_data) => {
                BytePropertyWriter::write(writer, property_data, name_table)?
            }
            PropertyData::Bool(property_data) => {
                BoolPropertyWriter::write(writer, property_data, name_table)?
            }
            PropertyData::Enum(property_data) => {
                EnumPropertyWriter::write(writer, property_data, name_table)?
            }
            PropertyData::Int16(property_data) => {
                Int16PropertyWriter::write(writer, property_data, name_table)?
            }
            PropertyData::Int32(property_data) => {
                IntPropertyWriter::write(writer, property_data, name_table)?
            }
            PropertyData::Int64(property_data) => {
                Int64PropertyWriter::write(writer, property_data, name_table)?
            }
            PropertyData::UInt16(property_data) => {
                UInt16PropertyWriter::write(writer, property_data, name_table)?
            }
            PropertyData::UInt32(property_data) => {
                UInt32PropertyWriter::write(writer, property_data, name_table)?
            }
            PropertyData::UInt64(property_data) => {
                UInt64PropertyWriter::write(writer, property_data, name_table)?
            }
            PropertyData::Float(property_data) => {
                FloatPropertyWriter::write(writer, property_data, name_table)?
            }
            PropertyData::Double(property_data) => {
                DoublePropertyWriter::write(writer, property_data, name_table)?
            }
            PropertyData::Array(property_data) => {
                ArrayPropertyWriter::write(writer, property_data, name_table)?
            }
            PropertyData::Map(property_data) => {
                MapPropertyWriter::write(writer, property_data, name_table)?
            }
            PropertyData::Object(property_data) => {
                ObjectPropertyWriter::write(writer, property_data, name_table)?
            }
            PropertyData::SoftObject(property_data) => {
                SoftObjectPropertyWriter::write(writer, property_data, name_table)?
            }
            PropertyData::Name(property_data) => {
                NamePropertyWriter::write(writer, property_data, name_table)?
            }
            PropertyData::Struct(property_data) => {
                StructPropertyWriter::write(writer, property_data, name_table)?
            }
            PropertyData::Str(property_data) => {
                StrPropertyWriter::write(writer, property_data, name_table)?
            }
            PropertyData::Text(property_data) => {
                TextPropertyWriter::write(writer, property_data, name_table)?
            }
            PropertyData::StructReference(property_data) => {
                MapStructPropertyWriter::write(writer, property_data, name_table)?
            }
        };

        Ok(size)
    }

    pub fn write_raw(
        writer: &mut Writer,
        property_data: &PropertyData,
        name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        let size = match property_data {
            PropertyData::Byte(property_data) => {
                BytePropertyWriter::write_raw(writer, property_data, name_table)?
            }
            PropertyData::Bool(property_data) => {
                BoolPropertyWriter::write_raw(writer, property_data, name_table)?
            }
            PropertyData::Enum(property_data) => {
                EnumPropertyWriter::write_raw(writer, property_data, name_table)?
            }
            PropertyData::Int16(property_data) => {
                Int16PropertyWriter::write_raw(writer, property_data, name_table)?
            }
            PropertyData::Int32(property_data) => {
                IntPropertyWriter::write_raw(writer, property_data, name_table)?
            }
            PropertyData::Int64(property_data) => {
                Int64PropertyWriter::write_raw(writer, property_data, name_table)?
            }
            PropertyData::UInt16(property_data) => {
                UInt16PropertyWriter::write_raw(writer, property_data, name_table)?
            }
            PropertyData::UInt32(property_data) => {
                UInt32PropertyWriter::write_raw(writer, property_data, name_table)?
            }
            PropertyData::UInt64(property_data) => {
                UInt64PropertyWriter::write_raw(writer, property_data, name_table)?
            }
            PropertyData::Float(property_data) => {
                FloatPropertyWriter::write_raw(writer, property_data, name_table)?
            }
            PropertyData::Double(property_data) => {
                DoublePropertyWriter::write_raw(writer, property_data, name_table)?
            }
            PropertyData::Array(property_data) => {
                ArrayPropertyWriter::write_raw(writer, property_data, name_table)?
            }
            PropertyData::Map(property_data) => {
                MapPropertyWriter::write_raw(writer, property_data, name_table)?
            }
            PropertyData::Object(property_data) => {
                ObjectPropertyWriter::write_raw(writer, property_data, name_table)?
            }
            PropertyData::SoftObject(property_data) => {
                SoftObjectPropertyWriter::write_raw(writer, property_data, name_table)?
            }
            PropertyData::Name(property_data) => {
                NamePropertyWriter::write_raw(writer, property_data, name_table)?
            }
            PropertyData::Struct(property_data) => {
                StructPropertyWriter::write_raw(writer, property_data, name_table)?
            }
            PropertyData::Str(property_data) => {
                StrPropertyWriter::write_raw(writer, property_data, name_table)?
            }
            PropertyData::Text(property_data) => {
                TextPropertyWriter::write_raw(writer, property_data, name_table)?
            }
            PropertyData::StructReference(property_data) => {
                MapStructPropertyWriter::write_raw(writer, property_data, name_table)?
            }
        };

        Ok(size)
    }
}

pub struct BytePropertyWriter;
pub struct BoolPropertyWriter;
pub struct EnumPropertyWriter;
pub struct MapPropertyWriter;
pub struct ArrayPropertyWriter;
pub struct ObjectPropertyWriter;
pub struct SoftObjectPropertyWriter;
pub struct NamePropertyWriter;
pub struct StructPropertyWriter;
pub struct StrPropertyWriter;
pub struct TextPropertyWriter;
pub struct MapStructPropertyWriter;

impl PropertyWriter<ByteProperty> for BytePropertyWriter {
    fn write(
        writer: &mut Writer,
        data: &ByteProperty,
        name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        name_table.write_name(writer, &data.enum_name)?;
        writer.write_u8(0)?;

        let size = match &data.value {
            BytePropertyValue::Byte(byte_value) => {
                writer.write_u8(*byte_value)?;
                1
            }
            BytePropertyValue::Enum(enum_value) => {
                name_table.write_name(writer, enum_value)?;
                2
            }
        };

        Ok(size)
    }

    fn write_raw(
        writer: &mut Writer,
        data: &ByteProperty,
        _name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        if let BytePropertyValue::Byte(value) = data.value {
            writer.write_u8(value)?;
        } else {
            bail!("Raw BytePropertyValue::Enum is not supported");
        }

        Ok(1)
    }
}

impl PropertyWriter<bool> for BoolPropertyWriter {
    fn write(
        writer: &mut Writer,
        data: &bool,
        name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        Self::write_raw(writer, data, name_table)?;
        writer.write_u8(0)?;

        Ok(0)
    }

    fn write_raw(
        writer: &mut Writer,
        data: &bool,
        _name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        writer.write_u8(*data as u8)?;

        Ok(0)
    }
}

impl PropertyWriter<EnumProperty> for EnumPropertyWriter {
    fn write(
        writer: &mut Writer,
        data: &EnumProperty,
        name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        name_table.write_name(writer, &data.enum_name)?;
        writer.write_u8(0)?;
        name_table.write_name(writer, &data.value)?;

        if data.enum_name.value == "None" {
            Ok(1)
        } else {
            Ok(2)
        }
    }

    fn write_raw(
        _writer: &mut Writer,
        _data: &EnumProperty,
        _name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        todo!("EnumPropertyWriter::write_raw")
    }
}

impl PropertyWriter<MapProperty> for MapPropertyWriter {
    fn write(
        writer: &mut Writer,
        data: &MapProperty,
        name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        name_table.write_name(writer, &data.key_type)?;
        name_table.write_name(writer, &data.value_type)?;

        writer.write_u8(0)?;
        writer.write_u32::<LittleEndian>(0)?;

        writer.write_u32::<LittleEndian>(data.elements.len() as u32)?;

        let mut size = 8;

        for (key, value) in &data.elements {
            size += PropertyComposer::write_raw(writer, key, name_table)?;
            size += PropertyComposer::write_raw(writer, value, name_table)?;
        }

        Ok(size)
    }

    fn write_raw(
        _writer: &mut Writer,
        _data: &MapProperty,
        _name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        todo!("MapPropertyWriter::write_raw")
    }
}

impl PropertyWriter<ArrayProperty> for ArrayPropertyWriter {
    fn write(
        writer: &mut Writer,
        data: &ArrayProperty,
        name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        name_table.write_name(writer, &data.inner_type)?;
        writer.write_u8(0)?;
        writer.write_u32::<LittleEndian>(data.elements.len() as u32)?;
        
        let mut size_pos = 0;
        let mut size = 4;

        match &data.head_data {
            HeadData::Struct {
                name,
                type_name,
                index,
                struct_name,
                guid,
            } => {
                name_table.write_name(writer, name)?;
                name_table.write_name(writer, type_name)?;
                size_pos = writer.position();
                writer.write_u32::<LittleEndian>(0)?; // placeholder for size
                writer.write_u32::<LittleEndian>(*index)?;
                name_table.write_name(writer, struct_name)?;
                guid.write(writer)?;
                writer.write_u8(0)?;

                size += 2 + 2 + 4 + 4 + 2 + 16 + 1;
            }
            HeadData::None => {}
        }

        let mut content_size = 0;
        
        for element in &data.elements {
            content_size += PropertyComposer::write_raw(writer, element, name_table)?;
        }

        size += content_size;

        if let HeadData::Struct { .. } = &data.head_data {
            let end_pos = writer.position();

            writer.seek(SeekFrom::Start(size_pos))?;
            writer.write_u32::<LittleEndian>(size)?;
            writer.seek(SeekFrom::Start(end_pos))?;
        }

        Ok(size)
    }

    fn write_raw(
        _writer: &mut Writer,
        _data: &ArrayProperty,
        _name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        todo!("ArrayPropertyWriter::write_raw")
    }
}

impl PropertyWriter<i32> for ObjectPropertyWriter {
    fn write(
        writer: &mut Writer,
        data: &i32,
        name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        writer.write_u8(0)?;
        Self::write_raw(writer, data, name_table)?;

        Ok(4)
    }

    fn write_raw(
        writer: &mut Writer,
        data: &i32,
        _name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        writer.write_i32::<LittleEndian>(*data)?;

        Ok(4)
    }
}

impl PropertyWriter<String> for SoftObjectPropertyWriter {
    fn write(
        writer: &mut Writer,
        data: &String,
        _name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        writer.write_u8(0)?;
        writer.write_fstring(data.clone())?;

        Ok(4 + data.len() as u32 + 1)
    }

    fn write_raw(
        _writer: &mut Writer,
        _data: &String,
        _name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        todo!("SoftObjectPropertyWriter::write_raw")
    }
}

impl PropertyWriter<FName> for NamePropertyWriter {
    fn write(
        writer: &mut Writer,
        data: &FName,
        name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        writer.write_u8(0)?;
        Self::write_raw(writer, data, name_table)?;

        Ok(2)
    }

    fn write_raw(
        writer: &mut Writer,
        data: &FName,
        name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        name_table.write_name(writer, data)?;

        Ok(2)
    }
}

impl PropertyWriter<String> for StrPropertyWriter {
    fn write(
        writer: &mut Writer,
        data: &String,
        name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        writer.write_u8(0)?;
        Self::write_raw(writer, data, name_table)?;

        Ok(4 + data.len() as u32 + 1)
    }

    fn write_raw(
        writer: &mut Writer,
        data: &String,
        _name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        writer.write_fstring(data.clone())?;

        Ok(4 + data.len() as u32 + 1)
    }
}

impl PropertyWriter<FGuid> for MapStructPropertyWriter {
    fn write(
        _writer: &mut Writer,
        _data: &FGuid,
        _name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        panic!("MapStructPropertyWriter::write")
    }

    fn write_raw(
        writer: &mut Writer,
        data: &FGuid,
        _name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        data.write(writer)?;

        Ok(16)
    }
}

impl PropertyWriter<TextProperty> for TextPropertyWriter {
    fn write(
        writer: &mut Writer,
        data: &TextProperty,
        _name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        writer.write_u8(0)?;
        let size = Self::write_raw(writer, data, _name_table)?;

        Ok(size)
    }

    fn write_raw(
        writer: &mut Writer,
        data: &TextProperty,
        _name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        writer.write_u32::<LittleEndian>(data.flags)?;

        let size = match &data.data {
            TextPropertyData::Base {
                namespace,
                key,
                source_string,
            } => {
                writer.write_u8(0)?;
                writer.write_fstring(namespace.clone())?;
                writer.write_fstring(key.clone())?;
                writer.write_fstring(source_string.clone())?;

                5 + (4 + namespace.len() as u32 + 1) + (4 + key.len() as u32 + 1) + (4 + source_string.len() as u32 + 1)
            }
            TextPropertyData::None {
                culture_invariant_string,
            } => {
                writer.write_u8(255)?;

                match culture_invariant_string {
                    Some(culture_invariant_string) => {
                        writer.write_u32::<LittleEndian>(1)?;
                        writer.write_fstring(culture_invariant_string.clone())?;
                        5 + 4 + (4 + culture_invariant_string.len() as u32 + 1)
                    }
                    None => {
                        writer.write_u32::<LittleEndian>(0)?;
                        5 + 4
                    }
                }
            }
        };

        Ok(size)
    }
}

macro_rules! impl_property_writer {
    (
        $name:ident, $write_method:ident, $prop_data_type:ty, $size:expr
    ) => {
        pub struct $name;

        impl PropertyWriter<$prop_data_type> for $name {
            fn write(
                writer: &mut Writer,
                data: &$prop_data_type,
                name_table: &mut NameTable,
            ) -> anyhow::Result<u32> {
                writer.write_u8(0)?;
                Self::write_raw(writer, data, name_table)?;

                Ok($size)
            }

            fn write_raw(
                writer: &mut Writer,
                data: &$prop_data_type,
                _name_table: &mut NameTable,
            ) -> anyhow::Result<u32> {
                writer.$write_method::<LittleEndian>(*data)?;

                Ok($size)
            }
        }
    };
}

impl_property_writer!(Int16PropertyWriter, write_i16, i16, 2);
impl_property_writer!(IntPropertyWriter, write_i32, i32, 4);
impl_property_writer!(Int64PropertyWriter, write_i64, i64, 8);
impl_property_writer!(UInt16PropertyWriter, write_u16, u16, 2);
impl_property_writer!(UInt32PropertyWriter, write_u32, u32, 4);
impl_property_writer!(UInt64PropertyWriter, write_u64, u64, 8);
impl_property_writer!(FloatPropertyWriter, write_f32, f32, 4);
impl_property_writer!(DoublePropertyWriter, write_f64, f64, 8);

impl PropertyWriter<StructProperty> for StructPropertyWriter {
    fn write(
        writer: &mut Writer,
        data: &StructProperty,
        name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        name_table.write_name(writer, &data.struct_name)?;
        data.guid.write(writer)?;

        writer.write_u8(0)?;

        let size = Self::write_struct_data(writer, data, name_table)?;

        Ok(size)
    }

    fn write_raw(
        writer: &mut Writer,
        data: &StructProperty,
        name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        let size = Self::write_struct_data(writer, data, name_table)?;

        Ok(size)
    }
}

impl StructPropertyWriter {
    fn write_struct_data(
        writer: &mut Writer,
        data: &StructProperty,
        name_table: &mut NameTable,
    ) -> anyhow::Result<u32> {
        let start_pos = writer.position();

        match &data.data {
            StructData::SoftClassPath(soft_class_path) => {
                writer.write_fstring(soft_class_path.clone())?;
            }
            StructData::SoftObjectPath(soft_object_path) => {
                writer.write_fstring(soft_object_path.clone())?;
            }
            StructData::Guid(guid) => {
                guid.write(writer)?;
            }
            StructData::Timespan(timespan) => {
                timespan.write(writer)?;
            }
            StructData::DateTime(date_time) => {
                date_time.write(writer)?;
            }
            StructData::Vector(vector) => {
                vector.write(writer)?;
            }
            StructData::Dynamic(dynamic_struct) => {
                dynamic_struct.write(writer, name_table)?;
            }
            StructData::PersistenceBlob(persistence_blob) => {
                let buf = Vec::new();
                let mut blob_writer = Writer::new(buf, 8);

                persistence_blob.write(&mut blob_writer)?;

                let buf = blob_writer.into_inner();

                writer.write_u32::<LittleEndian>(buf.len() as u32)?;
                writer.write_all(&buf)?;
            }
            StructData::PersistenceContainer(persistence_container) => {
                let buf = Vec::new();
                let mut blob_writer = Writer::new(buf, 8);

                persistence_container.write(&mut blob_writer)?;

                let buf = blob_writer.into_inner();

                writer.write_u32::<LittleEndian>(buf.len() as u32)?;
                writer.write_all(&buf)?;
            }
        }

        let end_pos = writer.position();

        Ok((end_pos - start_pos) as u32)
    }
}