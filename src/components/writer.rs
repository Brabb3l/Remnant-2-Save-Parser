use byteorder::{LittleEndian, WriteBytesExt};
use crate::components::{ComponentType, DynamicStructComponent, Variable, Variables, VariableValue};
use crate::io::Writer;
use crate::properties::Property;
use crate::sav::NameTable;

impl ComponentType {
    pub fn write(
        &self,
        writer: &mut Writer,
        name_table: &mut NameTable,
    ) -> anyhow::Result<()> {
        match self {
            ComponentType::GlobalVariables(variables) => {
                variables.write(writer, name_table)?;
            }
            ComponentType::Variables(variables) => {
                variables.write(writer, name_table)?;
            }
            ComponentType::Variable(variables) => {
                variables.write(writer, name_table)?;
            }
            ComponentType::PersistenceKeys(variables) => {
                variables.write(writer, name_table)?;
            }
            ComponentType::PersistanceKeys1(variables) => {
                variables.write(writer, name_table)?;
            }
            ComponentType::PersistenceKeys1(variables) => {
                variables.write(writer, name_table)?;
            }
            ComponentType::DynamicStruct(dynamic_struct) => {
                dynamic_struct.write(writer, name_table)?;
            }
        }

        Ok(())
    }
}

impl Variables {
    pub fn write(
        &self,
        writer: &mut Writer,
        name_table: &mut NameTable,
    ) -> anyhow::Result<()> {
        name_table.write_name(writer, &self.name)?;
        writer.write_u64::<LittleEndian>(0)?;
        writer.write_u32::<LittleEndian>(self.variables.len() as u32)?;

        for variable in &self.variables {
            variable.write(writer, name_table)?;
        }

        Ok(())
    }
}

impl Variable {
    pub fn write(
        &self,
        writer: &mut Writer,
        name_table: &mut NameTable,
    ) -> anyhow::Result<()> {
        name_table.write_name(writer, &self.name)?;

        match &self.value {
            VariableValue::None => {
                writer.write_u8(0)?;
                writer.write_u32::<LittleEndian>(0)?;
            }
            VariableValue::Bool(value) => {
                writer.write_u8(1)?;
                writer.write_u32::<LittleEndian>(*value as u32)?;
            }
            VariableValue::Int(value) => {
                writer.write_u8(2)?;
                writer.write_u32::<LittleEndian>(*value as u32)?;
            }
            VariableValue::Float(value) => {
                writer.write_u8(3)?;
                writer.write_f32::<LittleEndian>(*value)?;
            }
            VariableValue::Name(value) => { // TODO: Check if this is correct
                writer.write_u8(4)?;
                name_table.write_name(writer, value)?;
            }
        }

        Ok(())
    }
}

impl DynamicStructComponent {
    pub fn write(
        &self,
        writer: &mut Writer,
        name_table: &mut NameTable,
    ) -> anyhow::Result<()> {
        for field in &self.properties {
            field.write(writer, name_table)?;
        }

        Property::write_none(writer, name_table)?;

        writer.write_u64::<LittleEndian>(0)?;

        Ok(())
    }
}