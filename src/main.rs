use std::fs;
use io::Reader;
use crate::io::Readable;
use crate::sav::SavFile;
use crate::sav_data::SavData;

mod io;
mod sav;
mod sav_data;
mod properties;

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
                if extension == "sav" && !path.file_name().unwrap().to_str().unwrap().starts_with("save") {
                    parse(path.to_str().unwrap())?;
                }
            }
        }
    }

    Ok(())
}

fn parse(path: &str) -> anyhow::Result<()> {
    println!("Parsing {}", path);

    let buf = fs::read(path)?;
    let mut reader = Reader::new(buf);

    let sav_file = SavFile::read(&mut reader)?;
    let sav_file_content = sav_file.get_content()?;

    let sav_data = SavData::read(&mut Reader::new(sav_file_content))?;

    fs::write(format!("{}.json", path), serde_json::to_vec_pretty(&sav_data)?)?;

    Ok(())
}
