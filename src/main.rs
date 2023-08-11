use io::Reader;
use std::fs;
use std::path::PathBuf;
use crate::sav::SavFile;

mod io;
mod properties;
mod sav;
mod structs;
mod components;

fn main() -> anyhow::Result<()> {
    parse_all_in(".")?;

    Ok(())
}

fn parse_all_in(dir: &str) -> anyhow::Result<()> {
    let dir = fs::read_dir(dir)?;

    for entry in dir {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() {
            if let Some(extension) = path.extension() {
                if extension == "sav" {
                    println!("Parsing {:?}", path);

                    unpack(&path, ".")?;
                }
            }
        }
    }

    Ok(())
}

fn unpack(input_file: &PathBuf, output_dir: &str) -> anyhow::Result<()> {
    let file_name = input_file.file_name()
        .and_then(|x| x.to_str())
        .ok_or_else(|| anyhow::anyhow!("Invalid file name"))?;

    // parse binary file

    let input_bytes = fs::read(input_file)?;
    let mut reader = Reader::new(input_bytes, 4);

    let sav_file = SavFile::read(&mut reader)?;
    let archive = sav_file.get_archive()?;

    let json = serde_json::to_vec_pretty(&archive)?;

    // write json file

    let mut output_file = PathBuf::from(output_dir);

    output_file.push(format!("{}.json", file_name));

    fs::write(output_file, json)?;

    Ok(())
}
