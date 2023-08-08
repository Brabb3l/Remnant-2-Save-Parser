use byteorder::{LittleEndian, ReadBytesExt};
use serde::{Deserialize, Serialize};
use crate::io::Reader;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FVector {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl FVector {
    pub fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let x = reader.read_f64::<LittleEndian>()?;
        let y = reader.read_f64::<LittleEndian>()?;
        let z = reader.read_f64::<LittleEndian>()?;

        let vector = FVector {
            x,
            y,
            z,
        };

        Ok(vector)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FQuaternion {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl FQuaternion {
    pub fn read(reader: &mut Reader) -> anyhow::Result<Self> {
        let w = reader.read_f64::<LittleEndian>()?;
        let x = reader.read_f64::<LittleEndian>()?;
        let y = reader.read_f64::<LittleEndian>()?;
        let z = reader.read_f64::<LittleEndian>()?;

        let quaternion = FQuaternion {
            w,
            x,
            y,
            z,
        };

        Ok(quaternion)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FTransform {
    pub rotation: FQuaternion,
    pub position: FVector,
    pub scale: FVector,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FGuid {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}

impl FGuid {
    pub fn read(reader: &mut Reader) -> anyhow::Result<FGuid> {
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
