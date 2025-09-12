//#![feature(if_let_guard)]
//#![feature(let_chains)]

pub use param::Param;
pub use web::PageOps;
pub use web::html_encode;
pub use web::Menu;
pub use web::{get_file_modified, json_encode, sanitize_path, save_props,
  is_git_covered, list_files, url_encode};

pub use web::new_cookie_header;
pub use param::{http_format_time, has_root};

pub mod web;
mod template;
pub mod param;

extern crate simtime;