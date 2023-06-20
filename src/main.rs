use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::StatusCode;
use rocket::{
    fairing::{Fairing, Info, Kind},
    http::{Header, Status},
    serde::{Deserialize, Serialize},
    Request, Response,
};
use serde_json::{json, Value};

#[macro_use]
extern crate rocket;

const BASE_URL: &str = "https://testnet.polybase.xyz/v0/collections";
const COLLECTION_PATH: &str = "pk%2F0x98550a271a85832718f29cf70384e551b852ada0beec830f9c682b7de22d945ad828dbc50de17194936565d27ef6da583c8e8856d7f27bbd97b34419401e5b47%2FSoundverseTest3";

type Streamer = ForeignKey;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct ForeignKey {
    id: String,
    #[serde(alias = "collectionId")]
    collection_id: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[get("/healthz")]
fn health() -> Status {
    Status::Ok
}

#[get("/playlist")]
async fn current_playlist() -> String {
    let response = fetch_playlist().await;

    match response {
        Some(playlist) => serde_json::to_string(&playlist).unwrap(),
        _ => "".to_string(),
    }
}

#[get("/playlist/<id>")]
async fn playlist(id: String) -> String {
    let response = fetch_playlist_by(id).await;

    match response {
        Some(playlist) => serde_json::to_string(&playlist).unwrap(),
        _ => "".to_string(),
    }
}

#[get("/playlist/<id>/songs")]
async fn playlist_songs(id: String) -> String {
    let response = fetch_playlist_songs_by(id).await;

    match response {
        Some(songs) => serde_json::to_string(&songs).unwrap(),
        _ => "".to_string(),
    }
}

#[get("/song/<id>")]
async fn song(id: String) -> String {
    let response = fetch_song(id).await;

    match response {
        Some(song) => serde_json::to_string(&song).unwrap(),
        _ => "".to_string(),
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct NowPlayingResponse {
    total_duration: String,
    current_timestamp: String,
    song_cids: Vec<String>,
    current_song: serde_json::Value,
}

impl NowPlayingResponse {
    fn new(
        total_duration: String,
        current_timestamp: String,
        song_cids: Vec<String>,
        current_song: serde_json::Value,
    ) -> Self {
        Self {
            total_duration,
            current_timestamp,
            song_cids,
            current_song,
        }
    }
}

fn calculate_current_song_timestamp(
    playlist: ActivePlaylist,
    songs: Vec<Song>,
) -> serde_json::Value {
    let mut time_since_playlist_start =
        SystemTime::elapsed(&(UNIX_EPOCH + Duration::from_secs(playlist.start_timestamp as u64)))
            .unwrap()
            .as_secs();

    let song = songs.iter().find(|song| {
        if time_since_playlist_start > 0 {
            if time_since_playlist_start >= song.duration as u64 {
                time_since_playlist_start -= song.duration as u64;
                false
            } else {
                true
            }
        } else {
            true
        }
    });

    match song {
        Some(current_song) => {
            json!({
                "id": current_song.id,
                "filename": current_song.filename,
                "timestamp": current_song.duration as u64 - time_since_playlist_start
            })
        }
        None => {
            json!({"id": "", "filename": "", "timestamp": ""})
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
        if let Some(all_playlist_songs) =
            fetch_playlist_songs_by(active_playlist.playlist.id.clone()).await
        {
            let current_song = calculate_current_song_timestamp(
                active_playlist.clone(),
                all_playlist_songs.clone(),
            );

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
                current_song,
            ))
            .unwrap()
        } else {
            serde_json::to_string(&NowPlayingResponse::default()).unwrap()
        }
    } else {
        serde_json::to_string(&NowPlayingResponse::default()).unwrap()
    }
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

pub struct CORS;

#[rocket::async_trait]
impl Fairing for CORS {
    fn info(&self) -> Info {
        Info {
            name: "Add CORS headers to responses",
            kind: Kind::Response,
        }
    }

    async fn on_response<'r>(&self, _request: &'r Request<'_>, response: &mut Response<'r>) {
        response.set_header(Header::new("Access-Control-Allow-Origin", "*"));
        response.set_header(Header::new(
            "Access-Control-Allow-Methods",
            "POST, GET, PATCH, OPTIONS",
        ));
        response.set_header(Header::new("Access-Control-Allow-Headers", "*"));
        response.set_header(Header::new("Access-Control-Allow-Credentials", "true"));
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build().attach(CORS).mount(
        "/",
        routes![
            index,
            health,
            current_playlist,
            playlist,
            song,
            playlist_songs,
            now_playing
        ],
    )
}
