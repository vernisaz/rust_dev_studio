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
#[cfg(any(unix, target_os = "redox"))]
use std::path::MAIN_SEPARATOR_STR;
#[cfg(target_os = "windows")]
use std::path::MAIN_SEPARATOR;

#[cfg(target_os = "windows")]
pub fn has_root(path: impl AsRef<str>) -> bool {
  let path = path.as_ref().as_bytes();
  path.len() > 3 && path[1] == b':' && path[2] == b'\\'
      || !path.is_empty() && path[0] == MAIN_SEPARATOR as _
}

#[cfg(any(unix, target_os = "redox"))]
#[inline]
pub fn has_root(path: impl AsRef<str>) -> bool {
  path.as_ref().starts_with(MAIN_SEPARATOR_STR)
}
