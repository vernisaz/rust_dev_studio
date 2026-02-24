use simcfg::read_config_root;

use simweb::{sanitize_web_path, has_root};
use std::{
    collections::HashMap,
    fs::{File, read_to_string},
    io::{BufRead, BufReader},
    path::{MAIN_SEPARATOR, Path, PathBuf},
};

pub const SETTINGS_PREF: &str = "settings";
const RDS_CFG_DIR: &str = ".rds";

pub struct Config {
    pub config_dir: PathBuf,
    pub workspace_dir: PathBuf,
}

impl Config {
    pub fn new() -> Self {
        let config = read_config_root().unwrap_or_else(|_| PathBuf::new()); // TODO : return error if empty
        let mut config_dir = config.join(RDS_CFG_DIR);
        config_dir.push(".workspace");
        if let Ok(workspace_dir) = read_to_string(&config_dir) {
            let workspace_dir = PathBuf::from(&workspace_dir.trim());
            if workspace_dir.is_dir() {
                config_dir.pop();
                return Config {
                    config_dir,
                    workspace_dir,
                };
            } else {
                eprintln!("no directory {workspace_dir:?}")
            }
        }
        config_dir.pop();
        Config {
            config_dir,
            workspace_dir: config,
        }
    }

    #[allow(dead_code)]
    pub fn to_real_path(
        &self,
        project_path: impl AsRef<str> + std::fmt::Debug,
        in_project_path: Option<&String>,
    ) -> Option<String> {
        let project_path = project_path.as_ref();
        if has_root(project_path) {
            return None;
        }
        let mut res = self.workspace_dir.clone();
        if !project_path.is_empty() {
            res.push(project_path);
    
            if let Some(in_project_path) = in_project_path {
                res.push(in_project_path);
            }
        }
        //eprintln!{"parts to connect: config: {:?} {project_path:?} {in_project_path:?} = {res:?}", self.config_dir};
        Some(res.display().to_string())
    }

    #[allow(dead_code)]
    pub fn name_to_path(&self, name: Option<String>) -> Option<String> {
        let name = name?;
        let name = sanitize_web_path(name).ok()?;
        self.to_real_path(&name, None)
    }

    pub fn get_config_path(&self, proj: &Option<String>, prefix: &str, ext: &str) -> PathBuf {
        let mut res = self.config_dir.clone();
        match proj {
            Some(proj) if !proj.is_empty() && proj != "default" => {
                res.push(prefix.to_string() + "-" + proj)
            }
            _ => res.push(prefix),
        }
        res.set_extension(ext);
        res
    }

    pub fn get_project_home(&self, project: &Option<String>) -> Option<String> {
        let settings = self.get_config_path(project, SETTINGS_PREF, "prop");
        if sanitize_web_path(settings.display().to_string()).is_err() {
            return None;
        };
        let settings = read_props(&settings);
        if let Some(res) = settings.get("project_home") {
            let mut chars = res.chars();
            if let Some(c) = chars.next()
                && (c == MAIN_SEPARATOR || c == '~')
            {
                return None;
            }
            #[cfg(windows)]
            if let Some(c) = chars.next()
                && c == ':'
            {
                return None;
            }
            return Some(res.into());
        }
        None
    }
}

pub fn read_props(path: &Path) -> HashMap<String, String> {
    let mut props = HashMap::new();
    if let Ok(file) = File::open(path) {
        let lines = BufReader::new(file).lines();
        for prop_def in lines.map_while(Result::ok) {
            if prop_def.starts_with("#") {
                continue; // comment
            }
            if let Some((name, val)) = prop_def.split_once('=') {
                props.insert(name.to_string(), val.to_string());
            } else {
                eprintln!("Invalid property definition: {}", &prop_def)
            }
        }
    } else {
        eprintln! {"Props: {path:?} not found"}
    }
    props
}

