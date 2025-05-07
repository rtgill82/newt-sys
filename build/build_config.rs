//
// Copyright (C) 2025 Robert Gill <rtgill82@gmail.com>
//
// This file is a part of newt-sys.
//
// This library is free software; you can redistribute it and/or
// modify it under the terms of the GNU Lesser General Public
// License version 2.1 as published by the Free Software Foundation.
//
// This library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public
// License along with this library; if not, write to the Free Software
// Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA
//

use std::ffi::OsString;
use std::path::{Path,PathBuf};
use std::{env,fs};

pub struct BuildConfig {
    target: String,
    build_prefix: PathBuf,
    archive_path: PathBuf,
    src_path: PathBuf,
    install_prefix: PathBuf,
    pkg_config_path: PathBuf,
    autoconf_aux_path: Option<PathBuf>
}

impl BuildConfig {
    pub fn new(package: &str, version: &str) -> BuildConfig {
        let out_dir = env::var("OUT_DIR").unwrap();
        let version_name = format!("{}-{}", package, version);
        let build_prefix = format!("{}/build", out_dir);
        let src_path = format!("{}/{}", build_prefix, version_name);
        let install_prefix = format!("{}/install/{}", out_dir, version_name);
        let pkg_config_path = format!("{}/lib/pkgconfig", install_prefix);

        BuildConfig {
            target: target(),
            build_prefix: build_prefix.into(),
            archive_path: find_archive(&version_name).unwrap(),
            src_path: src_path.into(),
            install_prefix: install_prefix.into(),
            pkg_config_path: pkg_config_path.into(),
            autoconf_aux_path: None
        }
    }

    pub fn set_autoconf_aux_path<P: AsRef<Path>>(&mut self, path: P) {
        let mut path_buf = PathBuf::from(&self.src_path);
        path_buf.push(path);
        self.autoconf_aux_path = Some(path_buf);
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn build_prefix(&self) -> &Path {
        &self.build_prefix
    }

    pub fn archive_path(&self) -> &Path {
        &self.archive_path
    }

    pub fn src_path(&self) -> &Path {
        &self.src_path
    }

    pub fn install_prefix(&self) -> &Path {
        &self.install_prefix
    }

    pub fn pkg_config_path(&self) -> &Path {
        &self.pkg_config_path
    }

    pub fn autoconf_aux_path(&self) -> Option<&Path> {
        self.autoconf_aux_path.as_ref().map(|p| p.as_ref())
    }
}

fn file_exists<P: AsRef<Path>>(path: P) -> bool {
    if let Ok(metadata) = fs::metadata(path) {
        if metadata.is_file() {
            return true;
        }
    }
    false
}

fn find_archive(version_name: &str) -> Option<PathBuf> {
    let crate_path = env::var("CARGO_MANIFEST_DIR").unwrap();
    let extensions = [".tar.bz2", ".tar.gz"];

    let mut archive_base = OsString::from(crate_path);
    archive_base.push("/vendor/");
    archive_base.push(version_name);

    for ext in &extensions {
        let mut archive = archive_base.clone();
        archive.push(ext);
        if file_exists(&archive) {
            return Some(PathBuf::from(&archive));
        }
    }

    None
}

fn target() -> String {
    let target = env::var("TARGET").unwrap();
    match target.as_str() {
        "riscv64gc-unknown-linux-gnu" => String::from("riscv64-unknown-linux-gnu"),
        _ => target
    }
}
