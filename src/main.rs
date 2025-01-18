#!/usr/bin/env -S PATH=/Users/${USER}/.cargo/bin:${PATH} rust-script
//! mpd-status
//! <xbar.title>MPD status</xbar.title>
//! <xbar.version>v1.3</xbar.version>
//! <xbar.author>Korny Sietsma</xbar.author>
//! <xbar.author.github>kornysietsma</xbar.author.github>
//! <xbar.desc>Basic MPD widget demo</xbar.desc>
//! <xbar.dependencies>rust</xbar.dependencies>
//! <xbar.var>string(MPD_XBAR_LOG_FILE=""): path to a log file (optional)</xbar.var>
//! ```cargo
//! [dependencies]
//! anyhow = "1.0.91"
//! log = "0.4.22"
//! simplelog = "^0.12.0"
//! bitbar = "0.10.1"
//! mpd = "0.1.0"
//! ```
#![forbid(unsafe_code)]
#![warn(clippy::all)]
#![warn(rust_2018_idioms)]
// #![warn(rust_2024_compatibility)]

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;
use bitbar::ContentItem;
use bitbar::{Menu, MenuItem};
use log::warn;
use log::{info, LevelFilter};
use mpd::Client;
use simplelog::{Config, WriteLogger};
use std::env::{self};
use std::fs::{File, OpenOptions};

fn mpc_command(text: &str, command: &str) -> Result<MenuItem> {
    Ok(ContentItem::new(text)
        .command(("/opt/homebrew/bin/mpc", command))?
        .refresh()
        .into())
}

pub fn refresh(host: &str) -> Result<()> {
    let mut menuitems: Vec<MenuItem> = Vec::new();
    let maybe_conn = Client::connect(host);
    if let Err(e) = &maybe_conn {
        warn!("Error connecting to mpd: {:?}", e);
        menuitems.push(MenuItem::new("🔇".to_string()));
        menuitems.push(MenuItem::Sep);
        menuitems.push(MenuItem::new(e.to_string()));
    } else {
        let mut conn = maybe_conn.unwrap();
        let playlists = conn.playlists()?;
        let mut playlists_by_name: HashMap<String, Vec<String>> = std::collections::HashMap::new();
        let mut playlist_by_filename: HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        for playlist in &playlists {
            let songs = conn.playlist(&playlist.name)?;
            for song in songs {
                let name = playlist.name.clone();
                playlists_by_name
                    .entry(name)
                    .or_insert(Vec::new())
                    .push(song.file.clone());
                playlist_by_filename
                    .entry(song.file.clone())
                    .or_insert(Vec::new())
                    .push(playlist.name.clone());
            }
        }

        info!("Status: {:?}", conn.status());
        if let Some(song) = conn.currentsong()? {
            let title = &song.title.clone().unwrap_or("(no title)".to_string());
            let artist = &song.artist.clone().unwrap_or("(no artist)".to_string());
            '…';
            let mut label = format!("{} - {}", title, artist);
            let mut truncated = false;
            const MAX_LEN: usize = 25;
            let trunc_label = label.chars().take(MAX_LEN).collect::<String>();
            if label != trunc_label {
                label = format!("{}…", trunc_label);
                truncated = true;
            }

            menuitems.push(ContentItem::new(&label).into());
            menuitems.push(MenuItem::Sep);
            if truncated {
                menuitems.push(MenuItem::new(format!("{} - {}", title, artist)));
            }

            let mut playlists_with_current_song = Vec::new();
            if let Some(playlists) = playlist_by_filename.get(&song.file) {
                for playlist in playlists {
                    playlists_with_current_song.push(playlist.clone());
                }
            }
            playlists_with_current_song.sort();
            if !playlists_with_current_song.is_empty() {
                menuitems.push(
                    ContentItem::new("In playlists")
                        .sub(
                            playlists_with_current_song
                                .iter()
                                .map(|name| MenuItem::new(name.clone())),
                        )
                        .into(),
                );
            }

            info!("Current song: {:?}", &song);
            info!("Playlists: {:?}", playlists_with_current_song);
            menuitems.push(MenuItem::Sep);
            menuitems.push(mpc_command("pause/play", "toggle")?);
            menuitems.push(mpc_command("clear", "clear")?);
            menuitems.push(mpc_command("next", "next")?);
        } else {
            info!("Nothing playing");
            menuitems.push(MenuItem::new("♫".to_string()));
            menuitems.push(MenuItem::Sep);
            menuitems.push(mpc_command("pause/play", "toggle")?);
            menuitems.push(mpc_command("clear", "clear")?);
        }
    }
    print!("{}", Menu(menuitems));
    Ok(())
}

fn main() -> Result<()> {
    let base_dir = env::var("MPD_XBAR_LOG_FILE").ok().and_then(|filename| {
        if filename.is_empty() {
            return None;
        }
        let path = Path::new(&filename);
        let parent = path.parent()?;
        if !parent.is_dir() {
            panic!(
                "MPD_XBAR_LOG_FILE parent '{}' must be a valid directory",
                parent.to_string_lossy()
            );
        }
        Some(path.to_path_buf())
    });

    if let Some(log_file) = base_dir {
        WriteLogger::init(
            LevelFilter::Debug,
            Config::default(),
            OpenOptions::new().write(true).create(true).open(log_file)?,
        )
        .expect("Failed to create logger");
    }

    info!("Refreshing mpd-xbar");

    refresh("localhost:6600")
}
