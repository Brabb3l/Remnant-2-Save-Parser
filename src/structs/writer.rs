use std::io::{Seek, SeekFrom, Write};
use crate::io::{Writer, WriterExt};
use crate::structs::{Actor, DateTime, DynamicActor, DynamicStruct, FGuid, FInfo, FPackageVersion, FQuaternion, FTopLevelAssetPath, FTransform, FVector, PersistenceBlob, PersistenceContainer, Timespan};
use byteorder::{LittleEndian, WriteBytesExt};
use crate::properties::Property;
use crate::sav::NameTable;

impl FVector {
    pub fn write(&self, writer: &mut Writer) -> anyhow::Result<()> {
        writer.write_f64::<LittleEndian>(self.x)?;
        writer.write_f64::<LittleEndian>(self.y)?;
        writer.write_f64::<LittleEndian>(self.z)?;

        Ok(())
    }
}

impl FQuaternion {
    pub fn write(&self, writer: &mut Writer) -> anyhow::Result<()> {
        writer.write_f64::<LittleEndian>(self.w)?;
        writer.write_f64::<LittleEndian>(self.x)?;
        writer.write_f64::<LittleEndian>(self.y)?;
        writer.write_f64::<LittleEndian>(self.z)?;

        Ok(())
    }
}

impl FTransform {
    pub fn write(&self, writer: &mut Writer) -> anyhow::Result<()> {
        self.rotation.write(writer)?;
        self.position.write(writer)?;
        self.scale.write(writer)?;

        Ok(())
    }
}

impl FGuid {
    pub fn write(&self, writer: &mut Writer) -> anyhow::Result<()> {
        writer.write_u32::<LittleEndian>(self.a)?;
        writer.write_u32::<LittleEndian>(self.b)?;
        writer.write_u32::<LittleEndian>(self.c)?;
        writer.write_u32::<LittleEndian>(self.d)?;

        Ok(())
    }
}

impl FTopLevelAssetPath {
    pub fn write(&self, writer: &mut Writer) -> anyhow::Result<()> {
        writer.write_fstring(self.path.clone())?;
        writer.write_fstring(self.name.clone())?;

        Ok(())
    }
}

impl FInfo {
    fn write(&self, writer: &mut Writer) -> anyhow::Result<()> {
        writer.write_u64::<LittleEndian>(self.unique_id)?;
        writer.write_u32::<LittleEndian>(self.offset)?;
        writer.write_u32::<LittleEndian>(self.size)?;

        Ok(())
    }
}

impl DynamicActor {
    pub fn write(&self, writer: &mut Writer, unique_id: u64) -> anyhow::Result<()> {
        writer.write_u64::<LittleEndian>(unique_id)?;
        self.transform.write(writer)?;
        self.class_path.write(writer)?;

        Ok(())
    }
}

impl Actor {
    pub fn write(&self, writer: &mut Writer) -> anyhow::Result<()> {
        match &self.transform {
            Some(transform) => {
                writer.write_u32::<LittleEndian>(1)?;
                transform.write(writer)?;
            }
            None => {
                writer.write_u32::<LittleEndian>(0)?;
            }
        }

        self.archive.write(writer)?;

        Ok(())
    }
}

impl PersistenceBlob {
    pub fn write(&self, writer: &mut Writer) -> anyhow::Result<()> {
        self.archive.write(writer)?;

        Ok(())
    }
}

impl PersistenceContainer {
    pub fn write(&self, writer: &mut Writer) -> anyhow::Result<()> {
        writer.write_u32::<LittleEndian>(self.version)?;

        let index_offset = writer.position();
        writer.write_u32::<LittleEndian>(0)?; // placeholder for index offset

        let dynamic_actors_offset = writer.position();
        writer.write_u32::<LittleEndian>(0)?; // placeholder for dynamic actors offset

        let mut actor_info = Vec::new();

        for (unique_id, (_, actor)) in self.actors.iter().enumerate() {
            let offset = writer.position() as u32;
            let buf = Vec::new();
            let mut sub_writer = Writer::new(buf, 8);

            actor.write(&mut sub_writer)?;

            writer.write_all(&sub_writer.into_inner())?;

            let end_offset = writer.position() as u32;
            let info = FInfo {
                unique_id: unique_id as u64,
                offset,
                size: end_offset - offset,
            };

            actor_info.push((info, actor));
        }

        let index_offset_start = writer.position();

        writer.seek(SeekFrom::Start(index_offset))?;
        writer.write_u32::<LittleEndian>(index_offset_start as u32)?;
        writer.seek(SeekFrom::Start(index_offset_start))?;

        writer.write_u32::<LittleEndian>(actor_info.len() as u32)?;

        for (info, _) in &actor_info {
            info.write(writer)?;
        }

        writer.write_u32::<LittleEndian>(self.destroyed.len() as u32)?;

        for destroyed in &self.destroyed {
            writer.write_u64::<LittleEndian>(*destroyed)?;
        }

        let dynamic_actors_offset_start = writer.position();

        writer.seek(SeekFrom::Start(dynamic_actors_offset))?;
        writer.write_u32::<LittleEndian>(dynamic_actors_offset_start as u32)?;
        writer.seek(SeekFrom::Start(dynamic_actors_offset_start))?;

        let dynamic_actor_len_offset = writer.position();
        let mut dynamic_actor_len = 0;
        writer.write_u32::<LittleEndian>(0)?; // placeholder for dynamic actor length

        for (info, actor) in actor_info {
            if let Some(dynamic_actor) = &actor.dynamic_data {
                dynamic_actor.write(writer, info.unique_id)?;
                dynamic_actor_len += 1;
            }
        }

        writer.seek(SeekFrom::Start(dynamic_actor_len_offset))?;
        writer.write_u32::<LittleEndian>(dynamic_actor_len)?;

        Ok(())
    }
}

impl Timespan {
    pub fn write(&self, writer: &mut Writer) -> anyhow::Result<()> {
        writer.write_u64::<LittleEndian>(self.value)?;

        Ok(())
    }
}

impl DateTime {
    pub fn write(&self, writer: &mut Writer) -> anyhow::Result<()> {
        writer.write_u64::<LittleEndian>(self.value)?;

        Ok(())
    }
}

impl DynamicStruct {
    pub fn write(&self, writer: &mut Writer, name_table: &mut NameTable) -> anyhow::Result<()> {
        for property in &self.properties {
            property.write(writer, name_table)?;
        }

        Property::write_none(writer, name_table)?;

        Ok(())
    }
}

impl FPackageVersion {
    pub fn write(&self, writer: &mut Writer) -> anyhow::Result<()> {
        writer.write_u32::<LittleEndian>(self.ue4_version)?;
        writer.write_u32::<LittleEndian>(self.ue5_version)?;

        Ok(())
    }
}
