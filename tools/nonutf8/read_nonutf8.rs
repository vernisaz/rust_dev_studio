extern crate simcli;
extern crate simcolor;
#[cfg(target_os = "windows")]
use crate::simcli::WildCardExpansion;
use crate::simcli::{CLI, OptTyp};
use simcolor::Colorized;
use std::{
    fs::{self, File},
    io::{Error, Read},
    path::PathBuf,
};

const VERSION: &str = env!("VERSION");
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut cli = CLI::new();
    cli.description("Usage: nonutf8 <file name>...")
        .opt("v", OptTyp::None)
        .unwrap();
    #[cfg(target_os = "windows")]
    cli.process_wildcard(WildCardExpansion::All);
    if cli.get_opt("v").unwrap().is_some() {
        println!("nonutf8 v{VERSION}");
        return Ok(());
    }
    if cli.args().is_empty() {
        return Err(Box::new(cli.get_description().unwrap().bright().blue()));
    }
    for path in cli.args() {
        let mut file = File::open(path)?;
        let mut buf = Vec::with_capacity(
            file.metadata()?
                .len()
                .try_into()
                .map_err(|_e| Error::other("not enough memory"))?,
        );
        file.read_to_end(&mut buf)?;
        let contents = String::from_utf8_lossy(&buf);
        //print!("{contents}");
        let mut bak_path = PathBuf::from(path);
        bak_path.set_extension("bak");
        fs::rename(path, bak_path)?;
        fs::write(path, contents.as_bytes())?;
    }
    Ok(())
}
