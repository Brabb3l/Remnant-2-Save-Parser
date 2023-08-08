use std::io::{Seek, SeekFrom};
use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use crate::io::{Readable, Reader, ReaderExt};
use crate::properties::Property;

#[derive(Debug, Serialize, Deserialize)]
pub struct SavHeader {
    pub uncompressed_size: u32,
    pub build_number: u32,
    pub ue4_version: u32,
    pub ue5_version: u32,
    pub class_path: String,
    pub class_name: String,
    pub names_offset: u32,
    pub unk3: u32,
    pub unk4: u32,
    pub class_names_offset: u32,
    pub unk5: u64,
    pub unk6: u32,

    pub name_table: SavNameTable,
}

impl Readable for SavHeader {
    fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let uncompressed_size = reader.read_u32::<LittleEndian>()?;
        let build_number = reader.read_u32::<LittleEndian>()?;
        let ue4_version = reader.read_u32::<LittleEndian>()?;
        let ue5_version = reader.read_u32::<LittleEndian>()?;
        let class_path = reader.read_length_prefixed_cstring()?;
        let class_name = reader.read_length_prefixed_cstring()?;
        let names_offset = reader.read_u32::<LittleEndian>()?;
        let unk3 = reader.read_u32::<LittleEndian>()?;
        let unk4 = reader.read_u32::<LittleEndian>()?;
        let class_names_offset = reader.read_u32::<LittleEndian>()?;
        let unk5 = reader.read_u64::<LittleEndian>()?;
        let unk6 = reader.read_u32::<LittleEndian>()?;

        let name_table = SavNameTable::read(reader, names_offset - 8, class_names_offset - 8)?;

        Ok(SavHeader {
            uncompressed_size,
            build_number,
            ue4_version,
            ue5_version,
            class_path,
            class_name,
            names_offset,
            unk3,
            unk4,
            class_names_offset,
            unk5,
            unk6,
            name_table,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SavNameTable {
    pub names: Vec<String>,
    pub classes: Vec<ClassEntry>,
}

impl SavNameTable {
    pub fn read(reader: &mut Reader, names_offset: u32, class_names_offset: u32) -> anyhow::Result<Self> {
        let start_pos = reader.position();

        let names = Self::read_names(reader, names_offset)?;
        let classes = Self::read_imports(reader, class_names_offset)?;

        reader.seek(SeekFrom::Start(start_pos))?;

        Ok(
            SavNameTable {
                names,
                classes,
            }
        )
    }

    pub fn read_additional_class_data(&mut self, reader: &mut Reader) -> anyhow::Result<()> {
        let mut trim_offset = 0;

        for i in 0..self.classes.len() {
            if self.classes[i].data.is_none() {
                break;
            }

            trim_offset += 1;
        }

        for _ in 0..self.classes.len() - trim_offset {
            let id = reader.read_u32::<LittleEndian>()?;
            let length = reader.read_u32::<LittleEndian>()?;

            if length > 0 {
                let properties = Property::read_multiple(reader, self)?;

                reader.read_u32::<LittleEndian>()?;

                self.classes[id as usize].additional_data = Some(properties);
            };

            reader.read_u8()?;
        }

        Ok(())
    }

    fn read_names(reader: &mut Reader, offset: u32) -> anyhow::Result<Vec<String>> {
        reader.seek(SeekFrom::Start(offset as u64))?;

        let count = reader.read_u32::<LittleEndian>()?;
        let mut names = Vec::with_capacity(count as usize);

        for _ in 0..count {
            names.push(reader.read_length_prefixed_cstring()?);
        }

        Ok(names)
    }

    fn read_imports(reader: &mut Reader, offset: u32) -> anyhow::Result<Vec<ClassEntry>> {
        reader.seek(SeekFrom::Start(offset as u64))?;

        let count = reader.read_u32::<LittleEndian>()?;
        let mut imports = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let flag = reader.read_u8()? != 0;
            let name = reader.read_length_prefixed_cstring()?;

            let data = if !flag {
                let id = reader.read_u16::<LittleEndian>()?;
                let unk0 = reader.read_u16::<LittleEndian>()?;
                let unk1 = reader.read_i16::<LittleEndian>()?;
                let unk2 = reader.read_i32::<LittleEndian>()?;

                Some(ClassData {
                    id,
                    unk0,
                    unk1,
                    unk2,
                })
            } else {
                None
            };

            imports.push(ClassEntry {
                name,
                data,
                additional_data: None,
            });
        }

        Ok(imports)
    }

    pub fn read_name(&self, reader: &mut Reader) -> anyhow::Result<String> {
        Ok(self.names[reader.read_u16::<LittleEndian>()? as usize].clone())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SavData {
    pub header: SavHeader,
    pub objects: Vec<UObject>,
}

impl Readable for SavData {
    fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let mut header = SavHeader::read(reader)?;
        let name_table = &mut header.name_table;

        let mut objects = Vec::new();
        let first_object_properties = Property::read_multiple(reader, name_table)?;

        objects.push(
            UObject {
                unk0: 0,
                unk1: 0,
                unk2: 0,
                offset: 0,
                properties: first_object_properties,
            }
        );

        for _ in 0..2 {
            objects.push(UObject::read(reader, name_table)?);
        }

        reader.read_u32::<LittleEndian>()?;
        reader.read_u8()?;

        name_table.read_additional_class_data(reader)?;

        Ok(SavData {
            header,
            objects,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UObject {
    pub unk0: u8,
    pub unk1: u32,
    pub unk2: u32,
    pub offset: u32,
    pub properties: Vec<Property>,
}

impl UObject {
    fn read(reader: &mut Reader, name_table: &SavNameTable) -> anyhow::Result<UObject> {
        let unk0 = reader.read_u8()?;
        let unk1 = reader.read_u32::<LittleEndian>()?;
        let unk2 = reader.read_u32::<LittleEndian>()?;
        let offset = reader.read_u32::<LittleEndian>()?;

        let properties = Property::read_multiple(reader, name_table)?;

        Ok(UObject {
            unk0,
            unk1,
            unk2,
            offset,
            properties,
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClassEntry {
    pub name: String,
    pub data: Option<ClassData>,
    pub additional_data: Option<Vec<Property>>, // initialized last
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClassData {
    pub id: u16,
    pub unk0: u16,
    pub unk1: i16,
    pub unk2: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FGuid {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}

impl Readable for FGuid {
    fn read(reader: &mut Reader) -> anyhow::Result<FGuid> {
        let a = reader.read_u32::<LittleEndian>()?;
        let b = reader.read_u32::<LittleEndian>()?;
        let c = reader.read_u32::<LittleEndian>()?;
        let d = reader.read_u32::<LittleEndian>()?;

        Ok(FGuid {
            a,
            b,
            c,
            d,
        })
    }
}

impl FGuid {
    pub fn new() -> FGuid {
        FGuid {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
        }
    }
}