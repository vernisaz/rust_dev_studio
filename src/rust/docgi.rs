extern crate simtime;
extern crate web_cgi as web;
extern crate simweb;
extern crate simtpool;
extern crate simran;
extern crate simcfg;
extern crate simjson;

use std::{collections::HashMap,
        fs::{self, create_dir_all, read_dir, read_to_string, remove_file,write,File},
        io::{self,Write},
        path::{Path,PathBuf},
        process::{Command,Stdio},
        sync::{Arc, Mutex},thread,
        env, error::Error,time::UNIX_EPOCH, ops::Not, ffi::OsStr,
        };

mod crossref;
mod search;
mod config;

use simweb::{WebError,url_encode, json_encode};
use crossref::{RefType,Reference};
use web::{get_file_modified, sanitize_path, Menu,
    save_props, PageOps, param};
use simtpool::ThreadPool;
use config::{SETTINGS_PREF,read_props};

macro_rules! eprintln {
    ($($rest:tt)*) => {
        #[cfg(feature = "quiet")]
        std::eprintln!($($rest)*)
    }
}

const VERSION: &str = env!("VERSION");

fn main() {
    if let Err(e) = inner_main() {
        let page = PageStuff {
            content: format!{"Err: {e:#?}"}
        }; PageOps::err_out(&page, page.content.clone())
    }
}

fn inner_main() -> Result<(), Box<dyn std::error::Error>> {
    let params = web::Param::new();
    let config = config::Config::new();
    let page: Box<dyn PageOps> = match params.param("mode").as_deref() {
        None => match params.param("id") {
            None => Box::new(Redirect{session: params.param("session"),}),
            Some(_) =>
                Box::new(PageFile {
                    file_name: "main.html".to_string(),
                    session: params.param("session"),
                    home: config.config_dir.display().to_string(),
                    id: params.param("id"),
                })
            },
        Some("tree") => match config.get_project_home(&params.param("session")) {
            Some(path) => Box::new(JsonData {
                file: PageFile {
                    file_name: config
                        .to_real_path(&path, None),
                    ..Default::default()
                },
            }),
            _ => Box::new(JsonStuff {
                json: r#"{"name":"No correct project HOME is set yet", "type":"file"}"#.to_string(),
                name: "noproject".to_string()
             }),
        }
        Some("editor-file") => {
            let project_home = config.get_project_home(&params.param("session")).ok_or(Box::<dyn Error>::from("project home misconfiguration"))?;
            let in_project_path = params.param("path").ok_or(WebError{cause:None, reason: "no parameter path".to_string()})?;
            sanitize_path(&in_project_path)?; 
            let file = params.param("name").ok_or(WebError{cause:None, reason: "no file name".to_string()})?;
            sanitize_path(&file)?; 
            let file_path = PathBuf::from(&config.to_real_path(&project_home, Some(&in_project_path)));
            let modified = get_file_modified(&file_path);
            let edit: String = read_to_string(&file_path)?;
            Box::new(PageFrag { fragment: PageStuff {content: format!(r#"{{"modified":{modified}, "name":"{}", "path":"{}", "content": "{}"}}"#,
                json_encode(&file), json_encode(&in_project_path), json_encode(&edit))}, params,})
        }
        Some("save") => {
            if let Ok(met) = env::var("REQUEST_METHOD") && met == "POST" {
                let sub_path = &params.param("name").ok_or(WebError{cause:None, reason: "No parameter 'name'".to_string()})?; 
                //eprintln!("name:{sub_path}");
                let file_path =
                    config.to_real_path(config.get_project_home(&params.param("session")).ok_or(Box::<dyn Error>::from("project home misconfiguration"))?, Some(sub_path));
                let file_path = sanitize_path(&file_path)?; 
                let modified = get_file_modified(file_path);
                let remote_modifiled = &params
                    .param("modified")
                    .unwrap_or_else(||"0".to_string())
                    .parse::<u64>()
                    .unwrap_or_default();
                if modified <= *remote_modifiled {
                    if let Some(data) = params.param("data") {
                        if modified == 0 {
                            let _ = Path::new(&file_path).parent().and_then(|parent| create_dir_all(parent).ok());
                        }
                        let mut write_done = false;
                        let adata = Arc::new(data);
                        let data_for_1 = Arc::clone(&adata);
                        let data_for_2 = Arc::clone(&adata);
                        if file_path.extension() == Some(OsStr::new("rs")) {
                            let settings = config.get_config_path(&params.param("session"), SETTINGS_PREF, "prop");
                            let props = read_props(&settings);
                            if let Some(value) = props.get("format_on_save") && value == "yes" &&
                                let Some(json) = props.get("proj_conf") && let json = simjson::parse(json) &&
                                let Some(format_src) = simjson::get_path_as_text(&json,&"format_src") && !format_src.is_empty() {
                                let dir = config.to_real_path(config.get_project_home(&params.param("session")).unwrap_or_default(), None);
                                let mut parameters = format_src.split_whitespace();
                                let prog_name = parameters.next().unwrap();
                                let args : Vec<_> = parameters.collect();
                                if let Ok(mut p) = Command::new(prog_name).args(args).current_dir(&dir)
                                    .stdout(Stdio::piped())
                                    .stdin(Stdio::piped()).spawn() {
                                    //let value = data.clone();
                                    let mut p_stdin = p.stdin.take().unwrap();
                                    thread::spawn(move || {
                                        let _ = p_stdin.write_all(data_for_1.as_bytes());
                                    });
                                    if let Some(ref mut stdout) = p.stdout && let Ok(mut file_to_write) = File::create(file_path) &&
                                        let Ok(_len) = io::copy(stdout, &mut file_to_write) {
                                            write_done = true
                                    }
                                    if let Ok(status) = p.wait() {
                                        write_done = write_done && status.success()
                                    } else {
                                        write_done = false
                                    }
                                }
                            }
                        }
                        if !write_done {
                            match write(file_path, &*data_for_2) {
                                Ok(()) => {
                                    Box::new(PageStuff {
                                        content: format! {"Ok {}", get_file_modified(file_path)}
                                    })
                                }
                                Err(err)=> Box::new(PageStuff {
                                    content: format!("Err: {err} for {file_path:?}"),
                                })
                            }
                        } else {
                            Box::new(PageStuff {
                                    content: format! {"Ok {}", get_file_modified(file_path)}
                                })
                        }
                    } else {
                        Box::new(PageStuff {
                            content: "Err: No file content has provided".to_string(),
                        })
                    }
                } else {
                    Box::new(PageStuff {
                        content: format!{"Err: file is too old {modified} vs {remote_modifiled}"}
                    })
                }
            } else {
                Box::new(PageStuff {
                    content: "Err : not a POST".to_string(),
                })
            } 
        }
        Some("settings-project") => {
            let settings = config.get_config_path(&params.param("session"), SETTINGS_PREF, "prop");
            let settings_file = sanitize_path(&settings)?;
            Box::new(JsonSettings {
                file: PageFile {
                    file_name: settings_file.display().to_string(),
                    ..Default::default()
                },
                home_len: (config.workspace_dir.display().to_string().len()+1) as _,
            })
        }
        Some("save-settings-project") => {
            if let Ok(met) = env::var("REQUEST_METHOD") && met == "POST" {
                let settings = config.get_config_path(&params.param("session"), SETTINGS_PREF, "prop");
                let settings_path = sanitize_path(&settings)?;
                let mut props = read_props(settings_path);
                let mut set_value = |key| match params.param(&key) {
                    Some(val) => props.insert(key, val),
                    None => None
                };
                
                if  let Some(proj_dir ) =  params.param("project_home") {
                    sanitize_path(&proj_dir)?;
                    let real_dir = config.to_real_path(&proj_dir, None);
                    let real_dir = Path::new(&real_dir);
                    if real_dir.exists().not() {
                        // create dir if non existent (too many directories attack possible)
                        fs::create_dir_all(real_dir)?;
                    } else if real_dir.is_file() {
                        return Err("a file specified instead of a directory".into())
                    }
                }
                for key in  ["project_home", "theme", "autosave", "projectnp", "user", "persist_tabs", "proj_conf", "ai_server_url", "colapsed_dirs", "src_dir", "format_on_save"] {
                    set_value(key.to_string());
                }
                // TOOO there is a race condition which is currently ignored
                let _ = save_props(settings_path, &props);
                Box::new(PageStuff {
                    content: "Ok".to_string(),
                })
            } else {
                Box::new(PageStuff {
                    content: "Err : not a POST".to_string(),
                })
            } 
        }
        Some("dir-list") => {
            // list of dirs in
            let dir = config.name_to_path(params.param("name")).ok_or(Box::<dyn Error>::from("project home misconfiguration"))?;
            eprintln! {"Project dir: {:?}", &dir};
            Box::new(JsonDirs {
                file: PageFile {
                    file_name: dir,
                    ..Default::default()
                },
            })
        }
        Some("project-dir-list") => {
            // list of dirs in
            let dir = config.to_real_path(config.get_project_home(&params.param("session")).unwrap_or_default(), None);
            //eprintln! {"Project conn dir: {:?}", &dir};
            Box::new(JsonProj {
                file: PageFile {
                    file_name: dir,
                    ..Default::default()
                },
            })
        }
        Some("del-project") => if let Ok(met) = env::var("REQUEST_METHOD") && met == "POST" {
            let proj = params.param("project");
            if proj.is_none() {
                return Err("no project param".into())
            };
            let del_fil  = |file| -> io::Result<()> {
                remove_file(file)?;
                Ok(())
            };
            let settings = config.get_config_path(&proj, SETTINGS_PREF, "prop");
            let settings_path = sanitize_path(&settings)?;
            let mut all_fine = true;
            all_fine &= del_fil(settings_path).is_ok();
            let np = config.get_config_path(&proj, "notepad", "txt");
            let np_path = sanitize_path(&np)?; // further sanitizing is not required
            let _ = del_fil(np_path).is_ok();
            let tabs = config.get_config_path(&proj, "tabs", "sto");
            let _ = del_fil(&tabs).is_ok();
            let bm = config.get_config_path(&proj, "bookmark", "json");
            let _ = del_fil(&bm).is_ok();
            match all_fine {
                true => Box::new(PageStuff {
                        content: "Ok".to_string(),
                    }),
                _ => Box::new(PageStuff {
                    content: "Err : Some project files weren't deleted".to_string(),
                })
            }
        } else {
            Box::new(PageStuff {
                content: "Err : not a POST".to_string(),
            })
        }
        Some("info-about") => Box::new(PageFrag { fragment: PageStuff {
            content: format!{r#"{{"version":"{VERSION}", "server": "{}", "author": "D Rogatkin"}}"#,
                json_encode(&std::env::var(String::from("SERVER_SOFTWARE")).unwrap_or_else(|_| "Unknown server software".to_owned()))
            }}, params,
        }),
        Some("session-list") => { // TODO rename to project-list
            // list of dirs in
            Box::new(JsonSess {
                file: PageFile {
                    file_name: config.config_dir.display().to_string(),
                    ..Default::default()
                },
            })
        }
        Some("savenp") => {
            if let Ok(met) = env::var("REQUEST_METHOD") && met == "POST" {
                let settings = config.get_config_path(&params.param("session"), SETTINGS_PREF, "prop");
                let settings_path = sanitize_path(&settings)?;
                let props = read_props(settings_path);
                let spec_name =
                match props.get("projectnp") {
                    Some(spec) if spec == "yes" => params.param("session"), // maybe better 'true'?
                    _ => None,
                } ;
                let np = config.get_config_path(&spec_name, "notepad", "txt");
                let np_path = sanitize_path(&np)?;
                
                if let Some(data) = &params.param("name") {
                    write(np_path, data)?;
                    Box::new(PageStuff {
                        content: "Ok".to_string(),
                    })
                } else {
                    Box::new(PageStuff {
                        content: "Err : no notepad".to_string(),
                    })
                }
            } else {
                Box::new(PageStuff {
                    content: "Err : not a POST".to_string(),
                })
            }
        }
        Some("delete") => {
            if let Ok(met) = env::var("REQUEST_METHOD") && met == "POST" {
                let file = config.to_real_path(
                    &config.get_project_home(&params.param("session")).ok_or(Box::<dyn Error>::from("project home misconfiguration"))?, 
                    params.param("name").as_ref(), // may require param::adjust_separator(
                );
                let file = sanitize_path(&file)?;
                eprintln! {"Project file to del: {:?}", &file};
                remove_file(file)?;
                Box::new(PageStuff {
                    content: "Ok".to_string(),
                })
            } else {
                Box::new(PageStuff {
                    content: "Err : not a POST".to_string(),
                })
            }
        }
        Some("loadnp") => {
            let settings = config.get_config_path(&params.param("session"), SETTINGS_PREF, "prop");
            let props = read_props(sanitize_path(&settings)?);
            let spec_name =
                match props.get("projectnp") {
                    Some(spec) if spec == "yes" => params.param("session"), // same question - true?
                    _ => None,
                } ;
            let np = config.get_config_path(&spec_name, "notepad", "txt");
            let np_path = sanitize_path(&np)?;
            Box::new(PageStuff {
                content: read_to_string(np_path).unwrap_or_else(|_| "".to_string()),
            })
        }
        Some("vcs-list") => {
            let dir =
                config.to_real_path(config.get_project_home(&params.param("session")).unwrap_or_default(), None);
            eprintln! {"VCS dir: {:?}", &dir};
            Box::new(JsonVCS {
                dir: PageFile {
                    file_name: dir,
                    ..Default::default()
                },
                home: config.config_dir.display().to_string()
            })
        }
        Some("vcs-commit") => {
            if let Ok(met) = env::var("REQUEST_METHOD") && met == "POST" {
                let dir = config.to_real_path(config.get_project_home(&params.param("session")).unwrap_or_default(), None);
                if let Some(dir) = web::is_git_covered(&dir, &config.workspace_dir.display().to_string())
                {
                    let mut result_oper: Result<(), String> = Ok(());
                    // git rm --cached file
                    // git reset file
                    let reset_list = params.param("cache").unwrap_or_default();
                    let mut files = reset_list
                        .split('\t')
                        .filter(|s| s.is_empty().not())
                        .peekable();
                    if files.peek().is_some() {
                        let output = Command::new("git")
                            .arg("reset")
                            .args(files)
                            .current_dir(&dir)
                            .output()?;
                        if output.status.success().not() {
                            #[allow(unused)]
                            let stderr = String::from_utf8(output.stderr)?;
                            eprintln! {"git reset executed err for {:?}: {stderr}", output.status}
                            result_oper = Err(stderr)
                        }
                    }
                    if result_oper.is_ok() {
                        // git commit -m <msg>
                        let commit_list = params.param("name").unwrap_or_default();
    
                        let comment = params.param("comment").unwrap_or_default();
                        eprintln! {"to commit: {commit_list} for {comment}"};
                        let files: Vec<_> = commit_list
                            .split('\t')
                            .filter(|s| s.is_empty().not()).collect();
                        
                        if !files.is_empty() {
                            let output = Command::new("git")
                                .arg("add")
                                .args(files)
                                .current_dir(&dir)
                                .output()?;
                            if !output.status.success() {
                                #[allow(unused)]
                                let stderr = String::from_utf8(output.stderr)?;
                                eprintln! {"git add executed err for {:?}: {stderr}", output.status}
                                result_oper = Err(stderr)
                            }
                        }
                        if result_oper.is_ok() {
                            let mut command = Command::new("git");
                            command
                                .arg("commit")
                                .arg("-m")
                                .arg(&comment)
                                .env("HOME", config.to_real_path("", None))
                                .current_dir(&dir);
                            let settings = config.get_config_path(&params.param("session"), SETTINGS_PREF, "prop");
                            let settings_file = sanitize_path(&settings)?;
                            let props = read_props(settings_file);
                            let user = props.get("user");
                            if let Some(user) = user {
                                let author = format! {r#"--author={user}"#};
                                command.arg(&author);
                            }
                            let output = command.output()?;
                            if !output.status.success() {
                                let mut stderr = String::from_utf8(output.stderr) ?;  // stdout may have too verbose explanation
                                if stderr.is_empty() {
                                    stderr = String::from("nothing to commit")
                                }
                                eprintln! {"git commit executed err for {:?} : {stderr}", output.status}
                                result_oper = Err(stderr)
                            } else {
                                #[allow(unused)]
                                let stdout = String::from_utf8(output.stdout)?;
                                eprintln! {"git commit success {stdout}"}
                            }
                        } else {
                            result_oper = Err("nothing to commit".to_string());
                        }
                    }
                    Box::new(PageStuff {
                        content: match result_oper {
                            Ok(()) => String::from("Ok"),
                            Err(msg) => format!{"Err : {msg}"}
                        },
                    })
                } else {
                    Box::new(PageStuff {
                        content: "Err : not under git".to_string(),
                    })
                }
            } else {
                Box::new(PageStuff {
                    content: "Err : not a POST".to_string(),
                })
            }
        }
        Some("vcs-restore") => {
            // git checkout -- <file>
            if let Ok(met) = env::var("REQUEST_METHOD") && met == "POST" {
                // TODO make it the fn exec_git(git_act: impl AsRef<str>)) -> Result<(), String>
                let dir = config.to_real_path(config.get_project_home(&params.param("session")).unwrap_or_default(), None);
                if let Some(file) = params.param("name") {
                    let output = Command::new("git")
                        .arg("restore")
                        .arg(file)
                        .current_dir(&dir)
                        .output()?;
                        
                    if output.status.success() {
                        Box::new(PageStuff {
                            content: "Ok".to_string(),
                        })
                    } else {
                        #[allow(unused)]
                        let stderr = String::from_utf8(output.stderr)?;
                        eprintln! {"git restore executed err for {:?}: {stderr}", output.status};
                        Box::new(PageStuff {
                            content: format! {"Err : restore {stderr}"}.to_string(),
                        })
                    }
                } else {
                    Box::new(PageStuff {
                        content: "Err : no file".to_string(),
                    })
                }
            } else {
                Box::new(PageStuff {
                    content: "Err : not a POST".to_string(),
                })
            }
        }
        Some("vcs-stage") => {
            // git add <file>
            if let Ok(met) = env::var("REQUEST_METHOD") && met == "POST" {
                let dir = config.to_real_path(config.get_project_home(&params.param("session")).unwrap_or_default(), None);
                if let Some(file) = params.param("name") {
                    let output = Command::new("git")
                        .arg("add")
                        .arg(file)
                        .current_dir(&dir)
                        .output()?;
                    if output.status.success() {
                        Box::new(PageStuff {
                            content: "Ok".to_string(),
                        })
                    } else {
                        #[allow(unused)]
                        let stderr = String::from_utf8(output.stderr)?;
                        eprintln! {"git add executed err for {:?}: {stderr}", output.status};
                        Box::new(PageStuff {
                            content: format! {"Err : add {stderr}"}.to_string(),
                        })
                    }
                } else {
                    Box::new(PageStuff {
                        content: "Err : no file".to_string(),
                    })
                }
            } else {
                Box::new(PageStuff {
                    content: "Err : not a POST".to_string(),
                })
            }
        }
        Some("load-persist-tab") => {
            let tabs = config.get_config_path(&params.param("session"), "tabs", "sto");
            let tabs_file = sanitize_path(&tabs)?;
            match read_to_string(tabs_file) {
                Ok(tabs) => {
                    let tab_paths = tabs.split("\t");
                    let mut res = String::from("[");
                    res.push_str(&tab_paths.filter_map(|t| if t.is_empty() {None} else {Some(format!(r#""{}""#, &json_encode(t)))}).collect::<Vec<_>>().join(","));
                    res.push(']');
                    Box::new(PageStuff { content: res })
                }
                #[allow(unused)]
                Err(err) => { eprintln!{"no tabs {err}"}
                    Box::new(PageStuff { content: "[]".to_owned() })
                }
            }
        }
        Some("persist-tab") => {
            if let Ok(met) = env::var("REQUEST_METHOD") && met == "POST" {
                let tabs = config.get_config_path(&params.param("session"), "tabs", "sto");
                let tabs_file = sanitize_path(&tabs)?;
                params.param("tabs").and_then(|v| fs::write(tabs_file, v).ok());
                Box::new(PageStuff { content: "Ok".to_string() })
            } else {
                Box::new(PageStuff { content: "Err: not a POST".to_string() })
            }
        }
        Some("crossref-list") => {
            // get list of all .rs files of the project
            let mut use_pnts  = HashMap::new();
            let mut total_refs = vec![];
            
            let dir = config.to_real_path(config.get_project_home(&params.param("session")).unwrap_or_default(), None);
            let dir_len = dir.len();
            let rs_files = web::list_files(&dir, &".rs");
            //eprintln! {".rs: {rs_files:?}"}
            #[cfg(feature = "quiet")]
            let mut total_fun = 0;
            let mut json_res = String::from("[");
            for file in rs_files {
                #[cfg(dbg_ref)]
                if !&file.ends_with("test.rs") { // put actuall testing file name
                    continue
                } 
                let xrefs = crossref::scan_file(&file);
                #[cfg(feature = "quiet")]
                {total_fun += &xrefs.len()}
                //eprintln! {"XRef of {file}: {xrefs:?}"}
                for entry in &xrefs {
                    match entry.type_of_use {
                    // pass entire codebase to build use points and then second pass to fill json data
                        RefType::Access => {
                           // eprintln!{"added access to {}",&entry.name}
                            use_pnts.entry(entry.name.clone()).or_insert(vec![]).push(entry.clone());
                            continue
                        }
                        RefType::Function => total_refs.push(entry.clone()),
                        _ => continue
                    }
                }
            }
            // fill json now
            for entry in total_refs {
                if !entry.src.starts_with(&dir) || entry.name.is_empty() {
                    continue
                }
                if json_res.len() > 1 {
                    json_res.push(',')
                }
                let mut fn_ref = String::from("{\"name\":\"");
                fn_ref.push_str(&json_encode(&entry.name));
                fn_ref.push_str("\",\"path\":\"");
                #[cfg(any(unix, target_os = "redox"))]
                let rel_loc = entry.src[dir_len+1..].to_owned();
                #[cfg(target_os = "windows")]
                let rel_loc = param::to_web_separator(entry.src[dir_len+1..].to_owned());
                fn_ref.push_str(&json_encode(&rel_loc));
                fn_ref.push('"');
                if let Some(scope) = &entry.scope {
                    let data_name = match &scope.name_for {
                        None => "".to_string(),
                        Some(name) => name.to_string()
                    };
                    fn_ref.push_str(&format!{r#","trait":"{}", "data":"{}""#, scope.name, data_name})
                }
                let refs_to = match use_pnts.get(&json_encode(&entry.name).to_string()) {
                    None => String::new(),
                    Some(vec_val) => refs_to_json(vec_val, dir_len)
                };
                fn_ref.push_str(&format!{r#","line":{}, "col":{}, "use":[{}]}}"#,
                entry.line, entry.column,refs_to}); // probably format an entire entry
                json_res.push_str(&fn_ref)
            }
        
            json_res.push(']');
            eprintln! {"Xrefs JSON: {json_res} entries {total_fun}"}
            Box::new(JsonStuff {
                json: json_res,
                name: "references".to_string()
            })
        }
        Some("search-list") => {
            let shared = Arc::new(Mutex::new(String::from("[")));
            let tp = ThreadPool::new(3);
            if let Some(string) = params.param("name") {
                let dir = config.to_real_path(config.get_project_home(&params.param("session")).unwrap_or_default(), None);
                let dir_len = (dir).len();
                eprintln! {"Search for {string} in {dir:?}"}
                let exts = env!("SEARCH_EXTS");
            
                let files = web::list_files(&dir, &exts); // faster to pass an array of exts
                //eprintln! {"...in {} files", files.len()}
                for file in files {
                    let res = Arc::clone(&shared);
                    let string = string.clone();
                    tp.execute(move ||
                        // get file content in string in rust
                        if let Ok(content) = &fs::read_to_string(&file) 
                            && let Some((line,col)) = search::boyer_moore_search(content, &string) {
                               eprintln! {"found in {file}"}
                               let mut json_res = res.lock().unwrap(); // if let Ok(...)
                               if json_res.len() > 1 {
                                   json_res.push(',')
                               }
                               let name = Path::new(&file).file_name().unwrap().to_str().unwrap().to_owned();
                               #[cfg(any(unix, target_os = "redox"))]
                               let path = file [dir_len+1..].to_owned();
                               #[cfg(target_os = "windows")]
                               let path = param::to_web_separator(file [dir_len+1..].to_owned());
                               json_res.push_str(&format!{r#"{{"path":"{}","line":{line},"col":{col},"name":"{}"}}"#,
                                  &json_encode(&path), &json_encode(&name)})
                        }
                    );
                }
            }
            drop(tp);
            let res = Arc::clone(&shared);
            let mut json_res = res.lock().unwrap();
            json_res.push(']');
            Box::new(JsonStuff {
                json: json_res.to_string(),
                name: "findings".to_string()
            })
        }
        Some("save-bookmark") => {
            if let Ok(met) = env::var("REQUEST_METHOD") && met == "POST" {
                let bm = config.get_config_path(&params.param("session"), "bookmark", "json");
                let bm_file = sanitize_path(&bm)?;
                fs::write(bm_file, params.param("bookmarks").unwrap_or_default())?;
                Box::new(PageStuff { content: "Ok".to_string() })
            } else {
                Box::new(PageStuff { content: "Err: not a POST".to_string() })
            }
        }
        Some("load-bookmark") => {
            let bm = config.get_config_path(&params.param("session"), "bookmark", "json");
            let bm_file = sanitize_path(&bm)?;
            if let Ok(bookmarks) = fs::read_to_string(bm_file) {
                Box::new(JsonStuff {
                    json: bookmarks,
                    name: "bookmarks".to_string()
                })
            } else {
                Box::new(JsonStuff {
                    json: "[]".to_string(),
                    name: "empty_bookmarks".to_string()
                })
            }
        }
        Some("format") => {
            if let Ok(met) = env::var("REQUEST_METHOD") && met == "POST" {
                let settings = config.get_config_path(&params.param("session"), SETTINGS_PREF, "prop");
                let props = read_props(sanitize_path(&settings)?);
                
                let json = simjson::parse(props.get("proj_conf").ok_or("not configured formatting")?);
                
                let dir = config.to_real_path(config.get_project_home(&params.param("session")).unwrap_or_default(), None);
                if let Some(file) = params.param("name") && let Some(prog_name) = simjson::get_path_as_text(&json,&"format_src")
                    && !prog_name.is_empty() {
                    let mut parameters = prog_name.split_whitespace();
                    let prog_name = parameters.next().unwrap();
                    let mut args : Vec<_> = parameters.collect();
                    args . push(&file);
                    let output = Command::new(prog_name)
                        .args(args)
                        .current_dir(&dir)
                        .output()?;
                    if output.status.success() {
                        Box::new(PageStuff {
                            content: "Ok".to_string(),
                        })
                    } else {
                        #[allow(unused)]
                        let stderr = String::from_utf8(output.stderr)?;
                        eprintln! {"error code {:?} in formatting: {stderr}", output.status};
                        Box::new(PageStuff {
                            content: format! {"Err : format {stderr}"}.to_string(),
                        })
                    }
                } else {
                    Box::new(PageStuff {
                        content: "Err : no file or a formatter wasn't configured".to_string(),
                    })
                }
            } else {
                Box::new(PageStuff {
                    content: "Err : not a POST".to_string(),
                })
            }
        }
        Some(mode) => Box::new(PageStuffE {
            content: format! {r#"Err: The mode &quot;{mode}&quot; is not implemented in ver {VERSION}."#},
        }),
    };
    page.show();
    Ok(())
}

#[derive(Debug)]
pub struct PageStuff {
    content: String,
}

#[derive(Debug)]
pub struct PageStuffE {
    content: String,
}

type JsonStr = String;
    
pub struct JsonStuff {
    json: JsonStr,
    name: String
}

#[derive(Debug, Default)]
pub struct PageFile {
    file_name: String,
    session: Option<String>,
    id: Option<String>,
    home: String,
}

#[derive(Debug)]
pub struct JsonData {
    file: PageFile,
}

#[derive(Debug)]
pub struct JsonSess {
    file: PageFile,
}

#[derive(Debug)]
pub struct JsonDirs {
    file: PageFile,
}

#[derive(Debug)]
pub struct JsonProj {
    file: PageFile,
}

#[derive(Debug)]
pub struct PageFrag {
    fragment: PageStuff,
    params: param::Param,
}

#[derive(Debug)]
pub struct JsonSettings {
    file: PageFile,
    home_len: u16,
}

#[derive(Debug)]
pub struct JsonVCS {
    dir: PageFile,
    home: String
}

#[derive(Debug)]
pub struct Redirect {
    session: Option<String>,
}

macro_rules! json_ret{
  () => { fn content_type(&self) -> String {
        "application/json".to_string()
    } }
}

macro_rules! name_of{
  ($name:literal) => { fn name(&self) -> String {
        $name.to_string()
    } }
}

impl PageOps for JsonSettings {
    fn main_load(&self) -> Result<String, Box<dyn Error>> {
        let props = read_props(&PathBuf::from(&self.file.file_name));
        let binding = String::new();
        let project_home = props.get("project_home").unwrap_or(&binding);
        let light = "light".to_string();
        let theme = props.get("theme").unwrap_or(&light);
        let no = "no".to_string();
        let f_no = || &no;
        let f_binding = || &binding;
        let autosave = props.get("autosave").unwrap_or(&no); // == "yes";
        let projectnp = props.get("projectnp").unwrap_or_else(f_no);
        let format_on_save = props.get("format_on_save").unwrap_or_else(f_no) == "yes";
        let user = props.get("user").unwrap_or(&binding);
        let persist_tabs = props.get("persist_tabs").unwrap_or(&no);
        let home_len = self.home_len;
        let empty_obj = "{}".to_string();
        let proj_conf = props.get("proj_conf").unwrap_or(&empty_obj);
        let ai_url = props.get("ai_server_url").unwrap_or(&binding);
        let colapsed_dirs = props.get("colapsed_dirs").unwrap_or_else(f_binding);
        let src_dir = props.get("src_dir").unwrap_or(&binding);
        let project_home = json_encode(project_home);
        Ok(format! {r#"{{"project_home":"{project_home}", "theme":"{theme}", "autosave" : "{autosave}",
            "projectnp":"{projectnp}", "user":"{2}", "persist_tabs":"{persist_tabs}",
            "home_len":{home_len}, "proj_conf":{proj_conf}, "ai_server_url":"{}",
            "colapsed_dirs":"{}", "src_dir":"{3}", "format_on_save":{format_on_save}}}"#,
            &json_encode(ai_url), &json_encode(colapsed_dirs), &json_encode(user), json_encode(src_dir)})
    }

    json_ret!{}

    name_of!{"JSON"}
}

impl PageOps for JsonDirs {
    fn main_load(&self) -> Result<String, Box<dyn Error>> {
        let mut dirs: Vec<_> = read_dir(&self.file.file_name)
            .map_err(|e| format!{"can't read {} because {e:?}", self.file.file_name})?
            .filter_map(|f| if f.as_ref().map(|f| f.file_type().map(|t| t.is_dir()).unwrap_or(false)
                    && f.file_name().into_string().map(|n| n != ".git").unwrap_or(false)).unwrap_or(false)
                       {Some(f.unwrap().file_name().to_string_lossy().to_string())} else {None})
            .collect();
        dirs.sort(); // TODO reconsider do sorting on a client, was sort_by_key
        Ok("[".to_owned() + &dirs.into_iter().map(|ref curr| "\"".to_owned() +
            &json_encode(curr) + "\"").collect::<Vec<_>>().join(", ") + "]"
        )
    }

    json_ret!{}

    name_of!{"JSON"}
}

impl PageOps for JsonProj {
    fn main_load(&self) -> Result<String, Box<dyn Error>> {
        let mut res = "[".to_string();
        if let Ok(data) = recurse_dirs(Path::new(&self.file.file_name), None) {
            res.push_str(&data);
        }
        res.push(']');
        //eprintln!("{res}");
        Ok(res)
    }

    json_ret!{}

    name_of!{"name"}
}

impl PageOps for JsonSess {
    fn main_load(&self) -> Result<String, Box<dyn Error>> {
        let paths = read_dir(&self.file.file_name).map_err(|err| format!("Directory {} can't be read: {err}", &self.file.file_name))?;
        //eprintln!("looking in:{}", &self.file.file_name);
        let mut res = paths.fold("[".to_string(), |mut accum, path| {
            let file_name = path.unwrap().file_name().to_str().unwrap().to_owned();
            if file_name.starts_with(SETTINGS_PREF) && file_name.ends_with(".prop") {
                let mut session_name = &file_name[SETTINGS_PREF.len()..file_name.len() - ".prop".len ()];
                if !session_name.is_empty() {
                    if session_name[0..1] == *"-" {
                        session_name = &session_name[1..]
                    } else {
                        return accum
                    }
                }
                if accum.len() > 1 {
                    accum.push(',')
                }
                accum.push('"');
                if !session_name.is_empty() {
                    accum.push_str(&json_encode(session_name));
                }
                accum.push('"')
            }
            accum
        });
        res.push(']');
        //eprintln!("sessions:{res}");
        Ok(res)
    }

    json_ret!{}

    name_of!{"name"}
}

impl PageOps for JsonData {
    fn main_load(&self) -> Result<String, Box<dyn Error>> {
        recurse_files(Path::new(&self.file.file_name)).map_err(|err| format!("Directory {} can't be read: {err}", &self.file.file_name).into())
    }

    json_ret!{}

    name_of!{"JSON"}
}

impl PageOps for JsonVCS {
    fn main_load(&self) -> Result<String, Box<dyn Error>> {
        if let Some(dir) = web::is_git_covered(&self.dir.file_name, &self.home)
        {
            let output = Command::new("git")
                .arg("status")
                .arg("--porcelain")
                .current_dir(&dir)
                .output().map_err(|e| e.to_string())?;
            let mut res = "[".to_string();
            if output.status.success() {
                let out = String::from_utf8_lossy(&output.stdout);

                let status_arr = out.split('\n');
                for entry in status_arr {
                    if entry.len() > 3 {
                        if res.len() > 1 {
                            res.push(',');
                        }
                        res.push_str("{\"path\":\"");
                        let status_curr = &entry[..1];
                        let status_prev = &entry[1..2];
                        let path = 
                        if entry.chars().nth(3).unwrap() == '"' && entry.char_indices().nth_back(0).unwrap().1 == '"' {
                            &entry[4..=entry.char_indices().nth_back(1).unwrap().0]
                        } else {
                            &entry[3..]
                        };
                        let name = if let Some(slash) = path.rfind('/') {
                            if  slash < path.len() - 1 {
                                path[slash + 1..].to_owned()
                            } else {
                                path.to_owned()
                            } 
                        } else {
                            path.to_owned()
                        };
                        eprintln! {"--> {path} status {status_curr}:{status_prev}"};
                        res.push_str(&json_encode(path));
                        res.push_str("\",\"name\":\"");
                        res.push_str(&json_encode(&name));
                        res.push_str("\",\"status\":\"");
                        match (status_curr, status_prev) {
                            ("M", _) => res.push_str("staged"),
                            (_, "M") | (_, "U") | (_, "D") => res.push_str("modified"),
                            (_, "?") => res.push_str("unversioned"),
                            (_, _) => res.push_str("unknown"),
                        }
                        res.push_str("\"}")
                    }
                }
            } else {
                eprintln! {"executed err for {:?}", output.status};
            }
            res.push(']');
            eprintln!("vcs entries:{res}");
            Ok(res)
        } else {
            Err("No VCS established for the repository yet. Try 'git init' first.".into())
        }
    }

    json_ret!{}

    name_of!{"name"}
}

impl PageOps for PageFile {
    fn apply_specific(&self, page_map: &mut HashMap<&str, String>) {
        page_map.insert("session", self.session.clone().unwrap_or_default());
        #[cfg(target_os = "windows")]
        page_map.insert("windows", String::from("true"));
        #[cfg(any(unix, target_os = "redox"))]
        page_map.insert("windows", String::from("false"));
        page_map.insert("id", self.id.clone().unwrap_or_default());
    }

    fn main_load(&self) -> Result<String, Box<dyn Error>> {
        match std::env::current_exe() {
            Ok(cgi_exe) => { 
                let main =
                if env::var("PATH_INFO").is_ok(){
                     PathBuf::from(std::env::var("PATH_TRANSLATED").unwrap()).join(&self.file_name)
                } else {
                     cgi_exe.parent().unwrap().join("resource").join(&self.file_name)
                };
                read_to_string(&main)
                  .map_err(|_err| format! {"ERROR: misconfiguration - can't load {:?}", &main}.into())
            }
            Err(_err) => Err("ERROR: misconfiguration - can't get CGI script path".into())
        }
    }

    fn name(&self) -> String {
        match &self.session {
           Some(session) if !session.is_empty() =>  session.to_string(),
           _ => "Main".to_string()
        }
    }

    fn get_nav(&self) -> Option<Vec<web::Menu<'_>>> {
        let mut projs = Vec::new();
        if let Ok(paths) = read_dir(&self.home) {
            for file in paths.flatten() {
                if file.file_type().map(|t| t.is_file()).unwrap_or(false) {
                    let file_name = file.file_name().to_string_lossy().to_string();
                    if file_name.starts_with(SETTINGS_PREF) && file_name.ends_with(".prop") {
                        let mut session_name = &file_name[SETTINGS_PREF.len()..file_name.len() - ".prop".len ()];
                        if !session_name.is_empty() {
                            if session_name[..1] == *"-" {
                                session_name = &session_name[1..]
                            } else {
                                continue
                            }
                        }
                        let path_info = std::env::var("PATH_INFO").unwrap_or_else(|_| String::new());
                        projs.push(web::Menu::MenuItem{title: if session_name.is_empty() {"default".to_string()} else {
                             session_name.to_string()}, link:format!("/rustcgi/rustcgi{path_info}?session={}\" target=\"_blank",url_encode(session_name)),hint:None, icon:None,short:None})
                    }
                }
            }
        }
        projs.sort();
        let mut res = vec![web::Menu::MenuBox{title:"File".to_string(), hint:None, icon:None},
            web::Menu::MenuItem{title:"New...".to_string(), link:"javascript:newFile()".to_string(),hint:None, icon:None,short:None},
            Menu::Separator,
            web::Menu::MenuBox{title:"Project".to_string(), hint:None, icon:None},
                web::Menu::MenuItem{title:"New...".to_string(), link:"javascript:newProject()".to_string(), hint:None, icon:None,short:None},
            web::Menu::MenuEnd,
            Menu::Separator,
            web::Menu::MenuItem{title:"Save".to_string(), link:"javascript:saveCurrent()".to_string(),hint:None, icon:None, short:Some("^S")},
            web::Menu::MenuItem{title:"Save As...".to_string(), link:"javascript:saveCurrentAs()".to_string(),hint:None, icon:None,short:None},
            web::Menu::MenuItem{title:"Close".to_string(), link:"javascript:closeCurrent()".to_string(),hint:None, icon:None,short:None},
            Menu::Separator,
            web::Menu::MenuItem{title:"Delete".to_string(), link:"javascript:deleteCurrent()".to_string(), hint:None, icon:None,short:None},
            Menu::Separator,
            web::Menu::MenuItem{title:"Save All".to_string(), link:"javascript:saveAll()".to_string(), hint:None, icon:None,short:None},
            web::Menu::MenuItem{title:"Close All".to_string(), link:"javascript:closeAllTab()".to_string(), hint:None, icon:None,short:None},
        web::Menu::MenuEnd,
            
        web::Menu::MenuBox{title:"Edit".to_string(), hint:None, icon:None}, 
           Menu::MenuItem{title:"Undo".to_string(), link:"javascript:undoEdit()".to_string(), short:Some("^Z"), hint:None, icon:None},
           Menu::MenuItem{title:"Redo".to_string(), link:"javascript:redoEdit()".to_string(), hint:None, icon:None,short:Some("^Y")},
           Menu::Separator,
           Menu::MenuItem{title:"⏼ trim copy".to_string(), link:r#"javascript:copySpec()" id="cpySpec"#.to_string(), hint:Some("Copy to clipboard with trimming line numbers"), icon:None,short:None},
           Menu::Separator,
           web::Menu::MenuBox{title:"Change to".to_string(), hint:None, icon:None},
                Menu::MenuItem{title:"Lower".to_string(), link:"javascript:lower()".to_string(),
                  hint:Some("Change case to lower"), icon:None,short:None},
                Menu::MenuItem{title:"Upper".to_string(), link:"javascript:upper()".to_string(),
                  hint:Some("Change case to upper"), icon:None,short:None},
                Menu::MenuItem{title:"Snake".to_string(), link:"javascript:snake()".to_string(),
                  hint:Some("Change style to snake"), icon:None,short:None},
                Menu::MenuItem{title:"Camel".to_string(), link:"javascript:camel()".to_string(),
                  hint:Some("Change style to Camel"), icon:None,short:None},
                Menu::MenuItem{title:"UTF-16 surrogate".to_string(), link:"javascript:utf16()".to_string(),
                  hint:Some("Unicode 32 bit hex to two 16 bit surrogates"), icon:None,short:None},
                Menu::MenuItem{title:"From NP".to_string(), link:"javascript:fromNotepad()".to_string(),
                  hint:Some("Replace highlighted by the notepad highlighted"), icon:None,short:None},
           web::Menu::MenuEnd,
           Menu::Separator,
           Menu::MenuItem{title:"To Notepad".to_string(), link:"javascript:copySelected()".to_string(), hint:Some("Copy the selected to the notepad"), icon:None,short:None},
           Menu::MenuItem{title:"Save Notepad".to_string(), link:"javascript:saveNotepad()".to_string(), hint:None, icon:None,short:None},
           Menu::Separator,
           web::Menu::MenuItem{title:"Reload".to_string(), link:"javascript:reloadCurrent()".to_string(), hint:Some("Drop changes and Reload the currently edited file"), icon:None,short:None},
           web::Menu::MenuItem{title:"Refresh Proj".to_string(), link:"javascript:refresh()".to_string(), hint:Some("Refresh the list of the project files"), icon:None,short:None},
        web::Menu::MenuEnd,
 
         web::Menu::MenuBox{title:"Source".to_string(), hint:Some("The source navigation, compose and refactoring"), icon:None}, 
           Menu::MenuItem{title:"⏼ bookmark".to_string(), link:"javascript:toggleBookmark()".to_string(), hint:Some("Bookmark currently editing line"), icon:None,short:Some("^B")},
           Menu::Separator,
           Menu::MenuItem{title:"Format".to_string(), link:"javascript:formatSrc()".to_string(), hint:Some("Format source of the current file using configured formatter"), icon:None,short:None},
           Menu::Separator,
           Menu::MenuItem{title:"Prompt AI".to_string(), link:"javascript:promptAI()".to_string(), hint:Some("Use the current selection as a prompt"), icon:None,short:None},
        web::Menu::MenuEnd,
          
        web::Menu::MenuBox{title:"Project".to_string(), hint:None, icon:None},
            web::Menu::MenuBox{title:"Build".to_string(), hint:None, icon:None},
                Menu::MenuItem{title:"Debug".to_string(), link:"javascript:build_debug()".to_string(),
                  hint:Some("Build a debug version of the project"), icon:None,short:None},
                Menu::MenuItem{title:"Release".to_string(), link:"javascript:build_release()".to_string(),
                  hint:Some("Build a release version of the project"), icon:None,short:None},
                Menu::MenuItem{title:"Clippy".to_string(), link:"javascript:clippy_build()".to_string(),
                  hint:Some("Get a Clippy report for the project"), icon:None,short:None},
           web::Menu::MenuEnd,
           web::Menu::MenuBox{title:"Run".to_string(), hint:None, icon:None},
                Menu::MenuItem{title:"Debug".to_string(), link:"javascript:run_debug()".to_string(),
                  hint:Some("Run a debug version of the project"), icon:None,short:None},
                Menu::MenuItem{title:"Release".to_string(), link:"javascript:run_release()".to_string(),
                  hint:Some("Run a release version of the project"), icon:None,short:None},
                Menu::MenuItem{title:"Test".to_string(), link:"javascript:test_app()".to_string(),
                  hint:Some("Run unit tests for the project"), icon:None,short:None},
           web::Menu::MenuEnd,
           Menu::Separator,
           Menu::MenuItem{title:"Package".to_string(), link:"javascript:package()".to_string(), hint:None, icon:None,short:None},
           Menu::Separator,
           web::Menu::MenuBox{title:"VCS".to_string(), hint:None, icon:None},
                Menu::MenuItem{title:"Pull".to_string(), link:"javascript:vcsPull()".to_string(),
                  hint:Some("Pull changes in the project"), icon:None,short:None},
                Menu::MenuItem{title:"Push".to_string(), link:"javascript:vcsPush()".to_string(),
                  hint:Some("Push changes of the project in a remote repository"), icon:None,short:None},
           web::Menu::MenuEnd,
           Menu::Separator,
           Menu::MenuItem{title:"Config...".to_string(), link:"javascript:config_project()".to_string(), hint:None, icon:None,short:None},
        web::Menu::MenuEnd,
        
        web::Menu::MenuBox{title:"VCS".to_string(), hint:Some("Version Control System"), icon:None}, 
           Menu::MenuItem{title:"Status".to_string(), link:"javascript:vcsStatus()".to_string(), hint:None, icon:None,short:None},
           Menu::Separator,
           Menu::MenuItem{title:"Commit...".to_string(), link:"javascript:vcsCommit()".to_string(), hint:None, icon:None,short:None},
           Menu::Separator,
           Menu::MenuItem{title:"Restore".to_string(), link:"javascript:vcsRestore()".to_string(), hint:Some("Restore the current file content from VCS"), icon:None,short:None},
           Menu::MenuItem{title:"Stage".to_string(), link:"javascript:vcsStage()".to_string(), hint:Some("Stage the current file"), icon:None,short:None},
        web::Menu::MenuEnd,
        
        web::Menu::MenuItem{title:"⚙ Settings".to_string(), link:"javascript:showSettings()".to_string(), hint:None, icon:None,short:None},
        
        web::Menu::MenuBox{title:"Help".to_string(), hint:None, icon:None},
            web::Menu::MenuItem{title:"Documentation".to_string(), link:"/cgires/resource/documentation.html\" target=\"help".to_string(), hint:None,icon:None,short:None},
            web::Menu::MenuItem{title:"About...".to_string(), link:"javascript:about()".to_string(), hint:None, icon:None,short:None},
        web::Menu::MenuEnd];
        res.splice(5..5, projs);
        res.into()
    }
}

impl PageOps for PageFrag {

    fn main_load(&self) -> Result<String, Box<dyn Error>> {
        Ok(self.fragment.content.clone())
    }
    
    // prevent side effects
    fn apply_specific(&self, page_map: &mut HashMap<&str, String>) {
        page_map.clear()
    }

    fn name(&self) -> String {
        self.params.param("name").unwrap_or_default()
    }
}

impl PageOps for PageStuff {
    fn main_load(&self) -> Result<String, Box<dyn Error>> {
        Ok(self.content.clone())
    }
    
    fn content_type(&self) -> String {
        "text/plain".to_string()
    }
    
    name_of!{"None"}
}

impl PageOps for PageStuffE {
    fn main_load(&self) -> Result<String, Box<dyn Error>> {
        Ok(self.content.clone())
    }

    fn status(&self) -> Option<(u16, &str)> {
        Some((404, "Not found"))
    }
    
    fn content_type(&self) -> String {
        "text/plain".to_string()
    }
        
    name_of!{"None"}
}


impl PageOps for JsonStuff {
    fn main_load(&self) -> Result<String, Box<dyn Error>> {
        Ok(self.json.to_owned())
    }

    fn name(&self) -> String {
        self.name.to_owned()
    }
    
    json_ret!{}
}

impl PageOps for Redirect {
    fn main_load(&self) -> Result<String, Box<dyn Error>> {
        Ok("redirect".to_string())
    }
    
    fn get_extra(&self) -> Option<Vec<(String, String)>> {
        let id = simran::generate_random_sequence(12);
        let path_info = std::env::var("PATH_INFO").unwrap_or_else(|_| String::new());
        // TODO use ".?session={}&id={id}"
        Some(vec![("Location".to_string(), 
            format!("/rustcgi/rustcgi{path_info}?session={}&id={id}", simweb::url_encode(self.session.clone().unwrap_or_default())))])
        
    }
    
    fn status(&self) -> Option<(u16, &str)> {
        Some((302, "Found"))
    }
    
   name_of!{"None"}
}

fn recurse_files(path: &Path) -> Result<JsonStr, Box<dyn Error>> {
    let name = path
        .file_name().ok_or("no file name")?
        .to_str().ok_or("no string file name")?;

    let mut buf = JsonStr::from("{\"name\": \"");
    buf.push_str(&json_encode(name));
    let meta = match path.metadata() {
        Ok(metadata) => metadata,
        #[cfg(feature = "quiet")]
        Err(_err) => { // probably symlink, skip
            buf.push_str(r#"", "type": "dead"}"#);
            return Ok(buf)},
        #[cfg(not(feature = "quiet"))]
        Err(_err) => { // probably symlink, skip
            eprintln!("No metadata for {path:?} {err:?}"); 
            buf.push_str(r#"", "type": "dead"}"#);
            return Ok(buf)},
    };
    if let Ok(time) = meta.modified() {
        buf.push_str(&format!(r#"", "modified": {}, "type": ""#, time.duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()));
    } else {
        eprintln!("File modified time not supported on this platform");
        buf.push_str(r#"", "type": ""#);
    }
    if meta.is_dir() && name != ".git" {
        buf.push_str("folder\", \"children\": [");
        let mut paths: Vec<_> = read_dir(path)?.flatten().collect();
        paths.sort_by_key(|dir| dir.path());
        let mut entries = paths.iter();
        if let Some(entry) = entries.next() {
            buf.push_str(&recurse_files(&entry.path())?);
            for entry in entries {
                buf.push(',');
                buf.push_str(&recurse_files(&entry.path())?);
            }
        }
        buf.push_str("]}")
    } else if meta.is_file() {
        buf.push_str(&format!(r#"file", "size": {}}}"#, meta.len()))
    } else {
        buf.push_str("dead\"}")
    }
    Ok(buf)
}

fn recurse_dirs(path: &Path, parent: Option<&String>) -> io::Result<JsonStr> {
    //eprintln! {"called with parent {:?}", parent};
    let meta = path.metadata()?;
    let mut buf = JsonStr::from("");
    if meta.is_dir() && path.file_name().unwrap().to_str() != Some(".git") {
        let dirs: Vec<_> = read_dir(path)?
            .filter_map(|f| 
                match f {
                    Ok(f) if f.file_type().map(|t| t.is_dir()).unwrap_or(false)
                        && f.file_name().to_str() != Some(".git") => Some(f),
                    _ => None
                })
            // .sort_by_key(|dir| dir.path())
            .collect();
        let mut dirs_pick = dirs.iter().peekable();
        while let Some(entry) = dirs_pick.next() {
            buf.push('"');
            if let Some(parent) = parent {
                buf.push_str(parent);
                buf.push('/')
            }
            let file_name = entry.file_name().into_string().unwrap().to_string();
            buf.push_str(&json_encode(&file_name));
            buf.push('"');
            let mut parent_str = String::from("");
            if let Some(parent) = parent {
                parent_str.push_str(parent);
                parent_str.push('/')
            }
            parent_str.push_str(&file_name);
            let child_dirs = recurse_dirs(entry.path().as_path(), Some(&parent_str))?;
            if !child_dirs.is_empty() {
                buf.push(',');
                buf.push_str(&child_dirs)
            }
            if dirs_pick.peek().is_some() {
                buf.push(',')
            }
        }
    }
    Ok(buf)
}

fn refs_to_json(refs: &[Reference], exemp_len:usize) -> String {
    #[cfg(any(unix, target_os = "redox"))]
    let ser_ref = |current: &Reference| format!{r#"{{"name":"{}","path":"{}","line":{},"pos":{}}}"#,
        json_encode(&current.name), json_encode(&current.src[exemp_len+1..]), current.line, current.column};
    #[cfg(target_os = "windows")]
    let ser_ref = |current: &Reference| format!{r#"{{"name":"{}","path":"{}","line":{},"pos":{}}}"#,
        json_encode(&current.name), json_encode(&param::to_web_separator(current.src[exemp_len+1..].to_owned())), current.line, current.column};
    refs.iter().map(ser_ref).collect::<Vec<_>>().join(",")
}
