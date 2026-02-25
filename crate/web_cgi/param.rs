use simtime::get_datetime;
use std::{collections::HashMap, env, io, time::SystemTime};

#[derive(Debug)]
pub struct Param {
    params: HashMap<String, String>,
    cookies: HashMap<String, String>,
}

pub const HTTP_DAYS_OF_WEEK: &[&str] = &["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];

pub const HTTP_MONTH: &[&str] = &[
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

impl Default for Param {
    fn default() -> Self {
        Self::new()
    }
}

impl Param {
    pub fn new() -> Self {
        let mut res = Param {
            params: HashMap::new(),
            cookies: HashMap::new(),
        };
        if let Ok(query) = env::var(String::from("QUERY_STRING")) {
            let parts = query.split("&");
            for part in parts {
                if let Some(keyval) = part.split_once("=") {
                    res.params
                        .insert(res.url_comp_decode(keyval.0), res.url_comp_decode(keyval.1));
                }
            }
        }
        if let Ok(header_cookies) = env::var(String::from("HTTP_COOKIE")) {
            let parts = header_cookies.split(";");
            for part in parts {
                if let Some(keyval) = part.split_once('=') {
                    res.cookies
                        .insert(keyval.0.trim().to_string(), keyval.1.to_string());
                }
            }
        }
        //
        if let Ok(method) = env::var(String::from("REQUEST_METHOD"))
            && method == "POST"
        {
            let mut user_input = String::new();
            let stdin = io::stdin();
            if let Ok(content_type) = env::var(String::from("CONTENT_TYPE"))
                && content_type == "application/x-www-form-urlencoded"
            {
                // use simweb to process other types
                if let Ok(_ok) = stdin.read_line(&mut user_input) {
                    let parts = user_input.split("&");
                    for part in parts {
                        if let Some(keyval) = part.split_once('=') {
                            res.params.insert(
                                res.url_comp_decode(keyval.0),
                                res.url_comp_decode(keyval.1),
                            );
                        }
                    }
                }
            }
        }

        res
    }

    pub fn param(&self, key: impl AsRef<str>) -> Option<String> {
        self.params.get(key.as_ref()).cloned() // probably better to return as Option<&String> without using clone
    }

    pub fn cookie(&self, key: impl AsRef<str>) -> Option<String> {
        self.cookies.get(key.as_ref()).cloned() // probably better to return as Option<&String> without using clone
    }

    pub fn path_info(&self) -> String {
        if let Ok(pi) = env::var(String::from("PATH_INFO")) {
            pi.to_string()
        } else {
            // since path info is never an empty string
            "".to_string()
        }
    }

    pub fn url_comp_decode(&self, comp: &str) -> String {
        let mut res = Vec::with_capacity(256);

        let mut chars = comp.chars();
        while let Some(c) = chars.next() {
            match c {
                '%' => {
                    if let Some(c1) = chars.next() {
                        let d1 = c1.to_digit(16).unwrap();
                        if let Some(c2) = chars.next() {
                            let d2 = c2.to_digit(16).unwrap();
                            res.push(((d1 << 4) + d2) as u8)
                        }
                    }
                }
                '+' => res.push(b' '),
                _ => res.push(if c.is_ascii() { c as u8 } else { b'?' }),
            }
        }
        String::from_utf8_lossy(&res).to_string()
    }
}

pub fn http_format_time(time: SystemTime) -> String {
    let dur = time.duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let (y, m, d, h, min, s, w) = get_datetime(1970, dur.as_secs());
    format!(
        "{}, {d:0>2} {} {y:0>2} {h:0>2}:{min:0>2}:{s:0>2} GMT",
        HTTP_DAYS_OF_WEEK[w as usize],
        HTTP_MONTH[(m - 1) as usize]
    )
}

pub fn to_web_separator(mut path: String) -> String {
    unsafe {
        let path_vec: &mut [u8] = path.as_bytes_mut();

        for c in path_vec {
            if *c == b'\\' {
                *c = b'/';
            }
        }
    }
    path
}
