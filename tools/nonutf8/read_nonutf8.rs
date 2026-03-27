extern crate simcolor;
use simcolor::Colorized;
use std::fs::File;
use std::io::Read;
use std::fs;
use std::path::PathBuf;
use std::io::Error;
mod cli;
use crate::cli::CLI;
#[allow(unused)]
const VERSION: &str = env!("VERSION");
fn main() -> std::io::Result<()> {
    let mut cli = CLI::new();
    cli.description("Usage: nonutf8 <file name>");
    if cli.args().is_empty() {
        return Err(Error::other(cli.get_description().unwrap().bright().blue()))
    }
    let path = &cli.args()[0];
    let mut file = File::open(path)?;
    let mut buf = vec![]; // with_capacity(file_len)
    file.read_to_end(&mut buf)?;
    let contents = String::from_utf8_lossy(&buf);
    //print!("{contents}");
    let mut bak_path = PathBuf::from(path);
    bak_path.set_extension("bak");
    if std::fs::rename(path, bak_path).is_ok() {
        fs::write(path, contents.as_bytes())?;
    }
    Ok(())
}
