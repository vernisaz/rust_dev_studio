use simcfg::read_config_root;
#[cfg(any(unix, target_os = "redox"))]
use std::path::{PathBuf};
#[cfg(target_os = "windows")]
use std::path::{PathBuf};
use fs::read_to_string;
use web::sanitize_path;

const RDS_CFG_DIR : &str = ".rds";

pub struct Config {
    pub config_dir: PathBuf,
    pub workspace_dir: PathBuf, 
}

impl Config {
    pub fn new() -> Self {
        let config = read_config_root().unwrap_or(PathBuf::new());
        // TODO : return error if empty
        let mut config_dir = config.join(RDS_CFG_DIR);
        config_dir.push(".workspace");
        if let Ok(workspace_dir) = read_to_string(&config_dir) {
            let workspace_dir = PathBuf::from(&workspace_dir);
            if workspace_dir.exists() && workspace_dir.is_dir() {
                config_dir.pop();
                return Config {
                    config_dir: config_dir,
                    workspace_dir: workspace_dir,
                }
            }
        }
        config_dir.pop();
        Config {
            config_dir: config_dir,
            workspace_dir: PathBuf::from(config),
        }
    }
    
    pub fn to_real_path(
        &self,
        project_path: impl AsRef<str> + std::fmt::Debug,
        in_project_path: Option<&String>,
    ) -> String {
        let project_path = project_path.as_ref();
        let mut res = self.workspace_dir.clone();
        if project_path.starts_with('/') { // not allowed an absolute path yet, but it needs verify on Windows
            res.push(project_path[1..].to_owned());
        } else {
            res.push(project_path);
        }
        
        if let Some(in_project_path) = in_project_path {
            res.push(in_project_path);
        }
        eprintln!{"parts to connect: config: {:?} {project_path:?} {in_project_path:?} = {res:?}", self.config_dir};
        res.display().to_string()
    }
    
    pub fn name_to_path(&self, name: Option<String>) -> Option<String> {
        if let Some(name) = name {
            let _ = sanitize_path(&name).ok()?;
            Some(self.to_real_path(&name, None))
        } else {
            None
        }
    }
}