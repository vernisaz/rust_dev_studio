//use std::collections::HashMap;
//use std::env;
use std::fs::File;
//use std::io;
use std::io::prelude::*;

use crate::crossref::LexState::{InCallName, InParams, InCallParams, InKW, InName, Start, ExpInName, 
     InColSep, ExPNamSep, InNum, ExpInCallName, ExpInStruct, ExpInEnum, InDataDef, InStruct, InEnum,
     ExpImplName, InImplName, InExpFor, InForName, InForKW, ExpInForName, InExpOpenImpl, StartInScope,
     InTraitName, ExpInTraitName, ExpFnBody, InFnBody, ExpDirect, Direct, InComment, ExpComment, InStarComment,
     ExpEndComment, InGenTypeOrComp,
};

const BUF_SIZE: usize = 1024;

#[allow(dead_code)]
#[derive(Debug, PartialEq, Default, Clone)]
pub enum RefType {
    #[default] 
    Function, // add scope as impl of or impl trait for
    Variable,
    Data, // like struct or type
    Impl, // impl something for something
    Access,
}

#[derive(Debug, PartialEq, Default, Clone)]
pub enum ScopeType {
    #[default] 
    SelfImpl,
    TraitFor,
    Trait
}

#[derive(Debug, Clone)]
pub struct Scope {
    pub name: String,
    pub name_for: Option<String>,
    pub type_of_scope: ScopeType
}

#[derive(Debug, Clone)]
pub struct Reference {
    pub name: String,
    pub src: String,
    pub line: usize,
    pub column: u16,
    pub type_of_use: RefType,
    pub scope: Option<Scope>
}

pub struct Reader {
    buf: [u8; BUF_SIZE],
    pos: usize,
    end: usize,
    line: usize,
    line_offset: u16,
    pub path: String,
    file: File,
}

impl Reader {
    fn next(&mut self) -> Option<char> {
        self.pos += 1;
        if self.pos >= self.end {
            self.end = self.file.read(&mut self.buf).unwrap_or(0);
            if self.end == 0 { return None }
            self.pos = 0
        }
        self.line_offset += 1;
        // check if it can be UTF8
        let mut byte: u32 = self.buf[self.pos] as u32;
        if (byte & 0b1000_0000) != 0 {
            // UTF8
            let mut num_byte = if (byte & 0b1111_0000) == 0b1111_0000 {
                byte &= 0b0000_0111;
                3
            } else if (byte & 0b1110_0000) == 0b1110_0000 {
                byte &= 0b0000_1111;
                2
            } else if (byte & 0b1100_0000) == 0b1100_0000 {
                byte &= 0b0001_1111;
                1
            } else {
                0
            };

            let mut c32: u32 = byte;
            while num_byte > 0 {
                self.pos += 1;
                if self.pos >= self.end {
                    self.end = self.file.read(&mut self.buf).unwrap_or(0);
                    if self.end == 0 {
                        return None;
                    }
                    self.pos = 0
                }
                c32 = (c32 << 6) | ((self.buf[self.pos] as u32) & 0b0011_1111);
                num_byte -= 1
            }
            self.line_offset += 1;
            return Some(std::char::from_u32(c32).unwrap_or(std::char::REPLACEMENT_CHARACTER))
        }
        if self.buf[self.pos] == line_separator() {
            self.line += 1;
            self.line_offset = 0
        } else {
            self.line_offset += 1
        }
        Some(char::from(self.buf[self.pos]))
    }
}

pub fn scan_file(file: &impl AsRef<str>) -> Vec< Reference> {
    let path = file.as_ref();
    let mut r = Reader {
        file: File::open(path).unwrap(),
        path: path.to_string(),
        line: 1,
        buf: [0; 1024],
        pos: 0,
        end: 0,
        line_offset: 0,
    };
    scan(&mut r)
}

#[derive(PartialEq, Debug, Copy, Clone)]
enum LexState {
    Start,
    InKW,
    InName,
    ExpInName,
    ExpInCallName,
    InCallName,
    InCallParams,
    InParams,
    InNum,
    ExPNamSep,
    InColSep,
    ExpInEnum,
    ExpInStruct,
    InEnum,
    InStruct,
    InDataDef,
    ExpImplName,
    InImplName,
    InExpFor, InForName,
    InForKW,
    ExpInForName, InExpOpenImpl, 
    
    ExpInTraitName,
    InTraitName,
    ExpFnBody,
    InFnBody,
    
    StartInScope,
    ExpDirect,
    Direct,
    
    ExpComment,
    InStarComment,
    ExpEndComment,
    InComment,
    
    InGenTypeOrComp,
    //VarOrFn,
}

pub fn scan(reader: &mut Reader) -> Vec< Reference> {
    let mut res = Vec::new();
    let mut state = Start;
    let mut co = reader.next();
    let mut name = String::from("");
    let mut scope = Scope {
        name: String::from(""),
        name_for: None,
        type_of_scope : Default::default()
    };
    let mut cbracket_cnt : u16 = Default::default();
    let mut prev_state = Vec::new();
    while let Some(c) = co {
        match c {
            '"' => {} // TODO add processing
            'a'..='z' | 'A'..='Z' | '_' => {
                //eprintln!{"state in curr {state:?}"}
                if state == ExpComment {
                    state = prev_state.pop().unwrap()
                }
                match state {
                Start | StartInScope  => {name.push(c); state = InKW} //| InFnBody
                ExpInName  => {state = InName; name.push(c) }
                ExPNamSep | InColSep | ExpInCallName | InFnBody | ExpDirect | InCallParams => {state = InCallName; name.push(c) }
                ExpInEnum => {state = InEnum; name.push(c);
                    //eprintln!{"state in car {state:?} at {}:{}", reader.line, reader.line_offset}
                }
                ExpInStruct => {state = InStruct; name.push(c) }
                InName | InKW | InCallName | InImplName | InTraitName | InForKW | InForName => name.push(c),
                ExpImplName => {state = InImplName; name.push(c)}
                InExpFor => {state = InForKW; name.push(c)}
                ExpInTraitName => {state = InTraitName; name.push(c)}
                ExpInForName => {state = InForName; name.push(c)}
                
                _ => (),
               
                }
            }
            '0'..='9' => {
                //eprintln!{"state in dig {state:?}"}
                if state == ExpComment {
                    state = prev_state.pop().unwrap()
                }
                match state {
                Start | StartInScope => {state = InNum}
                ExpInName  => {state = InNum;  }
                ExPNamSep | InColSep => {state = InNum }
                InName | InKW | InCallName | InStruct |
                InEnum | InImplName | InForKW | InForName | InTraitName => name.push(c),
                _ => (),
               
                }
            }
            '(' => { //eprintln!{"state ( {state:?}, name={name}"}
                if state == ExpComment {
                    state = prev_state.pop().unwrap()
                }
                match state {
                   InName => {
                        let fn_def = Reference {
                        name: name.to_owned(),
                        src: reader.path.to_owned(),
                        line: reader.line,
                        column: reader.line_offset,
                        type_of_use: RefType::Function,
                        scope: if scope.name . is_empty() {None} else {Some(scope.clone())}
                        };
                        res.push(fn_def);
                        state = InParams
                    }
                    InCallName | InKW => {
                         let fn_cal = Reference {
                        name: name.to_owned(),
                        src: reader.path.to_owned(),
                        line: reader.line,
                        column: reader.line_offset,
                        type_of_use: RefType::Access,
                        scope: None // it needs to be quilified
                        };
                        res.push(fn_cal);
                        state = InCallParams
                    }
                   /* InKW => {
                        name.clear();
                        // check ??? KW
                        state = InCallParams
                    }*/
                    _ => ()
                }
                name.clear();
            }
            '<' => {
                if state == ExpComment {
                    state = prev_state.pop().unwrap()
                }
                match state {
                    InName | InCallName => {
                        prev_state.push(state);
                        state = InGenTypeOrComp
                    }
                    _ => ()
                }
            }
            ' ' | '\r' | '\n' | '\t' => {
                if state == ExpComment {
                    state = prev_state.pop().unwrap()
                }
                match state {
                    Start | StartInScope => {state = InKW; name.clear()}
                    InKW => {
                        //eprintln!{"state KW {name} at {}:{}", reader.line, reader.line_offset}
                        match name.as_str() {
                            "fn" => state = ExpInName,
                            "enum" => state = ExpInEnum,
                            "struct" => state = ExpInStruct,
                            "trait" => state = ExpInTraitName,
                            "impl" => state = ExpImplName,
                            // eventually all reserved words from https://doc.rust-lang.org/reference/keywords.html
                            _ => state = Start
                        }
                        name.clear()
                    }
                    InForKW => {
                        match name.as_str() {
                            "for" => state = ExpInForName,
                            _ => state = InExpOpenImpl
                        }
                        name.clear()
                    }
                    InImplName => {
                        state = InExpFor ;
                        scope.name.replace_range(.., &name.to_string());
                        name.clear()
                    }
                    InName | InCallName => name.clear(),
                    InColSep => state = Start,
                    InComment if c == '\n' => {state = prev_state.pop().unwrap();}
                    _ => (),
                };
                
            }
            '.' | '&' => {
                if state == ExpComment {
                    state = prev_state.pop().unwrap()
                }
                match state {
                    InCallName | InKW | InGenTypeOrComp => {
                    // just chain of struct names
                         name.clear();
                         state = InCallName;
                    }
                    _ => (),
                }
            }
            ';' => {
                match state {
                    //ExpComment => state = prev_state.pop().unwrap(),
                    _ => (),
                }
                //let temp = state;
                if scope.name.is_empty() {
                    state = Start
                } else {
                    state = StartInScope
                }
               // eprintln!{"state befor semi {temp:?} after {state:?} at {}:{}", reader.line, reader.line_offset}
            }
            ':' => {
                if state == ExpComment {
                    state = prev_state.pop().unwrap()
                }
                match state {
                    InKW => {
                        //name.clear();
                        state = InColSep
                    }
                    InName => {
                        // copy the current name in the scope quilifier
                        //name.clear();
                        state = InColSep
                    }
                    InColSep => {name.clear(); state = ExpInCallName}
                    _ => (),
                }
                name.clear()
            }
            ',' => {
                if state == ExpComment {
                    state = prev_state.pop().unwrap()
                }
               // eprintln!{"state befor comma {state:?} at {}:{}", reader.line, reader.line_offset}
                match state {
                    InDataDef | Start | Direct | ExpDirect => (),
                    InCallName => name.clear(),
                     _ => state = ExpInCallName
                }
            }
            '{' => { //eprintln!{"state {{ {state:?}"}
                if state == ExpComment {
                    state = prev_state.pop().unwrap()
                }
                match state {
                   InStruct => {
                        let struct_def = Reference {
                        name: name.to_owned(),
                        src: reader.path.to_owned(),
                        line: reader.line,
                        column: reader.line_offset,
                        type_of_use: RefType::Data,
                        scope: None
                        };
                        res.push(struct_def);
                        state = InDataDef;
                    }
                    InEnum => {
                         let enum_def = Reference {
                        name: name.to_owned(),
                        src: reader.path.to_owned(),
                        line: reader.line,
                        column: reader.line_offset,
                        type_of_use: RefType::Data,
                        scope: None
                        };
                        res.push(enum_def);
                        state = InDataDef;
                    }
                    InForName => {
                        scope.name_for = Some(name.to_owned());
                        scope.type_of_scope = ScopeType::TraitFor;
                        state = StartInScope 
                    }
                    InTraitName => {
                        scope.name = name.to_owned();
                        scope.name_for = None;
                        scope.type_of_scope = ScopeType::Trait;
                        state = StartInScope
                    }
                    InExpOpenImpl  => {
                        state = StartInScope
                    }
                    ExpFnBody => state = InFnBody,
                    InFnBody => cbracket_cnt += 1,
                    ExpDirect => {state = InFnBody; cbracket_cnt += 1},
                    _ => state = {
                    // TODO activate brackets logic
                        //cbracket_cnt += 1;
                        if scope.name.is_empty() {Start} else {StartInScope}
                    }
                }
                name.clear()
                
            }
            ')' => {
                if state == ExpComment {
                    state = prev_state.pop().unwrap()
                }
                match state {
                    InParams => {
                        state = ExpFnBody;
                    }
                    InCallParams => {
                        state = ExPNamSep;
                    }
                    _ => (),
                }
                name.clear()
            }
            '}' => {
                if state == ExpComment {
                    state = prev_state.pop().unwrap()
                }
               //eprintln!{"state {state:?} at closing }} balance: {cbracket_cnt} at {}:{}", reader.line, reader.line_offset}
               match state {
                    InDataDef => {
                        scope.name_for = None;
                        scope.name.clear();
                        scope.type_of_scope = Default::default();
                        name.clear();
                        state = Start
                    }
                    InFnBody | ExPNamSep | InCallName | InNum | ExpDirect | ExpInCallName => {
                        if cbracket_cnt > 0 {state = StartInScope; cbracket_cnt -= 1} else {
                         state = StartInScope
                        }
                    }
                    Start => if cbracket_cnt > 0 {state = StartInScope; cbracket_cnt -= 1},
                    InKW => if cbracket_cnt > 0 {state = StartInScope; cbracket_cnt -= 1} else {state = Start},
                   // ExPNamSep => 
                    _ => (),// eprintln!{"state {state:?} at closing }} balance: {cbracket_cnt} at {}:{}", reader.line, reader.line_offset},
                } 
            }
            '=' => {
                //eprintln!{"state = {state:?} name={name}"}
                if state == ExpComment {
                    state = prev_state.pop().unwrap()
                }
                match state {
                    ExpEndComment => state = InStarComment,
                    //InCallName => state = ExpDirect,
                    Direct | InComment => (),
                    _ => state = InCallName,
                }
                name.clear();
            }
            /*'&' | '|' => {
                if state == ExpComment {
                    state = prev_state.pop().unwrap()
                }
                match state {
                    ExPNamSep => state = InFnBody,
                    _ => ()
                }
            }*/
            '>' => { //eprintln!{"state > {state:?} name={name}"}
                if state == ExpComment {
                    state = prev_state.pop().unwrap()
                }
                match state {
                    InCallName => {state = ExpDirect},
                    InGenTypeOrComp => state = prev_state.pop().unwrap(),
                     _ => ()
                }
            }
            '#' => {
                if state == ExpComment {
                    state = prev_state.pop().unwrap()
                }
                match state {
                    Start => {state = ExpDirect},
                     _ => ()
                }
            }
            '[' => {
                if state == ExpComment {
                    state = prev_state.pop().unwrap()
                }
                match state {
                    ExpDirect => state = Direct,
                     _ => ()
                }
            }
            ']' => {
                if state == ExpComment {
                    state = prev_state.pop().unwrap()
                }
                match state {
                    Direct => state = Start,
                     _ => ()
                }
            }
            '/' => {
                match state {
                    ExpComment => state = InComment,
                    ExpEndComment => state = prev_state.pop().unwrap(),
                     _ => {prev_state.push(state); state = ExpComment;}
                }
            }
            '*' => {
                match state {
                    ExpComment => state = InStarComment,
                    InStarComment => state = ExpEndComment,
                     _ => ()
                }
            }
            _ => match state {
                
                _ => (),
            }
        }
        co = reader.next()
    }
    res
}

#[cfg(win)]
fn line_separator() -> u8 {
  13
}

#[cfg(not(win))]
fn line_separator() -> u8 {
  10
}



