pub use param::Param;
pub use web::PageOps;
pub use web::Menu;
pub use web::{get_file_modified, sanitize_path, save_props,
  is_git_covered, list_files, html_encode};

pub use param::{http_format_time};

pub mod web;
mod template;
pub mod param;

extern crate simtime;