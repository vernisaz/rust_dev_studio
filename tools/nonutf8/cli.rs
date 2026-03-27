#![allow(unused)]
use std::{collections::HashSet, env, fmt};

#[cfg(unix)]
const OPT_PREFIX: char = '-';
#[cfg(windows)]
const OPT_PREFIX: char = '/';

#[derive(PartialEq)]
#[allow(dead_code)]
pub enum OptTyp {
    Num,
    FNum,
    Str,
    InStr,
    None,
}
#[derive(PartialEq, Debug)]
pub enum OptVal {
    Num(i64),
    FNum(f64),
    Str(String),
    Arr(HashSet<(String, String)>),
    Empty,
    Unmatch,
}
#[derive(Debug)]
pub struct OptError {
    cause: String,
}
impl fmt::Display for OptError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Err: {}", self.cause)
    }
}
impl std::error::Error for OptError {}
pub struct CliOpt {
    t: OptTyp,
    v: Option<OptVal>,
    nme: String,
    descr: Option<String>,
}
#[allow(clippy::upper_case_acronyms)]
pub struct CLI {
    args: Vec<String>,
    opts: Vec<CliOpt>,
    descr: Option<String>,
    unprocessed: bool,
}
impl CLI {
    pub fn new() -> Self {
        CLI {
            args: vec![],
            opts: vec![],
            descr: None,
            unprocessed: true,
        }
    }

    pub fn opt(&mut self, name: &str, t: OptTyp) -> Result<&mut Self, OptError> {
        if !self.unprocessed {
            return Err(OptError {
                cause: format!("the option {name} can't be set after parsing arguments"),
            });
        }
        for opt in &self.opts {
            if opt.nme == name {
                return Err(OptError {
                    cause: format!("repeating option {name}"),
                });
            }
        }
        self.opts.push(CliOpt {
            t,
            nme: name.to_string(),
            descr: None,
            v: None,
        });
        Ok(self)
    }

    pub fn description(&mut self, descr: &str) -> &mut Self {
        match self.opts.last_mut() {
            Some(element) => element.descr = Some(descr.to_string()),
            _ => self.descr = Some(descr.to_string()),
        }
        self
    }

    pub fn get_description(&self) -> Option<String> {
        let mut descr = String::new();
        if let Some(some_descr) = &self.descr {
            descr += some_descr
        }
        for opt in &self.opts {
            descr += &format!("\n{OPT_PREFIX}{}", opt.nme);
            if let Some(some_descr) = &opt.descr {
                descr += &format!("\t{some_descr}")
            }
        }
        if descr.is_empty() {
            None
        } else {
            Some(descr)
        }
    }

    pub fn get_opt(&mut self, name: &str) -> Option<&OptVal> {
        if self.unprocessed {
            self.parse()
        }
        for opt in &self.opts {
            if opt.nme == name {
                return opt.v.as_ref();
            }
        }
        None
    }

    pub fn args(&mut self) -> &Vec<String> {
        if self.unprocessed {
            self.parse()
        }
        &self.args
    }

    fn parse(&mut self) {
        let mut args = env::args();
        args.next(); // swallow first
        while let Some(arg) = args.next() {
            if let Some(sarg) = arg.strip_prefix(OPT_PREFIX) {
                // TODO eat extra -'s
                for opt in &mut self.opts {
                    if opt.nme == sarg {
                        match opt.t {
                            OptTyp::Num => {
                                if let Some(val) = args.next() {
                                    match val.parse::<i64>() {
                                        Ok(num) => opt.v = Some(OptVal::Num(num)),
                                        _ => opt.v = Some(OptVal::Unmatch),
                                    }
                                }
                            }
                            OptTyp::None => opt.v = Some(OptVal::Empty),
                            OptTyp::FNum => {
                                if let Some(val) = args.next() {
                                    match val.parse::<f64>() {
                                        Ok(num) => opt.v = Some(OptVal::FNum(num)),
                                        _ => opt.v = Some(OptVal::Unmatch),
                                    }
                                }
                            }
                            OptTyp::Str => {
                                if let Some(str) = args.next() {
                                    opt.v = Some(OptVal::Str(str))
                                }
                            }
                            OptTyp::InStr => (),
                        }
                    } else if opt.t == OptTyp::InStr && sarg.starts_with(&opt.nme) {
                        if opt.v.is_none() {
                            opt.v = Some(OptVal::Arr(HashSet::new()))
                        }
                        match &mut opt.v {
                            &mut Some(OptVal::Arr(ref mut set)) => {
                                if let Some(pair) =
                                    sarg.strip_prefix(&opt.nme).unwrap().split_once('=')
                                {
                                    set.insert((pair.0.to_string(), pair.1.to_string()));
                                } else {
                                    set.insert((
                                        sarg.strip_prefix(&opt.nme).unwrap().to_string(),
                                        String::new(),
                                    ));
                                }
                            }
                            _ => {
                                // somehow to report data inconsistency
                                opt.v = Some(OptVal::Arr(HashSet::new()))
                            }
                        }
                    } else if opt.nme.len() == 1 && sarg.contains(&opt.nme) && opt.t == OptTyp::None
                    {
                        opt.v = Some(OptVal::Empty);
                    }
                }
            } else {
                self.args.push(arg)
            }
        }
        self.unprocessed = false
    }
}
