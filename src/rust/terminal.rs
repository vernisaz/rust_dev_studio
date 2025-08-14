//#![feature(file_lock)]
/// terminal web socket CGI

macro_rules! send {
    ($($arg:tt)*) => (
        //use std::io::Write;
        
        let s = format!($($arg)* ) ;
        /*let l = s.len();
        println!("{l}");*/
        match write!(stdout(), "{s}") {
            Ok(_) => stdout().flush().unwrap(),
            Err(x) => panic!("Unable to write to stdout (file handle closed?): {}", x),
        }
    )
}
//extern crate simjson;
extern crate simweb;
extern crate simtime;
use std::{io::{stdout,self,Read,BufRead,Write,Stdin,BufReader},
    fs::{self,read_to_string,File,OpenOptions,Metadata},thread,process::{Command,Stdio},
    path::{PathBuf,MAIN_SEPARATOR_STR},collections::HashMap,time::{SystemTime,UNIX_EPOCH},
    env, fmt,
};
#[cfg(target_os = "windows")]
use std::os::windows::prelude::*;
use simtime::seconds_from_epoch;

const VERSION: &str = env!("VERSION");

const MAX_BLOCK_LEN : usize = 4096;

const PROMPT: &str = "$";

fn main() -> io::Result<()> {
    let web = simweb::WebData::new();
    let binding = if web.path_info().starts_with("/") {web.path_info()[1..].to_string()} else {web.path_info()};
    let (project,session) = match binding.split_once('/') {
        Some((project,session)) => (project,session.strip_prefix("webId-").unwrap_or(session)),
        _ => ("","")
    };
    let ver = web.param("version").unwrap_or("".to_owned());
    let mut stdin = io::stdin();
    //let handle = stdin.lock();
    let home = read_home();
    send!("OS terminal {VERSION}\n") ;// {ver:?} {project} {session}");
    let project_path = get_project_home(&project, &home).
        unwrap_or_else(|| {send!("No project config found, the project is misconfigured\n"); home.display().to_string()}); 
    let mut cwd = PathBuf::from(&home);
    cwd.push(&project_path);
    let mut sessions = load_persistent(&home);
    if !session.is_empty() {
        let entry = sessions.get(session);
        if let Some(entry) = entry {
            cwd = PathBuf::from(entry.0.clone());
            send!("{}\n", cwd.as_path().display());
        } else {
            send!("No {session} found\n");
        }
    }
    let aliases = read_aliases(HashMap::new(), &home, None::<String>);
    let child_env: HashMap<String, String> = env::vars().filter(|&(ref k, _)|
             k != "GATEWAY_INTERFACE"
             && k != "QUERY_STRING"
             && k != "REMOTE_ADDR"
             && k != "REMOTE_HOST"
             && k != "REQUEST_METHOD"
             && k != "SERVER_PROTOCOL"
             && k != "SERVER_SOFTWARE"
             && k != "PATH_INFO"
             && k != "PATH_TRANSLATED"
             && k != "SCRIPT_NAME"
             && k != "REMOTE_IDENT"
             && k != "SERVER_NAME"
             && k != "SERVER_PORT"
             && k != "CONTENT_LENGTH"
             && k != "CONTENT_TYPE"
             && k != "AUTH_TYPE"
             //&& k != "XXX"
             && !k.starts_with("HTTP_")).collect();
    let mut buffer = [0_u8;MAX_BLOCK_LEN]; 
    let mut prev: Option<Vec<u8>> = None;
    loop {
        let vec_buf = match prev {
            None => {
                 let Ok(len) = stdin.read(&mut buffer) else {break};
                 if len == 0 {break};
                 &buffer[0..len]
            }
            Some(ref vec) => vec
        };
        if vec_buf.len() >= 4 && vec_buf[0] == 255 && vec_buf[1] == 255 &&
                    vec_buf[2] == 255 && vec_buf[3] == 4 {
                        break
        }
        if vec_buf.len() == 1 && vec_buf[0] == 3 {
            send!("^C\n");
            continue
        }
        let line = String::from_utf8_lossy(&vec_buf).into_owned();
        prev = None;
        let expand = line.chars().last() == Some('\t');
        // TODO parse with pipe
        let (mut cmd, piped, in_file, out_file) = parse_cmd(&line.trim());
        if cmd.is_empty() { continue };
        if expand {
            let ext = esc_string_blanks(extend_name(&cmd[cmd.len() - 1].clone(), &cwd));
            let mut beg = 
            piped.into_iter().fold(String::new(), |a,e| a + &e.into_iter().reduce(|a2,e2| a2 + " " + &esc_string_blanks(e2)).unwrap() + "|" );
           // for pipe in piped {
                
           // }
            if cmd.len() > 1 {
                cmd.pop();
                beg += &cmd.into_iter().reduce(|a,e| a + " " + &esc_string_blanks(e) ).unwrap()
            } 
            //eprintln!("line to send {} {ext}", beg);
            send!("\r{} {ext}", beg);// &line[..pos]);
            continue
        }
        send!("{PROMPT} {line}"); // \n is coming as part of command
        // TODO separate command by | and then instead of
        // display out, take it as input of next command
        cmd = cmd.into_iter().map(|el| interpolate_env(el)).collect();
        match cmd[0].as_str() {
            "dir" if cfg!(windows) => {
                let names_only =  cmd.len() > 1 && cmd[1] == "/b";
                let mut dir = 
                    if cmd.len() == if names_only {2} else {1} {
                        cwd.clone()
                    } else {
                        let mut dir = PathBuf::from(&cmd[if names_only {2} else {1}]);
                        if !dir.has_root() {
                           dir = cwd.join(dir); 
                        } 
                        dir
                    };
                if dir.display().to_string().find('*').is_none() {
                    let Ok(paths) = fs::read_dir(&dir) else {
                        send!("{dir:?} invalid");
                        continue
                    };
                    
                    let mut dir = String::from(format!("    Directory: {}\n\n", dir.display()));
                    if !names_only {
                        dir.push_str("Mode                 LastWriteTime         Length Name\n");
                        dir.push_str("----                 -------------         ------ ----\n");
                    }
                    for path in paths {
                        let Ok(path) = path else {
                            continue
                        };
                        if !names_only {
                            let metadata = path.metadata()?;
                            let tz = (simtime::get_local_timezone_offset_dst().0 * 60) as i64;
                            let (y,m,d,h,mm,_s,_) = simtime::get_datetime(1970, (metadata.modified().unwrap().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64 + tz) as u64);
                            let ro = metadata.permissions().readonly();
                            let file = metadata.is_file();
                            let link = metadata.is_symlink();
                            if file {
                                dir.push_str("-a")
                            } else {
                                dir.push_str("d-")
                            }
                            if ro { dir.push('r') } else { dir.push('-') }
                            #[cfg(target_os = "windows")]
                            {
                                let attributes = metadata.file_attributes();
                                const FILE_ATTRIBUTE_HIDDEN: u32 = 0x00000002;
                                const FILE_ATTRIBUTE_SYSTEM: u32 = 0x00000004;
                                //const FILE_ATTRIBUTE_ARCHIVE: u32 = 0x00000020;
                                if (attributes & FILE_ATTRIBUTE_HIDDEN) > 0 { // Check if the hidden attribute is set.
                                    dir.push('h')
                                } else {
                                    dir.push('-')
                                }
                                if (attributes & FILE_ATTRIBUTE_SYSTEM) > 0 { // Check if the system attribute is set.
                                    dir.push('s')
                                } else {
                                    dir.push('-')
                                }
                            }
                            if link { dir.push('l') } else { dir.push('-') }
                            let (h,pm) = match h {
                                0 => (12,'A'),
                                h @ 1..12 => (h,'A'),
                                12  => (12,'P'),
                                h @ 13..24 => (h-12,'P'),
                                _ => unreachable!()
                            };
                            dir.push_str( &format!("{:8}{m:>2}/{d:>2}/{y:4}  {h:>2}:{mm:02} {}M {:>14} ",' ', pm, EntryLen(&metadata)));
                        }
                        let path = path.path();
                        let mut reset = true;
                        if path.is_dir() {
                            dir.push_str("\x1b[34;1m");
                        } else if let Some(ext) = path.extension() {
                            let ext = ext.to_str().unwrap();
                            match ext {
                                "exe" | "com" | "bat" => dir.push_str("\x1b[92m"),
                                "zip" | "gz" | "rar" | "7z" | "xz" | "jar" => dir.push_str("\x1b[31m"),
                                "jpeg" | "jpg" | "png" | "bmp" | "gif"  => dir.push_str("\x1b[35m"),
                                _ => reset = false
                            }
                        } else if path.is_symlink() {
                            dir.push_str("\x1b[36m");
                        } else {
                            reset = false
                        }
                        dir.push_str(path.file_name().unwrap().to_str().unwrap());
                        if reset {
                            dir.push_str("\x1b[0m")
                        }
                        dir.push('\n');
                    }
                    send!("{dir}");
                } else {
                    let data = DeferData::from(&dir);
                    let mut res = String::new();
                    dir.pop();
                    for arg in data.src_wild {
                        dir.push(format!{"{}{arg}{}",&data.src_before, &data.src_after});
                        let path = dir.as_path().file_name();
                        res.push_str(path.unwrap().to_str().unwrap());
                        res.push('\n');
                        dir.pop();
                    }
                    send!("{res}"); 
                }
            }
            "pwd" => {
                send!("{}\n", cwd.as_path().display()); // path
            }
            "cd" => {
                if cmd.len() == 1 {
                    cmd.push(project_path.clone())
                }
                cwd.push(cmd[1].clone());
                cwd = remove_redundant_components(&cwd);
                if !cwd.is_dir() {
                    cwd = PathBuf::from(&home);
                    cwd.push(&project_path);
                } else {
                    sessions = load_persistent(&home);
                    sessions.insert(session.to_string(),(cwd.clone().into_os_string().into_string().unwrap(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()));
                    match save_persistent(&home,sessions) {
                        _ => ()
                    }
                }
                send!("{}\n", cwd.as_path().display());
            }
            "del" if cfg!(windows) => {
                if cmd.len() == 1 {
                    send!("No name specified\n");
                    continue
                }
                let mut file = PathBuf::from(&cmd[1]);
                if !file.has_root() {
                   file = cwd.join(file); 
                }
                send!("{} file(s) deleted\n", DeferData::from(&file).do_op(Op::DEL).unwrap());
            }
            "type" if cfg!(windows) => {
                if cmd.len() == 1 {
                    send!("No name specified\n");
                    continue
                }
                let mut file = PathBuf::from(&cmd[1]);
                if !file.has_root() {
                   file = cwd.join(file); 
                }
                let _ = DeferData::from(&file).do_op(Op::TYP);
            }
            "copy" | "ren" if cfg!(windows) => {
                if cmd.len() < 3 {
                    send!("Source and  destination have to be provided\n");
                    continue
                }
                let mut file = PathBuf::from(&cmd[1]);
                if !file.has_root() {
                   file = cwd.join(file); 
                }
                let mut file_to = PathBuf::from(&cmd[2]);
                if !file_to.has_root() {
                   file_to = cwd.join(file_to); 
                }
                match cmd[0].as_str() {
                    "copy" => {send!("{} file(s) copied\n", DeferData::from_to(&file, &file_to).do_op(Op::CPY).unwrap());},
                    "ren" => {send!("{} file(s) renamed\n", DeferData::from_to(&file, &file_to).do_op(Op::REN).unwrap());},
                    _ => unreachable!()
                }
            }
            "echo" if cfg!(windows) => {
                if cmd.len() == 2 {
                    send!("{}\n", cmd[1]);
                }
            }
            "md" | "mkdir" if cfg!(windows) => {
                if cmd.len() == 1 {
                    send!("No name specified\n");
                    continue
                }
                let mut file = PathBuf::from(&cmd[1]);
                if !file.has_root() {
                   file = cwd.join(file); 
                }
                match fs::create_dir(file) {
                    Ok(_) => {send!("{} created\n", cmd[1]);},
                    Err(err) => {send!("Err: {err} in {} creation\n", cmd[1]);},
                }
            }
            "rmdir" if cfg!(windows) => {
                if cmd.len() == 1 {
                    send!("No name specified\n");
                    continue
                }
                let mut file = PathBuf::from(&cmd[1]);
                if !file.has_root() {
                   file = cwd.join(file); 
                }
                match fs::remove_dir_all(file) {
                    Ok(_) => {send!("{} removed\n", cmd[1]);},
                    Err(err) => {send!("Err: {err} in removing {}\n", cmd[1]);},
                }
            }
            "ver!" => {
                send!("{VERSION}/{ver}\n"); // path
            }
            _ => {
                if piped.is_empty() {
                    cmd = expand_wildcard(&cwd, cmd);
                    cmd = expand_alias(&aliases, cmd);
                    if in_file.is_empty() && out_file.is_empty() {
                        prev = call_process(cmd, &cwd, &stdin, &child_env);
                    } else {
                        if in_file.is_empty() {
                            prev = call_process(cmd, &cwd, &stdin, &child_env);
                        } else {
                            let mut in_file = PathBuf::from(in_file);
                            if !in_file.has_root() {
                                in_file = PathBuf::from(&cwd).join(in_file);
                            }
                            match fs::read(&in_file) {
                                Ok(contents) =>  {
                                    let res = call_process_piped(cmd, &cwd, contents, &child_env).unwrap();
                                    if out_file.is_empty() {
                                        send!("{}\n",String::from_utf8_lossy(&res));
                                    } else {
                                        let mut out_file = PathBuf::from(out_file);
                                        if !out_file.has_root() {
                                            out_file = PathBuf::from(&cwd).join(out_file);
                                        }
                                        let _ =fs::write(&out_file, res);
                                    }
                                }
                                _ => ()
                            }
                        }
                    }
                } else {
                    // piping work
                    let mut res = vec![];
                    for mut pipe_cmd in piped {
                        pipe_cmd = expand_wildcard(&cwd, pipe_cmd);
                        pipe_cmd = expand_alias(&aliases, pipe_cmd);
                        // TODO add error handling
                        res = call_process_piped(pipe_cmd.clone(), &cwd, res, &child_env).unwrap(); 
                        //eprintln!("Called {pipe_cmd:?} returned {}", String::from_utf8_lossy(&res));
                    }
                    cmd = expand_wildcard(&cwd, cmd);
                    cmd = expand_alias(&aliases, cmd);
                    //eprintln!("before call {cmd:?}");
                    res = call_process_piped(cmd, &cwd, res, &child_env).unwrap();
                    if out_file.is_empty() {
                        send!("{}\n",String::from_utf8_lossy(&res));
                    } else {
                        let mut out_file = PathBuf::from(out_file);
                        if !out_file.has_root() {
                            out_file = PathBuf::from(&cwd).join(out_file);
                        }
                        let _ =fs::write(&out_file, res);
                    }
                }
            }
        }
    }

    //eprintln!{"exit and saving sess"}
    sessions = load_persistent(&home);
    sessions.insert(session.to_string(),(cwd.into_os_string().into_string().unwrap(),SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()));
    save_persistent(&home,sessions)
}

fn call_process(cmd: Vec<String>, cwd: &PathBuf, mut stdin: &Stdin, filtered_env: &HashMap<String, String>) -> Option<Vec<u8>> {
    let process = 
        if cmd.len() > 1 {
                Command::new(&cmd[0])
             .args(&cmd[1..])
             .stdout(Stdio::piped())
             .stdin(Stdio::piped())
             .stderr(Stdio::piped())
             .env_clear()
             .envs(filtered_env)
             .current_dir(&cwd).spawn()
         } else {
            Command::new(&cmd[0])
             .stdout(Stdio::piped())
             .stdin(Stdio::piped())
             .stderr(Stdio::piped())
             .env_clear()
             .envs(filtered_env)
             .current_dir(&cwd).spawn()
        };
    let mut res : Option<Vec<u8>> = None;
    match process {
        Ok(mut process) => {
        // TODO consider
        // let (mut recv, send) = std::io::pipe()?;
            let Some(mut stdout) = process.stdout.take() else {return None};
            thread::scope(|s| { 
                s.spawn(|| {
                    let mut buffer = [0_u8; MAX_BLOCK_LEN]; 
                    loop {
                        let Ok(l) = stdout.read(&mut buffer) else {break};
                        if l == 0 { break } // 
                        
                        let data = buffer[0..l].to_vec();
                        let string = String::from_utf8_lossy(&data);
                        send!{"{}", string};
                    }
                });
                
                if let Some(stderr) = process.stderr.take() {
                    s.spawn(|| {
                         let reader = BufReader::new(stderr);
                        /* it waits for new output */
                        for line in reader.lines() {
                            let string = line.unwrap();
                            send!{"{}\n", string};
                        }
                    });
                }
               
                if let Some(mut stdin_child) = process.stdin.take() {
                    let mut buffer = [0_u8;MAX_BLOCK_LEN]; 
                    loop {
                        //send!{"{}", "waiting user input"};
                        let Ok(len) = stdin.read(&mut buffer) else {break};
                        if len == 0 {break};
                        if len == 1 && buffer[0] == 3 {
                            // consider obtaining PID and send a kill signal SIGINT to the process, and then break
                            if process.kill().is_ok() {
                                send!("^C");
                                break
                            }
                        }
                        //let line = String::from_utf8_lossy(&buffer[0..len]);
                        match stdin_child.write_all(&buffer[0..len]) {
                            Ok(()) => {
                                stdin_child.flush().unwrap(); // can be an error?
                                send!{"{}", String::from_utf8_lossy(&buffer[0..len])} // echo
                                res = None; // user input consumed by the child process
                            }
                            Err(_) => {
                                res  = Some(buffer[0..len].to_vec()); // user input goes in the terminal way
                                break
                            }
                        }
                    }
                    //send!{"{}", "no input"};
                }
            });
            process.wait().unwrap();
        }
        Err(err) => {send!("Can't run: {} in {cwd:?} - {err}\n", &cmd[0]);},
    }
    res
}

fn call_process_piped(cmd: Vec<String>, cwd: &PathBuf, in_pipe: Vec<u8>, filtered_env: &HashMap<String, String>) -> io::Result<Vec<u8>> {
    let mut process = 
        if cmd.len() > 1 {
                Command::new(&cmd[0])
             .args(&cmd[1..])
             .stdout(Stdio::piped())
             .stdin(Stdio::piped())
             .stderr(Stdio::piped())
             .env_clear()
             .envs(filtered_env)
             .current_dir(&cwd).spawn()?
         } else {
            Command::new(&cmd[0])
             .stdout(Stdio::piped())
             .stdin(Stdio::piped())
             .stderr(Stdio::piped())
             .env_clear()
             .envs(filtered_env)
             .current_dir(&cwd).spawn()?
        };
    let mut stdout = process.stdout.take().unwrap();
    let stderr = process.stderr.take().unwrap();
    let mut stdin_child = process.stdin.take().unwrap();
    let handle = thread::spawn(move || {
        let mut buffer = [0_u8; MAX_BLOCK_LEN]; 
        let mut res = vec![];
        loop {
            let Ok(l) = stdout.read(&mut buffer) else {break};
            if l == 0 { break } // 
            res.extend_from_slice(&buffer[0..l])
        }
        res
    });
        
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            let string = line.unwrap();
            send!{"{}\n", string};
        }
    });

    if stdin_child.write_all(&in_pipe) .is_ok() {
        stdin_child.flush().unwrap()
    }
    drop(stdin_child);
   // process.wait().unwrap();
    Ok(handle.join().unwrap())
}

#[derive(Debug, Clone, PartialEq, Default)]
enum CmdState {
    #[default]
    StartArg ,
    QuotedArg,
    InArg,
    Esc,
    QEsc,
}

#[derive(Debug, Clone, PartialEq, Default)]
enum RedirectSate {
    #[default]
    NoRedirect,
    Input,
    Output,
}

fn parse_cmd(input: &impl AsRef<str>) -> (Vec<String>,Vec<Vec<String>>,String,String) { // TODO add < for first group and > for last gropu which can be be the same
    let mut pipe_res = vec![];
    let mut res = vec![];
    let mut input_file = String::new();
    let mut output_file = String::new();
    let mut state = Default::default();
    let mut curr_comp = String::new();
    let mut red_state = RedirectSate::default();
    let input = input.as_ref();
    for c in input.chars() {
        match c {
            ' ' | '\t' | '\r' | '\n' | '\u{00a0}' | '|' | '(' | ')' | '<' | '>' | ';' | '&' => { // TODO special processing for redrect symbols
                 match state {
                    CmdState:: StartArg => {
                        match c {
                            '|' => {
                                // finish the command + args group and start a new one
                                pipe_res.push(res.clone());
                                res.clear();
                            }
                            '<' => { red_state = RedirectSate::Input; }
                            '>' => { red_state = RedirectSate::Output }
                            _ => (),
                        }
                    }
                    CmdState:: InArg => {
                        state = CmdState:: StartArg;
                        match red_state {
                            RedirectSate::NoRedirect => {
                                res.push(curr_comp.clone());
                            }
                            RedirectSate::Input => {input_file = String::from(&curr_comp);}
                            RedirectSate::Output => {output_file = String::from(&curr_comp);}
                        }
                        curr_comp.clear();
                        match c {
                            '|' => {
                                pipe_res.push(res.clone());
                                res.clear();
                            }
                            '<' => { red_state = RedirectSate::Input; }
                            '>' => { red_state = RedirectSate::Output }
                            _ => red_state = RedirectSate::NoRedirect,
                        }
                    }
                    CmdState:: Esc => {
                        state = CmdState:: InArg;
                        curr_comp.push(c)
                    } 
                    CmdState:: QuotedArg => {
                        curr_comp.push(c);
                    }
                    _ => todo!()
                }
            }
            '"' => {
                match state {
                   CmdState:: StartArg => {
                       state = CmdState:: QuotedArg;
                   }
                   CmdState:: QuotedArg => {
                       state = CmdState:: StartArg;
                       res.push(curr_comp.clone());
                       curr_comp.clear();
                   }
                   CmdState:: InArg => {
                        state = CmdState:: QuotedArg;
                   }
                   _ => todo!()
                }
            }
            '\\' => {
                match state {
                    CmdState:: StartArg | CmdState:: InArg => {
                       state = CmdState:: Esc;
                    }
                    CmdState:: QuotedArg => {
                        state = CmdState:: QEsc;
                    }
                    CmdState:: Esc => {
                        state = CmdState:: InArg;
                        curr_comp.push(c);
                    }
                    CmdState:: QEsc => {
                        state = CmdState:: QuotedArg;
                        curr_comp.push(c);
                    }
                }
            }
            other => {
                match state {
                    CmdState:: StartArg => {
                       state = CmdState:: InArg;
                       curr_comp.push(other);
                   }
                   CmdState:: QuotedArg | CmdState:: InArg=> {
                       curr_comp.push(other);
                   }
                   CmdState:: Esc => {
                        state = CmdState:: InArg;
                        curr_comp.push('\\');
                        curr_comp.push(c);
                   }
                   _ => todo!()
                }
            }
        }
       
    }
    match state {
        CmdState:: Esc => {
            curr_comp.push('\\');
            state = CmdState:: InArg;
        }
        _ => ()
    }
    match state {
        CmdState:: InArg | CmdState::QuotedArg  => {
            match red_state {
                RedirectSate::NoRedirect => {
                    res.push(curr_comp);
                }
                RedirectSate::Input => {input_file = String::from(&curr_comp);}
                RedirectSate::Output => {output_file = String::from(&curr_comp);}
            }
        }
        CmdState:: StartArg => (),
        _ => todo!()
    }
    (res, pipe_res,input_file,output_file)
}

fn expand_wildcard(cwd: &PathBuf, cmd: Vec<String>) -> Vec<String> { // Vec<Cow<String>>
    #[cfg(not(target_os = "windows"))]
    let prog = cmd[0].clone();
    #[cfg(target_os = "windows")]
    let mut prog = cmd[0].clone();
    #[cfg(target_os = "windows")]
    if prog.starts_with(".\\") {
        prog = cwd.to_owned().join(prog).display().to_string();
    }
    let mut res = vec![prog];
    for comp in &cmd[1..] {
        if comp.find('*').is_none() {
            res.push(comp.to_string());
        } else {
            let mut comp_path = PathBuf::from(&comp);
            if !comp_path.has_root() {
                comp_path = cwd.clone().join(comp_path)
            }
            let data = DeferData::from(&comp_path);
            comp_path.pop();
            for arg in data.src_wild {
                comp_path.push(format!{"{}{arg}{}",&data.src_before, &data.src_after})
                ;
                res.push(comp_path.display().to_string());
                comp_path.pop();
            }
        }
    }
    res
}

fn expand_alias(aliases: &HashMap<String,Vec<String>>, mut cmd: Vec<String>) -> Vec<String> {
    match aliases.get(&cmd[0]) {
        Some(expand) => { cmd.splice(0..1, expand.clone()); cmd }
        _ => cmd
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
enum EnvExpState {
    #[default]
    InArg ,
    ExpEnvName,
    InBracketEnvName,
    InEnvName,
    Esc,
    NoInterpol,
    EscNoInterpol,
}

fn interpolate_env(s:String) -> String {
// this function called when parameters are going in the processing
    let mut res = String::new();
    let mut state = Default::default();
    let mut curr_env = String::new();
    
    for c in s.chars() {
        match c {
            '$' => {
                match state {
                    EnvExpState::InArg => 
                        state = EnvExpState:: ExpEnvName,
                    EnvExpState::Esc => { state = EnvExpState::InArg; res.push(c) },
                    EnvExpState::InEnvName | EnvExpState::ExpEnvName => {
                        let env_variable = env::var(&curr_env).unwrap_or_else(|_| "".to_string());
                        if !env_variable.is_empty() {
                            res.push_str(&env_variable)
                        }
                        curr_env.clear();
                        state = EnvExpState::ExpEnvName
                    }
                    EnvExpState::InBracketEnvName => curr_env.push(c),
                    EnvExpState:: NoInterpol => res.push(c),
                    EnvExpState::EscNoInterpol => { res.push('\\');
                        res.push(c); state =  EnvExpState::NoInterpol
                    }
                }
            }
            '\\' => {
                match state {
                    EnvExpState::InArg => { state =  EnvExpState::Esc }
                    EnvExpState::Esc => { res.push('\\');
                        state =  EnvExpState::InArg
                    }
                    EnvExpState::InEnvName | EnvExpState::ExpEnvName => {
                        let env_variable = env::var(&curr_env).unwrap_or_else(|_| "".to_string());
                        if !env_variable.is_empty() {
                            res.push_str(&env_variable)
                        }
                        curr_env.clear();
                        state = EnvExpState::Esc
                    }
                    EnvExpState::InBracketEnvName => curr_env.push(c),
                    EnvExpState:: NoInterpol => state = EnvExpState::EscNoInterpol,
                    EnvExpState::EscNoInterpol => {
                        res.push(c); state =  EnvExpState::NoInterpol
                    }
                }
            }
            'a'..='z' | 'A'..='Z' | '_' | '0'..='9' => {
                match state {
                    EnvExpState::InArg => { res.push(c) }
                    EnvExpState::Esc => { res.push('\\');
                        res.push(c); state =  EnvExpState::InArg
                    }
                    EnvExpState::InEnvName | EnvExpState::InBracketEnvName => {
                        curr_env.push(c);
                    }
                    EnvExpState::ExpEnvName => {
                        curr_env.push(c);
                        state = EnvExpState::InEnvName
                    }
                    EnvExpState:: NoInterpol => res.push(c),
                    EnvExpState::EscNoInterpol => { res.push('\\');
                        res.push(c); state =  EnvExpState::NoInterpol
                    }
                }
            }
            '{' => {
                match state {
                    EnvExpState::InArg => {
                        res.push(c)
                    }
                    EnvExpState::Esc => { res.push('\\');
                        res.push(c); state =  EnvExpState::InArg
                    }
                    EnvExpState::InEnvName => {
                        let env_variable = env::var(&curr_env).unwrap_or_else(|_| "".to_string());
                        if !env_variable.is_empty() {
                            res.push_str(&env_variable)
                        }
                        curr_env.clear();
                        res.push(c);
                        state = EnvExpState::InArg
                    }
                    EnvExpState::ExpEnvName => {
                        state = EnvExpState::InBracketEnvName
                    }
                    EnvExpState::InBracketEnvName => curr_env.push(c),
                    EnvExpState:: NoInterpol => res.push(c),
                    EnvExpState::EscNoInterpol => { res.push('\\');
                        res.push(c); state =  EnvExpState::NoInterpol
                    }
                }
            }
            '}' => {
                match state {
                    EnvExpState::InArg | EnvExpState:: NoInterpol => {
                        res.push(c)
                    }
                    EnvExpState::ExpEnvName => {
                        state = EnvExpState::InArg;
                        res.push(c)
                    }
                    EnvExpState::Esc => { res.push('\\');
                        res.push(c); state =  EnvExpState::InArg
                    }
                    EnvExpState::InEnvName => {
                        let env_variable = env::var(&curr_env).unwrap_or_else(|_| "".to_string());
                        if !env_variable.is_empty() {
                            res.push_str(&env_variable)
                        }
                        curr_env.clear();
                        res.push(c);
                        state = EnvExpState::InArg
                    }
                    EnvExpState::InBracketEnvName => {
                        let env_variable = env::var(&curr_env).unwrap_or_else(|_| "".to_string());
                        if !env_variable.is_empty() {
                            res.push_str(&env_variable)
                        }
                        curr_env.clear();
                        state = EnvExpState::InArg
                    }
                    EnvExpState::EscNoInterpol => { res.push('\\');
                        res.push(c); state =  EnvExpState::NoInterpol
                    }
                }
            }
            '\'' => { // no interpolation inside ''
                match state {
                    EnvExpState::InArg => 
                        state = EnvExpState:: NoInterpol,
                    EnvExpState:: NoInterpol => state = EnvExpState::InArg,
                    EnvExpState::EscNoInterpol => {
                        res.push(c);
                        state = EnvExpState:: NoInterpol
                    }
                    EnvExpState::Esc => {
                        res.push(c); state =  EnvExpState::InArg
                    }
                    EnvExpState::InBracketEnvName | EnvExpState::InEnvName | EnvExpState::ExpEnvName => (), // generally error
                }
            }
            _ => {
                match state {
                    EnvExpState::InArg | EnvExpState:: NoInterpol => {
                        res.push(c)
                    }
                    EnvExpState::Esc => { res.push('\\');
                        res.push(c); state =  EnvExpState::InArg
                    }
                    EnvExpState::EscNoInterpol => { res.push('\\');
                        res.push(c); state =  EnvExpState::NoInterpol
                    }
                    EnvExpState::InEnvName | EnvExpState::ExpEnvName => {
                        let env_variable = env::var(&curr_env).unwrap_or_else(|_| "".to_string());
                        if !env_variable.is_empty() {
                            res.push_str(&env_variable)
                        }
                        curr_env.clear();
                        res.push(c);
                        state = EnvExpState::InArg
                    }
                    EnvExpState::InBracketEnvName => curr_env.push(c),
                }
            }
        }
    }
    match state {
        EnvExpState::InArg | EnvExpState::ExpEnvName | EnvExpState::InBracketEnvName | EnvExpState::NoInterpol=> {
        }
        EnvExpState::Esc | EnvExpState::EscNoInterpol => { res.push('\\');
        }
        EnvExpState::InEnvName => {
            let env_variable = env::var(&curr_env).unwrap_or_else(|_| "".to_string());
            if !env_variable.is_empty() {
                res.push_str(&env_variable)
            }
        }
    }
    res
}

fn extend_name(arg: &impl AsRef<str>, cwd: &PathBuf) -> String {
    let  path = PathBuf::from(arg.as_ref());
    let (dir, part_name) =
    match path.parent() {
        None => (cwd.clone(), arg.as_ref().to_string()),
        Some(dir) => {
            if !dir.has_root( ) {
                let mut dir = cwd.join(dir);
                dir.push(".");
                (dir,path.file_name().unwrap().to_str().unwrap().to_string())
            } else {
                (dir.to_path_buf(),path.file_name().unwrap().to_str().unwrap().to_string())
            }
        }
    };
    let files: Vec<String> =
        match dir.read_dir() {
            Ok(read_dir) => read_dir
              .filter(|r| r.is_ok())
              .map(|r| {let p = r.unwrap().path(); p.file_name().unwrap().to_str().unwrap().to_owned() + if p.is_dir() {MAIN_SEPARATOR_STR} else {""}})
              .filter(|r| r.starts_with(&part_name))
              .collect(),
            Err(_) => vec![],
        };
    let dir = dir.display().to_string(); // String =String::from(cwd.to_string_lossy());
    //let cwd = cwd.display().to_string();
    //let dir = dir.strip_prefix(&cwd).unwrap();
    match files.len() {
        0 => format!("{}{}{part_name}",dir,MAIN_SEPARATOR_STR),
        1 => format!("{}{}{}",dir,MAIN_SEPARATOR_STR,files[0].clone()),
        _ => format!("{}{}{}\x07",dir,MAIN_SEPARATOR_STR,longest_common_prefix(files))
    }
}

fn longest_common_prefix(strs: Vec<String>) -> String {
    if strs.is_empty() {
        return String::new();
    }

    let mut prefix = strs[0].clone();

    for i in 1..strs.len() {
        let mut j = 0;
        while j < prefix.len() && j < strs[i].len() && prefix.chars().nth(j) == strs[i].chars().nth(j) {
            j += 1;
        }
        prefix = prefix[..j].to_string();
        if prefix.is_empty() {
            break;
        }
    }

    prefix
}

fn remove_redundant_components(path: &PathBuf) -> PathBuf {
    let mut components = path.components().peekable();
    let mut result = PathBuf::new();

    while let Some(component) = components.next() {
        match component {
            std::path::Component::CurDir => continue,
            std::path::Component::ParentDir => {
                result.pop();
            },
            _ => result.push(component.as_os_str()),
        }
    }

    result
}

fn read_home() -> PathBuf {
    if let Ok(ws_exe) = env::current_exe() {
        if let Some(current_path) = ws_exe.parent() {
            let home_file = current_path.join(".home");
            if let Ok(home) = read_to_string(&home_file) {
                PathBuf::from(home.trim())
            } else {
                eprintln! {"Misconfiguration: HOME isn't set in .home in {:?}", &home_file};
                ws_exe
            }
        } else {
            eprintln! {"Misconfiguration: no executable_dir"};
            PathBuf::new()
        } 
    } else {
        eprintln! {"Misconfiguration: no current_exe"};
       PathBuf::new()
    }  
}

fn read_aliases(mut res: HashMap<String,Vec<String>>, home: &PathBuf, project: Option<impl AsRef<str> + std::fmt::Display> ) -> HashMap<String,Vec<String>> {
    let mut aliases = home.clone();
    aliases.push(".rustcgi");
    match project {
        None => aliases.push("aliases"),
        Some(project) => aliases.push(format!("aliases-{project}"))
    }
    aliases.set_extension("prop");
    if let Ok(lines) = read_lines(&aliases) {
        // Consumes the iterator, returns an (Optional) String
        for line in lines.map_while(Result::ok) {
            let line = line.trim();
            if line.is_empty() { continue }
            if line.starts_with('#') { // ignore
                continue
            }
            if let Some((name,value)) = line.split_once('=') {
                if name.starts_with("alias ") {
                    let name = name.strip_prefix("alias ").unwrap();
                    let name = name.trim();
                    let q: &[_] = &['"', '\''];
                    let value = value.trim_matches(q);
                    res.insert(name.to_string(),value.split_ascii_whitespace().map(str::to_string).collect());
                }
            }
            //println!("{}", line);
        }
    }
    
    res
}

fn read_lines(filename: &PathBuf) -> io::Result<io::Lines<io::BufReader<File>>> {
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn get_project_home(project: &(impl AsRef<str> + std::fmt::Display), home: &PathBuf) -> Option<String> {
     let settings =
        match  project.as_ref() {
            "default" | "" => "settings",
            _  => &format!{"settings-{project}"},
        };
    let mut project_prop_path = home.clone();
    project_prop_path.push(".rustcgi");
    project_prop_path.push(settings);
    project_prop_path.set_extension("prop");
    let settings = read_props(&project_prop_path);
    if let Some(res) = settings.get("project_home") {
        return if res.starts_with("~") {
            Some(res[1..].to_string())
        } else {
            Some(res.into())
        }
    }
    None
}

pub fn read_props(path: &PathBuf) -> HashMap<String, String> {
    let mut props = HashMap::new();
    if let Ok(file) = File::open(path) {
        let lines = io::BufReader::new(file).lines();
        for line in lines {
            if let Ok(prop_def) = line {
                if prop_def.starts_with("#") {
                    // comment
                    continue
                }
                if let Some(pos) = prop_def.find('=') {
                    let name = &prop_def[0..pos];
                    let val = &prop_def[pos + 1..];
                    props.insert(name.to_string(), val.to_string());
                } else {
                    eprintln!("Invalid property definition: {}", &prop_def)
                }
            }
        }
    } else {
        eprintln! {"Props: {path:?} not found"}
    }
    props
}

fn load_persistent(home: &PathBuf) -> HashMap<String, (String,u64)> {
    let mut props = HashMap::new();
    let mut props_path = home.clone();
    props_path.push(".rustcgi");
    props_path.push("webdata.properties");
    if let Ok(file) = File::open(&props_path) {
        let lines = io::BufReader::new(file).lines();
        for line in lines {
            if let Ok(prop_def) = line {
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
                        eprintln!("Invalid property value: {}", &val)
                    };
                    
                } else {
                    eprintln!("Invalid property definition: {}", &prop_def)
                }
            }
        }
    } else {
        eprintln! {"Props: {props_path:?} not found"}
    }
    props
}

fn save_persistent(home: &PathBuf, sessions: HashMap<String, (String,u64)>) -> io::Result<()> {
    // update current (before save)
    // TODO consider to write a lock wrapper for something like
    // lock(save_persistent())

    // as for now using webdata.LOCK file
    let mut props_path = home.clone();
    props_path.push(".rustcgi");
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
            let (y,m,d,h,mm,s,_) = simtime::get_datetime(1970, value.1);
            #[cfg(windows)]
            write!{file,
               "{key}={y:04}-{m:02}-{d:02}T{h:02}\\:{mm:02}\\:{s:02}.0000000;{}\r\n",esc_string(value.0) }?;
            #[cfg(not(windows))]
            write!{file,
               "{key}={y:04}-{m:02}-{d:02}T{h:02}\\:{mm:02}\\:{s:02}.0000000;{}\n",esc_string(value.0) }?;
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

fn unescape(string:&impl AsRef<str>) -> String {
    let mut res = String::new();
    let mut esc = false;
    for c in string.as_ref().chars() {
        match c {
            '\\' => { if esc { esc=false;} else { esc = true; continue} }
            //':' | ' ' | '!' => {esc=false;}
            _ => esc=false,
        }
        res.push(c);
    }
    res
}

fn esc_string_blanks(string:String) -> String {
let mut res = String::new();
    for c in string.chars() {
        match c {
            ' ' | '\\' | '"' => { res.push('\\'); }
            _ => ()
        }
        res.push(c);
    }
    res
}

fn split_at_star(line: &impl AsRef<str>) -> Option<(String,String)> {
    let mut char_indices = line.as_ref().char_indices();
    let mut state = Default::default();
    let mut current = String::new();
    let mut before = None;
    while let Some((_,c)) = char_indices.next() {
        match c { 
            '\\' => match state {
                CmdState::Esc | CmdState::QEsc => current.push(c),
                CmdState::StartArg => {
                    state = CmdState::Esc
                }
                CmdState::InArg => {
                    state = CmdState::QEsc
                }
                _ => unreachable!()
            }
            '*' => match state {
                CmdState::Esc => {current.push(c); state = CmdState::StartArg},
                CmdState::StartArg => {
                    state = CmdState::InArg;
                    before = Some(current.clone());
                    current . clear()
                }
                CmdState::InArg | CmdState::QEsc => {
                    state = CmdState::InArg;
                    current.push(c)
                }
                _ => unreachable!()
            }
            _ => match state {
                CmdState::Esc => { state = CmdState::StartArg;
                current.push('\\'); current.push(c)},
                CmdState::QEsc => { state = CmdState::InArg;
                    current.push('\\'); current.push(c)},
                CmdState::StartArg | CmdState::InArg => {
                    current.push(c)
                }
                _ => unreachable!()
            }
        }
    }
    match state {
        CmdState::InArg => Some((before.unwrap(),current)),
        CmdState::StartArg | CmdState::Esc => None,
        CmdState::QEsc => { current.push('\\'); Some((before.unwrap(),current))},
        _ => unreachable!()
    } 
}

// Windows related

struct EntryLen<'a>(&'a Metadata);

impl fmt::Display for EntryLen<'_> {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_dir() {
            "".fmt(fmt)
        } else {
            self.0.len().fmt(fmt)
        }
    }
}

enum Op {DEL, CPY, REN, TYP}
struct DeferData {
	    src: PathBuf,
	    src_before: String,
	     src_after: String,
	     src_wild: Vec<String>,
	    dst: Option<PathBuf>,
	    dst_before: Option<String>,
	    dst_after: Option<String>,
	    // not for the Rust version
	    //defer_op: Option<Op>,
}

impl DeferData {
    fn from(from:&PathBuf) -> DeferData {
        let from_name = from.file_name().unwrap().to_str().unwrap().to_string();
        let from_dir = from.parent().unwrap_or(&PathBuf::from("")).to_path_buf();
        //let mut src_wild = Vec::new();
        let (src_before,src_after,src_wild) =
        match split_at_star(&from_name) { //.split_once('*') {
            None => {
                (String::new(),String::new(), vec![from_name])
            }
            Some((before,after)) => {
                (before.to_string(),after.to_string(),
                match (before.as_str(),after.as_str()) {
                    ("","") => {
                          from_dir.read_dir().unwrap()
                          .filter(|r| r.is_ok())
                          .map(|r| r.unwrap().path().file_name().unwrap().to_str().unwrap().to_string())
                          .collect::<Vec<String>>()
                    }
                    ("",after) => {
                          from_dir.read_dir().unwrap()
                          .filter(|r| r.is_ok())
                          .map(|r| r.unwrap().path().file_name().unwrap().to_str().unwrap().to_string())
                          .filter(|r| r.ends_with(&after))
                          .map(|r| r.strip_suffix(after).unwrap().to_string())
                          .collect::<Vec<String>>()
                    }
                    (before,"") => {
                          from_dir.read_dir().unwrap()
                          .filter(|r| r.is_ok())
                          .map(|r| r.unwrap().path().file_name().unwrap().to_str().unwrap().to_string())
                          .filter(|r| r.starts_with(&before))
                          .map(|r| r.strip_prefix(before).unwrap().to_string())
                          .collect::<Vec<String>>()
                    }
                    (before,after) => {
                          from_dir.read_dir().unwrap()
                          .filter(|r| r.is_ok())
                          .map(|r| r.unwrap().path().file_name().unwrap().to_str().unwrap().to_string())
                          .filter(|r| r.starts_with(&before) && r.ends_with(&after) && r.len() > before.len() + after.len())
                          .map(|r| r.strip_suffix(after).unwrap().strip_prefix(before).unwrap().to_string())
                          .collect::<Vec<String>>()
                    }
                }
                )
            }
        };
        DeferData {
    	    src: from_dir,
    	    src_before: src_before,
    	     src_after: src_after,
    	     src_wild: src_wild,
    	    dst: None,
    	    dst_before: None,
    	    dst_after: None,
    	    //defer_op: None,
        }
    }
    
    fn from_to(from:&PathBuf, to:&PathBuf) -> Self {
        let mut res = DeferData::from(from);
        let mut to_name = to.file_name().unwrap().to_str().unwrap().to_string();
        let mut to_dir = if to.is_dir() {
            to_name = String::new();
            to
        } else {
            &to.parent().unwrap_or(&PathBuf::from("")).to_path_buf() // ??? the code needs review in case of no parent
        };
        // 
        let (to_before,to_after) =
        match to_name.split_once('*') {
            None => {
                // no wild card 
                to_dir = to;
                (None, None)
            }
            Some((before,after)) => {
                (Some(before.to_string()),Some(after.to_string()))
            }
        };
        res.dst = Some(to_dir.to_path_buf());
	    res.dst_before = to_before;
	    res.dst_after = to_after;
        res
    }
    
    fn do_op(&self,op: Op) -> io::Result<usize> {
        let mut succ_count = 0;
        let mut file = self.src.clone();
        for name in &self.src_wild {
            let name_to =
            if self.dst.is_some() && self.dst_before.is_some() && self.dst_after.is_some() {
                format!{"{}{name}{}",self.dst_before.as_ref().unwrap(), self.dst_after.as_ref().unwrap()}
            } else {
                String::new()
            };
            //eprintln!{"name to {name_to:?}"}
            let name = format!{"{}{name}{}",&self.src_before, &self.src_after};
            file.push(&name) ;
            match op {
                Op::TYP => {
                        //eprintln!{"typing: {file:?}"}
                    let contents = fs::read_to_string(&file)?;
                    send!("{}\n", contents);
                    succ_count += 1
                },
                Op::DEL => {
                    if file.is_file() {
                        if fs::remove_file(&file).is_ok() {
                           succ_count += 1 
                        };
                    } else if file.is_dir() {
                        if fs::remove_dir_all(&file).is_ok() {
                            succ_count += 1
                        };
                    }
                }
                Op::CPY => {
                    let mut file = self.src.clone();
                    let mut dest = self.dst.clone().unwrap();
                    file.push(name) ;
                    if !name_to.is_empty() {
                        dest.push(&name_to)
                    }
                    if file.is_file() {
                        if fs::copy(&file, &dest).is_ok() {
                            succ_count += 1
                        };
                    } else if file.is_dir() {
                    }
                    if !name_to.is_empty() {
                        dest.pop();
                    }
                }
                Op::REN => {
                    let mut file = self.src.clone();
                    let mut dest = self.dst.clone().unwrap();
                    file.push(name) ;
                    if !name_to.is_empty() {
                        dest.push(&name_to)
                    }
                    if file.is_file() || file.is_dir() {
                        if fs::rename(&file, &dest).is_ok() {
                           // eprintln!{"renaming {file:?} to {dest:?}"}
                            succ_count += 1
                        };
                    } 
                    if !name_to.is_empty() {
                        dest.pop();
                    }
                },
            }
            file.pop();
        }
        Ok(succ_count)
    }
}
