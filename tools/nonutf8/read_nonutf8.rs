extern crate simcli;
extern crate simcolor;
use crate::simcli::CLI;
use simcolor::Colorized;
use std::fs::{self, File};
use std::io::{Error, Read};
use std::path::PathBuf;
#[allow(unused)]
const VERSION: &str = env!("VERSION");
fn main() -> std::io::Result<()> {
    let mut cli = CLI::new();
    cli.description("Usage: nonutf8 <file name>");
    if cli.args().is_empty() {
        return Err(Error::other(cli.get_description().unwrap().bright().blue()));
    }
    let path = &cli.args()[0];
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
    fs::write(path, contents.as_bytes())
}
