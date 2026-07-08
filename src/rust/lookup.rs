use std::io::BufReader;
fn lookup(s: &str) -> Option<String> {
    // read 'rust subs.md'
    let filename = "kb/rust subs.md";
    let file = File::open(filename).ok()?;
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut current_subsystem = None;
    while let Some(line) = lines.next() {
        let line = line.ok()?;
        if line.starts_with("# ") {
            continue;
        }
        if let Some(new_current_subsystem) = line.strip_prefix("## ") {
            let new_current_subsystem = new_current_subsystem.trim().to_string();
            if s == new_current_subsystem {
                return Some(new_current_subsystem);
            }
            current_subsystem = Some(new_current_subsystem);
            continue;
        } else {
            // parse it looking for the string every bucket
            for token in line.split_whitespace() {
                if s == token {
                    // return name of the bucket
                    return current_subsystem;
                }
            }
        }
    }

    // return  None
    None
}
