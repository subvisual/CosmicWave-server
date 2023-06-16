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
    songs: Vec<Song>,
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

    println!("{:?}", response);

    match response {
        Some(value) => {
            let data = &value["data"][0]["data"];
            println!(
                "{:?}",
                serde_json::from_value::<ActivePlaylist>(data.clone()).unwrap()
            );
        }
        _ => println!("nothing"),
    };
}

#[get("/song/<id>")]
async fn song(id: String) {
    let response = fetch_song(id).await;

    match response {
        Some(song) => {
            let data = &song["data"];
            println!(
                "{:?}",
                serde_json::from_value::<Song>(data.clone()).unwrap()
            );
        }
        _ => println!("nothing"),
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build().mount("/", routes![index, current_playlist, song])
}

async fn fetch_playlist() -> Option<Value> {
    let client = reqwest::Client::new();

    let url = format!("{}/{}", BASE_URL, COLLECTION_PATH);

    let response = client
        .get(format!("{}%2FActivePlaylist/records", url))
        .send()
        .await
        .unwrap();

    match response.status() {
        StatusCode::OK => Some(serde_json::from_str(&response.text().await.unwrap()).unwrap()),
        _ => None,
    }
}

async fn fetch_song(id: String) -> Option<Value> {
    let client = reqwest::Client::new();

    let url = format!("{}/{}", BASE_URL, COLLECTION_PATH);

    let response = client
        .get(format!("{}%2FSong/records/{}", url, id))
        .send()
        .await
        .unwrap();

    match response.status() {
        StatusCode::OK => {
            let v = &response.text().await.unwrap();
            println!("{}", v);
            Some(serde_json::from_str(v).unwrap())
        }
        _ => None,
    }
}
