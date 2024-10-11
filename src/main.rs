mod manifest;
#[allow(unused)]
mod path_structure;

use crate::manifest::{Manifest, ManifestItem};
use clap::{Parser, Subcommand};
use color_eyre::eyre::{eyre, Context};
use color_eyre::Result;
use std::collections::HashMap;
use std::io::{Cursor, Write};
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

static WHITE_LIST: &str = include_str!("../assets/white_list.txt");

#[derive(Debug, Parser)]
pub struct Alien {
    #[command(subcommand)]
    language: Language,
}

#[derive(Default, Debug, Clone, Subcommand)]
pub enum Language {
    #[default]
    #[command(name = "zh")]
    Chinese,
    #[command(name = "en")]
    English,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    tracing_subscriber::fmt::init();

    let alien = Alien::parse();

    let mut manifest = Manifest::read_from_language_zip()?;
    match alien.language {
        Language::Chinese => {
            let filtered = manifest.filter_hans_dir();
            check_manifest_for_game_data(&filtered)?;
            backup_alien_isolation_data(&filtered).await?;
            chinese(filtered).await?;
        }
        Language::English => {
            let needs_remove = manifest;
            let manifest = Manifest::read_from_backup_zip()?;
            english(manifest, needs_remove).await?;
        }
    }
    Ok(())
}

fn check_manifest_for_game_data(filtered: &[(&mut ManifestItem, PathBuf)]) -> Result<()> {
    let instant = std::time::Instant::now();
    let alien_isolation_dir = path_structure::alien_isolation_dir();
    tracing::info!(
        "Checking manifest for game data [{}]",
        alien_isolation_dir.display()
    );
    let white_list = WHITE_LIST.lines().collect::<Vec<_>>();
    let not_found = filtered
        .iter()
        .flat_map(|(_, striped)| {
            let path = alien_isolation_dir.join(striped);
            if striped.components().count() > 0
                && !path.exists()
                && !white_list.contains(&striped.display().to_string().as_str())
            {
                Some(striped.to_path_buf())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    if !not_found.is_empty() {
        let mut first = Err(eyre!("File missing"));
        for path in not_found.into_iter() {
            first = first.with_context(|| path.display().to_string());
        }
        return first;
    }
    tracing::info!(
        "Checked manifest for game data take {:?}",
        instant.elapsed()
    );
    Ok(())
}

async fn backup_alien_isolation_data(filtered: &[(&mut ManifestItem, PathBuf)]) -> Result<()> {
    let instant = std::time::Instant::now();
    let alien_isolation_dir = path_structure::alien_isolation_dir();
    let backup_dir = path_structure::backup_dir();
    let backup_zip = backup_dir.join(path_structure::language_zip());
    tracing::info!(
        "Backing up [{}] to [{}]",
        alien_isolation_dir
            .join(path_structure::data_dir())
            .display(),
        backup_zip.display(),
    );
    let mut data_buffer = Cursor::new(Vec::new());
    let mut archive = ZipWriter::new(&mut data_buffer);
    let white_list = WHITE_LIST.lines().collect::<Vec<_>>();

    let buffers = futures::future::join_all(
        filtered
            .iter()
            .map(|(_, striped)| (alien_isolation_dir.join(striped), striped))
            .map(|(path, striped)| {
                let white_list = white_list.clone();
                async move {
                    let metadata = match tokio::fs::metadata(&path).await {
                        Ok(metadata) => metadata,
                        Err(error) => {
                            return if error.kind() == std::io::ErrorKind::NotFound
                                && white_list.contains(&striped.display().to_string().as_str())
                            {
                                Ok(None)
                            } else {
                                Err(error)
                            }
                        }
                    };
                    if metadata.is_file() {
                        let buffer = tokio::fs::read(&path).await?;
                        Ok(Some((striped, Some(buffer))))
                    } else {
                        Ok(Some((striped, None)))
                    }
                }
            }),
    )
    .await;

    let mut buffer_map = buffers
        .into_iter()
        .map(|it| Ok(it?))
        .collect::<Result<Vec<Option<_>>>>()?
        .into_iter()
        .flatten()
        .collect::<HashMap<_, _>>();

    for (_, striped) in filtered.iter() {
        let buffer = match buffer_map.get_mut(striped) {
            Some(buffer) => buffer,
            None => continue,
        };
        match buffer {
            None => {
                archive
                    .add_directory(striped.display().to_string(), SimpleFileOptions::default())
                    .with_context(|| {
                        format!("Failed to add directory [{}] to archive", striped.display())
                    })?;
            }
            Some(buffer) => {
                archive
                    .start_file(striped.display().to_string(), SimpleFileOptions::default())
                    .with_context(|| {
                        format!("Failed to start file [{}] in archive", striped.display())
                    })?;
                archive.write_all(buffer).with_context(|| {
                    format!("Failed to write [{}] to archive", striped.display())
                })?;
            }
        }
    }
    archive.finish()?;

    let mut data_zip = tokio::fs::File::create(backup_zip).await?;
    data_buffer.set_position(0);
    tokio::io::copy(&mut data_buffer, &mut data_zip).await?;

    tracing::info!("Backed up take {:?}", instant.elapsed());
    Ok(())
}

async fn chinese(mut filtered: Vec<(&mut ManifestItem, PathBuf)>) -> Result<()> {
    tracing::info!("Converting to Chinese");
    let instant = std::time::Instant::now();
    let alien_isolation_dir = path_structure::alien_isolation_dir();

    let result: Vec<Result<()>> =
        futures::future::join_all(filtered.iter_mut().map(|(item, striped)| async move {
            let path = alien_isolation_dir.join(striped);
            write_file(item, path).await
        }))
        .await;
    result.into_iter().collect::<Result<Vec<_>>>()?;

    tracing::info!("Converted to Chinese take {:?}", instant.elapsed());
    Ok(())
}

async fn english(mut manifest: Manifest, mut needs_remove: Manifest) -> Result<()> {
    let instant = std::time::Instant::now();
    tracing::info!("Restore to English");
    let filtered = needs_remove.filter_hans_dir();
    let needs_remove_dir_len = filtered.iter().filter(|(item, _)| item.is_dir).count();
    let manifest_dir_len = manifest.iter().filter(|item| item.is_dir).count();
    if needs_remove_dir_len != manifest_dir_len {
        return Err(eyre!(
            "needs remove dir len [{}] not equal to manifest dir len [{}]",
            needs_remove_dir_len,
            manifest_dir_len
        ));
    }
    let alien_isolation_dir = path_structure::alien_isolation_dir();
    let result = futures::future::join_all(
        filtered
            .into_iter()
            .filter(|(item, _)| item.is_file)
            .map(|(_, striped)| alien_isolation_dir.join(striped))
            .map(|path| async move {
                if let Ok(metadata) = tokio::fs::metadata(&path).await {
                    if metadata.is_file() {
                        tokio::fs::remove_file(&path).await?;
                    }
                }
                Ok(())
            }),
    )
    .await;
    result.into_iter().collect::<Result<Vec<_>>>()?;

    let result: Vec<Result<()>> =
        futures::future::join_all(manifest.iter_mut().map(|item| async move {
            let path = alien_isolation_dir.join(&item.lowercase_name);
            write_file(item, path).await
        }))
        .await;
    result.into_iter().collect::<Result<Vec<_>>>()?;

    tracing::info!("Restored to English take {:?}", instant.elapsed());
    Ok(())
}

async fn write_file(item: &mut ManifestItem, path: PathBuf) -> Result<()> {
    if item.is_file {
        let parent = path
            .parent()
            .ok_or(eyre!("{} not found parent", path.display()))?;
        if matches!(tokio::fs::try_exists(parent).await, Ok(true)) {
            tokio::fs::create_dir_all(parent).await?;
        }
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)
            .await?;
        file.write_all(&item.bytes).await?;
    } else if matches!(tokio::fs::try_exists(&path).await, Ok(true)) {
        tokio::fs::create_dir_all(path).await?;
    }
    Ok(())
}
