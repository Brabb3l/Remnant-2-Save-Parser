mod reader;
mod writer;

use crate::properties::Property;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::sav::SaveGameArchiveContent;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FVector {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FQuaternion {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FTransform {
    pub rotation: FQuaternion,
    pub position: FVector,
    pub scale: FVector,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FGuid {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FTopLevelAssetPath {
    pub path: String,
    pub name: String,
}

#[derive(Debug)]
pub struct FInfo {
    pub unique_id: u64,
    pub offset: u32,
    pub size: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DynamicActor {
    pub unique_id: u64,
    pub transform: FTransform,
    pub class_path: FTopLevelAssetPath,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Actor {
    pub transform: Option<FTransform>,
    pub archive: SaveGameArchiveContent,
    pub dynamic_data: Option<DynamicActor>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistenceBlob {
    pub archive: SaveGameArchiveContent,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersistenceContainer {
    pub version: u32,
    pub destroyed: Vec<u64>,
    pub actors: HashMap<u64, Actor>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Timespan {
    pub value: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DateTime {
    pub value: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DynamicStruct {
    pub properties: Vec<Property>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum StructData {
    SoftClassPath(String),
    SoftObjectPath(String),
    PersistenceBlob(PersistenceBlob),
    PersistenceContainer(PersistenceContainer),
    Guid(FGuid),
    Timespan(Timespan),
    DateTime(DateTime),
    Vector(FVector),
    Dynamic(DynamicStruct),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FName {
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none", default = "Option::default")]
    pub number: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FPackageVersion {
    pub ue4_version: u32,
    pub ue5_version: u32,
}

// utility functions

impl FName {
    pub fn from(name: &str) -> FName {
        FName {
            value: name.to_owned(),
            number: None,
        }
    }

    pub fn none() -> FName {
        FName::from("None")
    }
}
