use std::path::Path;

static CARGO_MANIFEST_DIR: &str = env!("CARGO_MANIFEST_DIR");

pub(crate) static LANGUAGE_ZIP_DATA: &[u8] = include_bytes!("../assets/language/language.zip");

pub fn project_dir() -> &'static Path {
    Path::new(CARGO_MANIFEST_DIR)
}

pub fn assets_dir() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/assets"))
}

pub fn backup_dir() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/backup"))
}

pub fn language_zip() -> &'static Path {
    Path::new("language.zip")
}

pub fn temporary_dir() -> &'static Path {
    Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "temporary"))
}

pub fn language_dir() -> &'static Path {
    Path::new("language")
}

pub fn hans_dir() -> &'static Path {
    Path::new("language/zh_cn_hans")
}

pub fn data_dir() -> &'static Path {
    Path::new("data")
}

pub fn alien_isolation_dir() -> &'static Path {
    Path::new("/Users/bppleman/Library/Application Support/Steam/steamapps/common/Alien Isolation/AlienIsolationData")
}
