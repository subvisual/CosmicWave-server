use std::time::{SystemTime, UNIX_EPOCH};

use reqwest::StatusCode;
use rocket::serde::{Deserialize, Serialize};
use serde_json::Value;

#[macro_use]
extern crate rocket;

const BASE_URL: &str = "https://testnet.polybase.xyz/v0/collections";
const COLLECTION_PATH: &str = "pk%2F0x98550a271a85832718f29cf70384e551b852ada0beec830f9c682b7de22d945ad828dbc50de17194936565d27ef6da583c8e8856d7f27bbd97b34419401e5b47%2FSoundverseTest3";

type Streamer = ForeignKey;

#[derive(Debug, Serialize, Deserialize)]
struct ForeignKey {
    id: String,
    #[serde(alias = "collectionId")]
    collection_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct Song {
    id: String,
    title: String,
    artist: String,
    filename: String,
    duration: f64,
    owner: Streamer,
}

#[derive(Debug, Serialize, Deserialize)]
struct Playlist {
    id: String,
    songs: Vec<ForeignKey>,
    owner: Streamer,
}

#[derive(Debug, Serialize, Deserialize)]
struct ActivePlaylist {
    id: String,
    playlist: ForeignKey,
    playing: bool,
    owner: Streamer,
    #[serde(alias = "startTimestamp")]
    start_timestamp: f64,
}

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

#[get("/playlist")]
async fn current_playlist() {
    let response = fetch_playlist().await;

    match response {
        Some(playlist) => {
            println!("{:?}", playlist);
        }
        _ => println!("nothing"),
    };
}

#[get("/playlist/<id>")]
async fn playlist(id: String) {
    let response = fetch_playlist_by(id).await;

    match response {
        Some(playlist) => {
            println!("{:?}", playlist);
        }
        _ => println!("nothing"),
    };
}

#[get("/playlist/<id>/songs")]
async fn playlist_songs(id: String) {
    let response = fetch_playlist_songs_by(id).await;

    match response {
        Some(songs) => {
            println!("{:?}", songs);
        }
        _ => println!("nothing"),
    };
}

#[get("/song/<id>")]
async fn song(id: String) {
    let response = fetch_song(id).await;

    match response {
        Some(song) => {
            println!("{:?}", song);
        }
        _ => println!("nothing"),
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct NowPlayingResponse {
    total_duration: String,
    current_timestamp: String,
    song_cids: Vec<String>,
}

impl NowPlayingResponse {
    fn new(total_duration: String, current_timestamp: String, song_cids: Vec<String>) -> Self {
        Self {
            total_duration,
            current_timestamp,
            song_cids,
        }
    }
}

#[get("/now")]
async fn now_playing() -> String {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    if let Some(active_playlist) = fetch_playlist().await {
        if let Some(all_playlist_songs) = fetch_playlist_songs_by(active_playlist.playlist.id).await
        {
            let total_playlist_duration = all_playlist_songs
                .iter()
                .fold(0.0, |acc, song| acc + song.duration);

            let song_cids = all_playlist_songs
                .iter()
                .map(|song| song.id.clone())
                .collect::<Vec<String>>();

            serde_json::to_string(&NowPlayingResponse::new(
                total_playlist_duration.to_string(),
                now.to_string(),
                song_cids,
            ))
            .unwrap()
        } else {
            serde_json::to_string(&NowPlayingResponse::default()).unwrap()
        }
    } else {
        serde_json::to_string(&NowPlayingResponse::default()).unwrap()
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount(
        "/",
        routes![
            index,
            current_playlist,
            playlist,
            song,
            playlist_songs,
            now_playing
        ],
    )
}

async fn fetch_playlist() -> Option<ActivePlaylist> {
    let client = reqwest::Client::new();

    let url = format!("{}/{}", BASE_URL, COLLECTION_PATH);

    let response = client
        .get(format!("{}%2FActivePlaylist/records", url))
        .send()
        .await
        .unwrap();

    match response.status() {
        StatusCode::OK => {
            let json_data: Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();

            let playlist =
                serde_json::from_value::<ActivePlaylist>(json_data["data"][0]["data"].clone())
                    .unwrap();

            Some(playlist)
        }
        _ => None,
    }
}

async fn fetch_playlist_by(id: String) -> Option<Playlist> {
    let client = reqwest::Client::new();

    let url = format!("{}/{}", BASE_URL, COLLECTION_PATH);

    let response = client
        .get(format!("{}%2FPlaylist/records/{}", url, id))
        .send()
        .await
        .unwrap();

    match response.status() {
        StatusCode::OK => {
            let json_data: Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();

            let playlist = serde_json::from_value::<Playlist>(json_data["data"].clone()).unwrap();

            Some(playlist)
        }
        _ => None,
    }
}

async fn fetch_playlist_songs_by(id: String) -> Option<Vec<Song>> {
    match fetch_playlist_by(id).await {
        Some(playlist) => {
            let song_ids: Vec<String> = playlist.songs.iter().map(|s| s.id.clone()).collect();

            let mut songs = Vec::new();

            for id in song_ids {
                if let Some(song) = fetch_song(id).await {
                    songs.push(song);
                }
            }

            Some(songs)
        }
        None => None,
    }
}

async fn fetch_song(id: String) -> Option<Song> {
    let client = reqwest::Client::new();

    let url = format!("{}/{}", BASE_URL, COLLECTION_PATH);

    let response = client
        .get(format!("{}%2FSong/records/{}", url, id))
        .send()
        .await
        .unwrap();

    match response.status() {
        StatusCode::OK => {
            let json_data: Value = serde_json::from_str(&response.text().await.unwrap()).unwrap();

            let song = serde_json::from_value::<Song>(json_data["data"].clone()).unwrap();

            Some(song)
        }
        _ => None,
    }
}
