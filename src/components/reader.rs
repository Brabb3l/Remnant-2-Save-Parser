use anyhow::bail;
use byteorder::{LittleEndian, ReadBytesExt};
use crate::components::{ComponentType, DynamicStructComponent, Variable, Variables, VariableValue};
use crate::io::Reader;
use crate::properties::Property;
use crate::sav::SaveGameArchiveContent;

impl ComponentType {
    pub fn read(
        reader: &mut Reader,
        sav_data: &SaveGameArchiveContent,
        name: &str,
    ) -> anyhow::Result<Self> {
        match name {
            "GlobalVariables" => {
                let variables = Variables::read(reader, sav_data)?;

                Ok(ComponentType::GlobalVariables(variables))
            }
            "Variables" => {
                let variables = Variables::read(reader, sav_data)?;

                Ok(ComponentType::Variables(variables))
            }
            "Variable" => {
                let variables = Variables::read(reader, sav_data)?;

                Ok(ComponentType::Variable(variables))
            }
            "PersistenceKeys" => {
                let variables = Variables::read(reader, sav_data)?;

                Ok(ComponentType::PersistenceKeys(variables))
            }
            "PersistanceKeys1" => {
                let variables = Variables::read(reader, sav_data)?;

                Ok(ComponentType::PersistenceKeys1(variables))
            }
            "PersistenceKeys1" => {
                let variables = Variables::read(reader, sav_data)?;

                Ok(ComponentType::PersistenceKeys1(variables))
            }
            _ => {
                let dynamic_struct = DynamicStructComponent::read(reader, sav_data)?;

                Ok(ComponentType::DynamicStruct(dynamic_struct))
            },
        }
    }
}

impl Variables {
    pub fn read(reader: &mut Reader, sav_data: &SaveGameArchiveContent) -> anyhow::Result<Self> {
        let name = sav_data.read_name(reader)?;
        let empty = reader.read_u64::<LittleEndian>()?;

        if empty != 0 {
            bail!("Variables::read: {:X}", empty);
        }

        let count = reader.read_u32::<LittleEndian>()?;
        let mut variables = Vec::with_capacity(count as usize);

        for _ in 0..count {
            let variable = Variable::read(reader, sav_data)?;

            variables.push(variable);
        }

        Ok(Variables { name, variables })
    }
}

impl Variable {
    pub fn read(reader: &mut Reader, sav_data: &SaveGameArchiveContent) -> anyhow::Result<Self> {
        let name = sav_data.read_name(reader)?;
        let var_type = reader.read_u8()?;

        let value = match var_type {
            0 => VariableValue::None,
            1 => VariableValue::Bool(reader.read_u32::<LittleEndian>()? != 0),
            2 => VariableValue::Int(reader.read_i32::<LittleEndian>()?),
            3 => VariableValue::Float(reader.read_f32::<LittleEndian>()?),
            4 => VariableValue::Name(sav_data.read_name(reader)?), // TODO: check if this is correct
            _ => bail!(anyhow::anyhow!("Variable::read: unknown var_type")),
        };

        Ok(Variable { name, value })
    }
}

impl DynamicStructComponent {
    pub fn read(reader: &mut Reader, sav_data: &SaveGameArchiveContent) -> anyhow::Result<Self> {
        let properties = Property::read_multiple(reader, sav_data)?;
        let empty = reader.read_u64::<LittleEndian>()?;

        if empty != 0 {
            bail!("DynamicStruct::read: {:X?}", empty);
        }

        Ok(DynamicStructComponent { properties })
    }
}