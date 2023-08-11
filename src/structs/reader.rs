use crate::io::{Reader, ReaderExt};
use crate::properties::Property;
use crate::structs::{Actor, DateTime, DynamicActor, DynamicStruct, FGuid, FInfo, FPackageVersion, FQuaternion, FTopLevelAssetPath, FTransform, FVector, PersistenceBlob, PersistenceContainer, Timespan};
use byteorder::{LittleEndian, ReadBytesExt};
use std::collections::HashMap;
use std::io::{Read, Seek, SeekFrom};
use crate::sav::SaveGameArchiveContent;

impl FVector {
    pub fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let x = reader.read_f64::<LittleEndian>()?;
        let y = reader.read_f64::<LittleEndian>()?;
        let z = reader.read_f64::<LittleEndian>()?;

        let vector = FVector { x, y, z };

        Ok(vector)
    }
}

impl FQuaternion {
    pub fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let w = reader.read_f64::<LittleEndian>()?;
        let x = reader.read_f64::<LittleEndian>()?;
        let y = reader.read_f64::<LittleEndian>()?;
        let z = reader.read_f64::<LittleEndian>()?;

        let quaternion = FQuaternion { w, x, y, z };

        Ok(quaternion)
    }
}

impl FTransform {
    pub fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let rotation = FQuaternion::read(reader)?;
        let position = FVector::read(reader)?;
        let scale = FVector::read(reader)?;

        let transform = FTransform {
            rotation,
            position,
            scale,
        };

        Ok(transform)
    }
}

impl FGuid {
    pub fn read(reader: &mut Reader) -> anyhow::Result<FGuid> {
        let a = reader.read_u32::<LittleEndian>()?;
        let b = reader.read_u32::<LittleEndian>()?;
        let c = reader.read_u32::<LittleEndian>()?;
        let d = reader.read_u32::<LittleEndian>()?;

        Ok(FGuid { a, b, c, d })
    }
}

impl FTopLevelAssetPath {
    pub fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let path = reader.read_fstring()?;
        let name = reader.read_fstring()?;

        Ok(FTopLevelAssetPath { path, name })
    }
}

impl FInfo {
    pub fn read(reader: &mut Reader) -> anyhow::Result<FInfo> {
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

impl PersistenceBlob {
    pub fn read(reader: &mut Reader) -> anyhow::Result<PersistenceBlob> {
        let archive = SaveGameArchiveContent::read(reader, true, false)?;

        Ok(PersistenceBlob { archive })
    }
}

impl PersistenceContainer {
    pub fn read(reader: &mut Reader) -> anyhow::Result<PersistenceContainer> {
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

            let mut sub_reader = Reader::new(bytes, 8);
            let actor = Actor::read(&mut sub_reader)?;

            actors.insert(info.unique_id, actor);
        }

        reader.seek(SeekFrom::Start(dynamic_offset as u64))?;

        let dynamic_actor_count = reader.read_u32::<LittleEndian>()?;

        for _ in 0..dynamic_actor_count {
            let dynamic_actor = DynamicActor::read(reader)?;
            let actor = actors.get_mut(&dynamic_actor.unique_id).unwrap();

            actor.dynamic_data = Some(dynamic_actor);
        }

        Ok(PersistenceContainer {
            version,
            actors,
            destroyed,
        })
    }
}

impl Timespan {
    pub fn read(reader: &mut Reader) -> anyhow::Result<Timespan> {
        let value = reader.read_u64::<LittleEndian>()?;

        Ok(Timespan { value })
    }
}

impl DateTime {
    pub fn read(reader: &mut Reader) -> anyhow::Result<DateTime> {
        let value = reader.read_u64::<LittleEndian>()?;

        Ok(DateTime { value })
    }
}

impl DynamicStruct {
    pub fn read(
        reader: &mut Reader,
        save_archive: &SaveGameArchiveContent,
    ) -> anyhow::Result<DynamicStruct> {
        let properties = Property::read_multiple(reader, save_archive)?;

        Ok(DynamicStruct { properties })
    }
}

impl FPackageVersion {
    pub fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let ue4_version = reader.read_u32::<LittleEndian>()?;
        let ue5_version = reader.read_u32::<LittleEndian>()?;

        Ok(FPackageVersion {
            ue4_version,
            ue5_version,
        })
    }
}
