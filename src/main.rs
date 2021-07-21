use actix_web::{web, App, HttpResponse, HttpServer, Responder};
use mpvipc::*;
use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use std::{thread, time};

pub enum PlayerState {
    Playing,
    Paused,
    Error(String),
}
struct PlayerStateVisitor;

impl<'de> Visitor<'de> for PlayerStateVisitor {
    type Value = PlayerState;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("string \"playing\" or \"paused\"")
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        if value == "playing" {
            Ok(PlayerState::Playing)
        } else if value == "paused" {
            Ok(PlayerState::Paused)
        } else if value == "error" {
            Ok(PlayerState::Error("error".to_string()))
        } else {
            Err(de::Error::custom("Unexpected string"))
        }
    }
}

impl<'de> Deserialize<'de> for PlayerState {
    fn deserialize<D>(deserializer: D) -> Result<PlayerState, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_string(PlayerStateVisitor)
    }
}
impl Serialize for PlayerState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            PlayerState::Playing => "playing",
            PlayerState::Paused => "paused",
            PlayerState::Error(e) => e,
        }
        .serialize(serializer)
    }
}
pub enum Status {
    Success,
    Error,
}

impl Serialize for Status {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Status::Success => "success",
            Status::Error => "error",
        }
        .serialize(serializer)
    }
}

#[derive(Serialize)]
pub struct Response<T: Serialize> {
    status: Status,
    data: T,
}

async fn play_test() -> impl Responder {
    let mpv = get_mpv();
    mpv.playlist_clear().unwrap();
    mpv.playlist_add("http://rem.lan/1qweww45//home/pi/drives/X/Animu/Air%20(2005)%20%5BDoki%5D%5B1280x720%20Hi10P%20BD%20FLAC%5D/%5BDoki%5D%20Air%20-%2002%20(1280x720%20Hi10P%20BD%20FLAC)%20%5BFEECD6B6%5D.mkv", PlaylistAddTypeOptions::File, PlaylistAddOptions::Append).unwrap();
    mpv.playlist_play_id(0_usize).unwrap();
    "Video playback should've started."
}

async fn pause() -> impl Responder {
    let mpv = get_mpv();
    let play_state = match mpv.get_property::<bool>("pause") {
        Ok(s) => match s {
            true => Err(PlayerState::Paused),
            false => Ok(PlayerState::Playing),
        },
        Err(e) => Err(PlayerState::Error(format!("{}", e))),
    };
    match play_state {
        Ok(s) => {
            mpv.pause().unwrap();
            HttpResponse::Ok().json(Response {
                status: Status::Success,
                data: "Paused playback.",
            })
        }
        Err(e) => HttpResponse::Ok().json(Response {
            status: Status::Error,
            data: e,
        }),
    }
}

async fn unpause() -> impl Responder {
    let mpv = get_mpv();
    let play_state = match mpv.get_property::<bool>("pause") {
        Ok(s) => match s {
            true => Ok(PlayerState::Paused),
            false => Err(PlayerState::Playing),
        },
        Err(e) => Err(PlayerState::Error(format!("{}", e))),
    };
    match play_state {
        Ok(s) => {
            mpv.set_property("pause", false).unwrap();
            HttpResponse::Ok().json(Response {
                status: Status::Success,
                data: "Playback resumed.",
            })
        }
        Err(e) => HttpResponse::Ok().json(Response {
            status: Status::Error,
            data: e,
        }),
    }
}
fn spawn_mpv() {
    match std::process::Command::new("mpv")
        .args(&[
            "--input-ipc-server=/tmp/mpvsocket",
            "--idle",
            // "--no-terminal",
        ])
        .spawn()
    {
        Ok(_) => (),
        Err(err) => panic!("Error while trying to spawn mpv: {}", err),
    };
}

fn get_mpv() -> Mpv {
    thread::sleep(time::Duration::from_millis(100));
    match Mpv::connect("/tmp/mpvsocket") {
        Ok(mpv) => mpv,
        Err(_) => {
            spawn_mpv();
            get_mpv()
        }
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(move || {
        App::new()
            .route("/", web::get().to(play_test))
            .route("/pause", web::get().to(pause))
            .route("/unpause", web::get().to(unpause))
    })
    .bind(("127.0.0.1", 8080))?
    .run()
    .await
}
