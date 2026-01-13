extern crate simterm;
extern crate simweb;
extern crate simtime;
extern crate simcfg;
mod config;
use std::{
    collections::HashMap,
    path::{PathBuf,Path,MAIN_SEPARATOR_STR},
    time::{UNIX_EPOCH,SystemTime},
   fs::{File,OpenOptions,self},
    io::{self,BufRead,Write,stdout}, error::Error, env,
};
use simtime::{seconds_from_epoch,get_datetime};
use simterm::{Terminal,unescape,send};
use config::{Config};

const VERSION: &str = env!("VERSION");

struct WebTerminal {
    config: Config,
    project_dir: String,
    session: String,
    cwd: PathBuf,
    version: String
}

impl Terminal for WebTerminal {
    fn init(&self) -> (PathBuf, PathBuf, HashMap<String,Vec<String>>,&str) {
        let aliases = read_aliases(HashMap::new(), &self.config, &None::<String>);
        unsafe{env::set_var("PWD", &self.cwd)}
        #[cfg(windows)]
        unsafe { env::set_var("TERM", "xterm-256color") }
        (self.cwd.clone(),self.config.workspace_dir.join(&self.project_dir),aliases,&self.version)
    }
    
    fn save_state(&self) -> Result<(), Box<dyn Error>> {
        let mut sessions = load_persistent(&self.config);
        sessions.insert(self.session.to_string(),(self.cwd.clone().display().to_string(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()));
        save_persistent(&self.config, sessions)
    }
    fn persist_cwd(&mut self, cwd: &Path) {
        let mut sessions = load_persistent(&self.config);
        let cwd_str = cwd.display().to_string();
        sessions.insert(self.session.to_string(),(cwd_str.clone(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()));
        self.cwd = cwd.into();
        let _ = save_persistent(&self.config, sessions);
        unsafe{env::set_var("PWD", cwd_str)}
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let web = simweb::WebData::new();
    let binding = if web.path_info().starts_with("/") {web.path_info()[1..].to_string()} else {web.path_info()};
    let (project_name,session) = match binding.split_once('/') {
        Some((project,session)) => (Some(project.to_owned()),session.strip_prefix("webId-").unwrap_or(session)),
        _ => (None,"")
    };
    let env_ver = web.param("version").unwrap_or_else(|| "".to_owned());
    let config = config::Config::new();
    let version = format!("{VERSION}-{}/{env_ver}", simterm::VERSION);
    let project_path = config.get_project_home(&project_name).
        unwrap_or_else(|| {send!("No {project_name:?} config found, the project is misconfigured\n"); config.workspace_dir.display().to_string()}); 
    
    let sessions = load_persistent(&config);
    let cwd;
    if !session.is_empty() {
        let entry = sessions.get(session);
        if let Some(entry) = entry {
            let initial_dir = entry.0.strip_suffix(MAIN_SEPARATOR_STR).unwrap_or(&entry.0).to_string();
            cwd = PathBuf::from(initial_dir);
        } else {
            send!("No {session} found\n");
            cwd = PathBuf::from(&config.workspace_dir.join(&project_path));
        }
    } else {
        send!("No session specified, the terminal needs to be restarted\n");
        cwd = PathBuf::from(&config.workspace_dir);
    }

    let _ = WebTerminal{config, project_dir:project_path,
         session: session.to_string(), cwd, version }.main_loop();
    Ok(())
}

fn load_persistent(config: &Config) -> HashMap<String, (String,u64)> {
     let mut props = HashMap::new();
    let props_path = config.get_config_path(&None::<String>, "webdata", "properties");
    if let Ok(file) = File::open(&props_path) {
        let lines = io::BufReader::new(file).lines();
        for prop_def in lines.map_while(Result::ok) {
             if prop_def.is_empty() || prop_def.starts_with("#") {
                 // comment
                  continue
             }
             // zKMfbJn35gFy=2025-04-26T17\:38\:36.2465801;C\:\\Users\\sunil\\projects\\simecho
             if let Some((key,val)) = prop_def.split_once("=") {
                 if let Some((date,cwd)) = val.split_once(';') {
                    let last = 
                    if let Some((date,_time)) = date.split_once('T') {
                        let parts: Vec<&str> = date.splitn(3, '-').collect();
                        seconds_from_epoch(1970, parts[0].parse::<u32>().unwrap_or(2025), parts[1].parse::<u32>().unwrap_or(1),
                            parts[2].parse::<u32>().unwrap_or(2),0u32,0u32,0u32).unwrap()
                    } else {
                        SystemTime::now().duration_since(UNIX_EPOCH) .unwrap().as_secs()
                    };
                    let cwd = unescape(&cwd);
                    if PathBuf::from(&cwd).exists() {
                        props.insert(key.to_string(), (cwd.to_string(),last));
                    }
                } else {
                    eprintln!("Invalid property value: {val}")
                };
                
            } else {
                eprintln!("Invalid property definition: {prop_def}")
            }
        }
    } else {
        eprintln! {"Props: {props_path:?} not found"}
    }
    props
}

fn save_persistent(config: &Config, sessions: HashMap<String, (String,u64)>) -> Result<(), Box<dyn std::error::Error>> {
    // update current (before save)
    // TODO consider to write a lock wrapper for something like
    // lock(save_persistent())

    // as for now using webdata.LOCK file
    let mut props_path = config.config_dir.clone();
    props_path.push("webdata");
    props_path.set_extension("LOCK");
    { // check if LOCK file is here
        File::create_new(&props_path)?;
    }
    props_path.set_extension("properties");
    let mut file = OpenOptions::new()
     .write(true)
     .truncate(true)
     .create(true)
     .open(&props_path)?;
    //file.lock()?;
    writeln!{file, "# WebSocket sessions"}?;
    let now = SystemTime::now();
    writeln!{file, "# {}", simweb::http_format_time(now)}?;
    let now = now.duration_since(UNIX_EPOCH).unwrap().as_secs();
    for (key, value) in sessions {
        if value.1 > now - 7*2*24*60*60 && PathBuf::from(&value.0).is_dir() {
            let (y,m,d,h,mm,s,_) = get_datetime(1970, value.1);
            writeln!{file,
               "{key}={y:04}-{m:02}-{d:02}T{h:02}\\:{mm:02}\\:{s:02}.0000000;{}",esc_string(value.0) }?;
        } else {
            //eprintln!{"path {} too old {}", value.0, value.1}
        }
    }
    //file.unlock()?;
    { // remove  LOCK file if is here
        props_path.set_extension("LOCK");
        fs::remove_file(&props_path)?;
    }
    Ok(())
}

fn read_aliases(mut res: HashMap<String,Vec<String>>, config: &Config, project: &Option<String> ) -> HashMap<String,Vec<String>> {
    let aliases = config.get_config_path(project, "aliases", "prop");
    if let Ok(lines) = read_lines(&aliases) {
        // Consumes the iterator, returns an (Optional) String
        for line in lines.map_while(Result::ok) {
            let line = line.trim();
            if line.is_empty() ||
              line.starts_with('#') { // ignore
                continue
            }
            if let Some((name,value)) = line.split_once('=') &&
                name.starts_with("alias ") {
                let name = name.strip_prefix("alias ").unwrap();
                let name = name.trim();
                let q: &[_] = &['"', '\''];
                let value = value.trim_matches(q);
                res.insert(name.to_string(),value.split_ascii_whitespace().map(str::to_string).collect());
            }
            //println!("{}", line);
        }
    }
    
    res
}

fn esc_string(string:String) -> String {
    let mut res = String::new();
    for c in string.chars() {
        match c {
            ':' | '\\' | ' ' | '!' => { res.push('\\'); }
            _ => ()
        }
        res.push(c);
    }
    res
}

fn read_lines(filename: &PathBuf) -> io::Result<io::Lines<io::BufReader<File>>> {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}