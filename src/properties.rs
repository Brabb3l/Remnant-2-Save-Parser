mod reader;
mod writer;

use crate::properties::reader::{BytePropertyValue, TextPropertyData};
use crate::structs::{FGuid, FName, StructData};
use serde::{Deserialize, Serialize};

const REMNANT_SAVE_GAME_PROFILE: &str = "/Game/_Core/Blueprints/Base/BP_RemnantSaveGameProfile";
const REMNANT_SAVE_GAME: &str = "/Game/_Core/Blueprints/Base/BP_RemnantSaveGame";

#[derive(Debug, Serialize, Deserialize)]
pub struct Property {
    pub name: FName,
    pub index: u32,
    pub type_name: FName,
    pub size: u32,
    pub data: PropertyData,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum PropertyData {
    Byte(ByteProperty),
    Bool(bool),
    Enum(EnumProperty),
    Int16(i16),
    Int32(i32),
    Int64(i64),
    UInt16(u16),
    UInt32(u32),
    UInt64(u64),
    Float(f32),
    Double(f64),
    Map(MapProperty),
    Array(ArrayProperty),
    Object(/* class_name_index: */ i32),
    SoftObject(String),
    Name(FName),
    Struct(StructProperty),
    Str(String),
    StructReference(FGuid),
    Text(TextProperty),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ByteProperty {
    pub enum_name: FName,
    pub value: BytePropertyValue,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EnumProperty {
    pub enum_name: FName,
    pub value: FName,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MapProperty {
    pub key_type: FName,
    pub value_type: FName,
    pub elements: Vec<(PropertyData, PropertyData)>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ArrayProperty {
    pub inner_type: FName,
    pub head_data: HeadData,
    pub elements: Vec<PropertyData>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StructProperty {
    pub struct_name: FName,
    pub guid: FGuid,
    pub data: StructData,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TextProperty {
    pub flags: u32,
    pub data: TextPropertyData,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum HeadData {
    Struct {
        name: FName,
        type_name: FName,
        index: u32,
        struct_name: FName,
        guid: FGuid,
    },
    None
}