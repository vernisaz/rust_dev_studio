pub use param::Param;
pub use web::Menu;
pub use web::PageOps;
pub use web::{
    get_file_modified, html_encode, http_format_time, is_git_covered, list_files, path_info,
    sanitize_path, save_props,
};

pub mod param;
mod template;
pub mod web;

extern crate simtime;
