use std::{collections::HashMap, env, io,};

#[derive(Debug)]
pub struct Param {
    params: HashMap<String, String>,
    cookies: HashMap<String, String>,
}

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
        if let Ok(query) = env::var("QUERY_STRING") {
            let parts = query.split("&");
            for part in parts {
                if let Some(keyval) = part.split_once("=")
                    && let Some(name) = res.url_comp_decode(keyval.0)
                    && let Some(val) = res.url_comp_decode(keyval.1)
                {
                    res.params.insert(name, val);
                }
            }
        }
        if let Ok(header_cookies) = env::var("HTTP_COOKIE") {
            let parts = header_cookies.split(";");
            for part in parts {
            // cookie name and value do not require any encoding besides of semicolon, comma, and blank, however most clients will use url or base64 
                if let Some(keyval) = part.split_once('=')
                    && let Some(name) = res.url_comp_decode(keyval.0)
                    && let Some(val) = res.url_comp_decode(keyval.1)
                {
                    res.cookies.insert(name.trim().to_string(), val);
                }
            }
        }
        //
        if let Ok(method) = env::var("REQUEST_METHOD")
            && method == "POST"
        {
            let mut user_input = String::new();
            let stdin = io::stdin();
            if let Ok(content_type) = env::var("CONTENT_TYPE")
                && content_type == "application/x-www-form-urlencoded"
            {
                // use simweb to process other types
                if let Ok(_ok) = stdin.read_line(&mut user_input) {
                    let parts = user_input.split("&");
                    for part in parts {
                        if let Some(keyval) = part.split_once('=')
                            && let Some(name) = res.url_comp_decode(keyval.0)
                            && let Some(val) = res.url_comp_decode(keyval.1)
                        {
                            res.params.insert(name, val);
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
        env::var("PATH_INFO").unwrap_or_else(|_e| String::new())
    }

    // TODO think of returning Cow
    pub fn url_comp_decode(&self, comp: &str) -> Option<String> {
        let mut res = Vec::with_capacity(comp.len());

        let mut chars = comp.chars();
        while let Some(c) = chars.next() {
            match c {
                '%' => {
                    let d1 = chars.next()? .to_digit(16)?;
                    let d2 = chars.next()?.to_digit(16)?;
                    res.push(((d1 << 4) + d2) as u8)
                }
                '+' => res.push(b' '),
                _ => res.push(if c.is_ascii() { c as u8 } else { return None }),
            }
        }
        String::from_utf8(res).ok()
    }
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
