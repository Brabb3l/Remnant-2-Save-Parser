mod reader;
mod writer;

use serde::{Deserialize, Serialize};
use crate::properties::Property;
use crate::structs::FName;

#[derive(Debug, Serialize, Deserialize)]
pub struct Component {
    pub component_key: String,
    pub component_type: ComponentType,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ComponentType {
    GlobalVariables(Variables),
    Variables(Variables),
    Variable(Variables),
    PersistenceKeys(Variables),
    PersistanceKeys1(Variables),
    PersistenceKeys1(Variables),
    DynamicStruct(DynamicStructComponent),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Variables {
    pub name: FName,
    pub variables: Vec<Variable>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Variable {
    pub name: FName,
    pub value: VariableValue,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum VariableValue {
    None,
    Bool(bool),
    Int(i32),
    Float(f32),
    Name(FName), // TODO: is this correct?
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DynamicStructComponent {
    pub properties: Vec<Property>,
}