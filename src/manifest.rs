use crate::path_structure;
use color_eyre::Result;
use std::fmt::{Debug, Display, Formatter};
use std::io::{Cursor, Read, Seek};
use std::ops::{Deref, DerefMut};
use std::path::PathBuf;
use zip::read::ZipFile;
use zip::ZipArchive;

pub struct Manifest(Vec<ManifestItem>);

impl Deref for Manifest {
    type Target = Vec<ManifestItem>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for Manifest {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Display for Manifest {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for item in &self.0 {
            writeln!(f, "{}", item)?;
        }
        Ok(())
    }
}

impl Manifest {
    pub fn new<T: Read + Seek>(mut archive: ZipArchive<T>) -> Result<Self> {
        let len = archive.len();
        let items = (0..len)
            .into_iter()
            .map(|i| {
                let file = archive.by_index(i)?;
                Ok(ManifestItem::new(file))
            })
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect();
        Ok(Self(items))
    }

    pub fn read_from_language_zip() -> Result<Manifest> {
        let instant = std::time::Instant::now();
        tracing::info!(
            "Read manifest from bytes: {}",
            path_structure::LANGUAGE_ZIP_DATA.len()
        );
        let cursor = Cursor::new(path_structure::LANGUAGE_ZIP_DATA);
        let archive = ZipArchive::new(cursor)?;
        let manifest = Manifest::new(archive)?;
        tracing::info!("Read manifest in {:?}", instant.elapsed());
        Ok(manifest)
    }

    pub fn read_from_backup_zip() -> Result<Manifest> {
        let instant = std::time::Instant::now();
        let backup_zip = path_structure::backup_dir().join(path_structure::language_zip());
        tracing::info!("Read manifest from [{}]", backup_zip.display());
        let cursor = Cursor::new(std::fs::read(backup_zip)?);
        let archive = ZipArchive::new(cursor)?;
        let manifest = Manifest::new(archive)?;
        tracing::info!("Read manifest in {:?}", instant.elapsed());
        Ok(manifest)
    }

    pub fn filter_hans_dir(&mut self) -> Vec<(&mut ManifestItem, PathBuf)> {
        let instant = std::time::Instant::now();
        let hans_dir = path_structure::hans_dir();
        tracing::info!("Filtering for [{}]", hans_dir.display());
        let filtered = self
            .iter_mut()
            .filter(|item| item.lowercase_name.starts_with(hans_dir))
            .flat_map(|item| {
                let striped = item
                    .lowercase_name
                    .strip_prefix(hans_dir)
                    .ok()?
                    .to_path_buf();
                Some((item, striped))
            })
            .filter(|(_, striped)| striped.components().count() > 0)
            .collect::<Vec<_>>();
        tracing::info!(
            "Filtered {} items in {:?}",
            filtered.len(),
            instant.elapsed()
        );
        filtered
    }
}

pub struct ManifestItem {
    pub path: PathBuf,
    pub lowercase_name: PathBuf,
    pub bytes: Vec<u8>,
    pub is_file: bool,
    pub is_dir: bool,
}

impl ManifestItem {
    fn new(file: ZipFile<'_>) -> Option<Self> {
        let is_file = file.is_file();
        let is_dir = file.is_dir();
        let path = file.enclosed_name()?;
        let lowercase_name = PathBuf::from(path.display().to_string().to_lowercase());
        let bytes = file
            .bytes()
            .into_iter()
            .map(|it| Ok(it?))
            .collect::<Result<Vec<_>>>()
            .ok()?;
        Some(Self {
            path,
            lowercase_name,
            bytes,
            is_file,
            is_dir,
        })
    }
}

impl Debug for ManifestItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let flag = if self.is_file { "F" } else { "D" };
        write!(f, "[{}] {:?}", flag, self.path)
    }
}

impl Display for ManifestItem {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let flag = if self.is_file { "F" } else { "D" };
        write!(f, "[{}] {}", flag, self.path.display())
    }
}
