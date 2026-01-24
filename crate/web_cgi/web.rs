use std::{collections::HashMap, io, fs::{self},
    path::{Path,Component}, time::SystemTime, error::Error};
use crate::template;
use crate::param;
use crate::web::Menu::{MenuEnd, MenuBox, MenuItem, Separator};

use simtime::{DAYS_OF_WEEK, get_datetime, get_local_timezone_offset};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Menu <'a> {
    MenuBox {
        title: String, // HTML encode applied
        hint: Option<&'a str>, // HTML encode applied
        icon: Option<&'a str>,
    },
    
    MenuItem {
        link: String,  // URL encode isn't applied
        title: String, // HTML encode applied
        short: Option<&'a str>,
        hint: Option<&'a str>,
        icon: Option<&'a str>,
    },
    MenuEnd,
    Separator,
}

pub trait PageOps {
    fn content_type(&self) -> String {
        "text/html".to_string()
    }

    fn main_load(&self) -> Result<String, Box<dyn Error>>;

    fn name(&self) -> String;

    fn get_nav(&self) -> Option<Vec<Menu<'_>>> {None}
    
    // any additional header including cookie set
    fn get_extra(&self) -> Option<Vec<(String, String)>> {None}

    fn apply_specific(&self, _page_map: &mut HashMap<&str, String>) {}
    
    fn status(&self) -> Option<(u16, &str)> {
        None
    }
    
    fn err_out(&self, err: String) {
       // eprintln!{"{err}"}
        print!{ "Status: {} Internal Server Error\r\n", 501 }
        print!{"Content-length: {}\r\n", err.len()}
        print! {"Content-type: text/plain\r\n\r\n{err}"}
    }

    fn show(&self) { // => Result<(), String>
        match self.main_load() { 
            Ok(page) => {
                if let Some(status) = self.status() {
                    print!{ "Status: {} {}\r\n", status.0, status.1 }
                }
                if let Some(extra_headers) = Self::get_extra(&self) {
                    for header in extra_headers {
                        print!{ "{}: {}\r\n", header.0, header.1 }
                    }
                }
                print! {"Content-type: {}\r\n\r\n", self.content_type()};
                let mut page_items = HashMap::from([
                    ("name", self.name()),
                    ("menu", form_nav(self.get_nav())),
                    ("theme", String::from("")),
                    ("path_info", std::env::var("PATH_INFO").unwrap_or("".to_string())),
                ]);
                self.apply_specific(&mut page_items);
                //eprintln! {"{page_items:?}"};
                print! {"{}", if page_items.is_empty() {page} else {template::interpolate(&page, &page_items)}}
            }
            Err(error) => Self::err_out(&self, error.to_string())
        }
    }
}

fn form_nav(items: Option<Vec<Menu>>) -> String {
    let mut res = String::from(r#"    <menu>"#);
    if let Some(items) = items {
        let mut _ident = 0;
        for item in items {
            match item {
                MenuBox {
                    title: item,
                    hint,
                    icon,
                } => {
                    _ident += 4;
                    res.push_str(&format! {r#"
        <menuitem>
            <a {1}>{2}{0}</a>
            <menu>
                "#, html_encode(&item), get_hint(&hint), get_img(&icon)})
                }
                MenuItem {
                    title: item,
                    link,
                    hint,
                    icon,
                    short
                } => {
                    res.push_str(&format! {r#"
                <menuitem>
                    <a href="{0}" {2}>{3}{1}{4}</a>
                </menuitem>
                "#, link, html_encode(&item), get_hint(&hint),
                          get_img(&icon), get_short(&short)})
                }
                MenuEnd => {
                    _ident -= 4;
                    res.push_str(&format! {r#"
            </menu>
        </menuitem>
"#})
                    }
                Separator => {
                }
            }
        }
    }
        // add js code ?
    res.push_str(r#"   </menu>"#);
    res
}

pub fn new_cookie_header(name: &String, value: &String, exparation: Option<SystemTime>) -> (String, String) {
    if let Some(time) = exparation {
        ("Set-Cookie".to_string(), format!{"{name}={value}; Expires={}", param::http_format_time(time)})
    } else {
        ("Set-Cookie".to_string(), format!{"{name}={value}"})
    }
}

pub fn html_encode(orig: &impl AsRef<str>) -> String {
    let chars = orig.as_ref(). chars();
    let mut res = String::from("");
    for c in chars {
        match c {
            '<' => res.push_str("&lt;"),
            '>' => res.push_str("&gt;"),
            '"' => res.push_str("&quot;"),
            '\'' => res.push_str("&#39;"),
            '&' => res.push_str("&amp;"),
            _ => res.push(c),
        }
    }
    res
}

/// it's an analog of URL component encode
pub fn url_encode(orig: &impl AsRef<str>) -> String {
    let chars = orig.as_ref().chars();
    let mut res = String::from("");
    let mut b = [0; 4];
    for c in chars {
        if (c as u32) < 256 && matches!(c as u8, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' |  b'-' | b'.' | b'_' | b'~') {
            res.push(c)
        } else {
            b.fill(0);
            c.encode_utf8(&mut b);
            res.push_str(&format!{"%{:02x}", b[0]});
            for i in 1..b.len() {
                if b[i]==0 {
                    //continue
                    break 
                }
                res.push_str(&format!{"%{:02x}", b[i]})
            }
        }
    }
    res
}

pub fn sanitize_path<'l>(path: &'l impl AsRef<Path>) -> Result<&'l Path, Box<dyn Error>> { // perhaps String is better
    let path = path.as_ref();
    for component in path.components() {
        if component == Component::ParentDir {
            return Err(".. isn't allowed in a path".into())
        }
    }
    Ok(path)
}

pub fn save_props(path: &Path, props: &HashMap<String, String>) -> io::Result<()> {
    let mut data =
        format! {"# property file on {}\n", &format_system_time(SystemTime::now())}.to_string();
    for (key, value) in props {
        data.push_str(&format! {"{}={}\n", key, value})
    }
    // since writing can happen concurrently
    // try create PROP.LOCK
    // if success, do write and then delete the LOCK
    fs::write(path, data)
}

pub fn get_file_modified<P: AsRef<Path>>(path: P) -> u64 { // in seconds
    match fs::metadata(path) {
        Ok(metadata) => if let Ok(time) = metadata.modified() {time.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()} else {0}
        _ => 0
    }
}

pub fn format_system_time(time: SystemTime) -> String {
    let dur = time.duration_since(SystemTime::UNIX_EPOCH).unwrap();
    // calc timezone
    let tz = get_local_timezone_offset();
    let (y, m, d, h, min, s, w) = get_datetime(1970, (dur.as_secs() as i64 + (tz as i64) * 60) as u64);
    format!("{m:0>2}-{d:0>2}-{y:0>2} {}, {h:0>2}:{min:0>2}:{s:0>2} {:03}{:02}",
         DAYS_OF_WEEK[w as usize], tz/60, tz%60)
}

pub fn list_files(path: impl AsRef<Path>, ext: &impl AsRef<str>) -> Vec<String> {
    let mut res: Vec<String> = Vec::new();
    let str_ext = ext.as_ref();
    if path.as_ref().is_dir() {
        for path in fs::read_dir(&path).into_iter().flatten().filter_map(|e| if let Ok(path) = e { Some(path.path()) } else {None}) {
            // no reason to dive for non dir path
            res.append(&mut list_files(path, ext))
        }
    } else {
        if let Some(curr_ext) = path.as_ref().extension() {
            let curr_ext = curr_ext.to_str().unwrap().to_string();
            if str_ext.contains(&curr_ext) {
                res.push(path.as_ref().to_str().unwrap().to_string())
            }
        }
    }
    res
}

fn get_hint(hint: &Option<&str>) -> String {
    if let Some(hint) = hint {
        format! {r#" alt="{0}" title="{0}""#, html_encode(&hint)}
    } else {
        String::new()
    }
}

fn get_img(icon: &Option<&str>) -> String {
    if let Some(icon) = icon {
        format! {"<img src=\"{}\">", icon}
    } else {
        String::new()
    }
}

fn get_short(short: &Option<&str>) -> String {
    if let Some(short) = short {
        format! {"<span style=\"float:right\">{0}</span>", html_encode(&short)}
    } else {
        "".to_string()
    }
}

pub fn is_git_covered(dir: &impl AsRef<Path>, home: &impl AsRef<Path> ) -> Option<String> {
    let git_dir = dir.as_ref().join(".git");
    if git_dir.is_dir() {
        Some(dir.as_ref().display().to_string())
    } else {
        if dir.as_ref() == home.as_ref() {
            None
        } else {
            if let Some(parent) = dir.as_ref().parent() {
                is_git_covered(&parent, home)
            } else {
                None
            }
        }
    } 
}
