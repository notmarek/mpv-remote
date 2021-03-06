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
    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_string(value.to_string())
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

#[derive(Deserialize)]
pub struct PlayFromUrl {
    url: String,
    state: PlayerState,
}

async fn play_from_url(data: web::Json<PlayFromUrl>) -> impl Responder {
    let mpv = get_mpv();
    let data = data.into_inner();
    mpv.playlist_clear().unwrap();
    mpv.playlist_add(
        &data.url,
        PlaylistAddTypeOptions::File,
        PlaylistAddOptions::Append,
    )
    .unwrap();
    
    match mpv.playlist_play_id(0_usize) {
        Ok(_) => {
            match data.state {
                PlayerState::Paused => mpv.pause().unwrap(),
                _ => {}
            };
            HttpResponse::Ok().json(Response {
                status: Status::Success,
                data: "Playback started.",
            })
        }
        Err(e) => HttpResponse::Ok().json(Response {
            status: Status::Success,
            data: PlayerState::Error(e.to_string()),
        }),
    }
}

async fn index() -> impl Responder {

    let content = " 
    <!DOCTYPE html>
    <html lang='en'>
    
    <head>
        <title>MPV Controller</title>
    </head>
    
    <body>
        <form onsubmit='play_url(event)'>
            <input type='text' id='url' placeholder='File url' value='' /><br>
            Start paused: <input type='checkbox' name='Start paused' id='paused'><br>
            <input type='submit' value='Play'>
        </form>
        <button onclick='unpause()'>Unpause</button>
        <button onclick='pause()'>Pause</button>
    
        <script>
            function unpause() {
                fetch('/unpause');
            }
            function pause() {
                fetch('/pause');
            }
            function play_url(e) {
                e.preventDefault();
                let url = document.getElementById('url').value;
                let paused = document.getElementById('paused').checked ? 'paused' : 'playing';
    
                fetch('/play', { method: 'POST', body: JSON.stringify({ url: url, state: paused }), headers: { 'Content-Type': 'application/json' } });
    
            }
        </script>
    </body>
    
    </html>
    ";
HttpResponse::Ok().body(content)
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
        Ok(_) => {
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
        Ok(_) => {
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
            .route("/", web::get().to(index))
            .route("/pause", web::get().to(pause))
            .route("/unpause", web::get().to(unpause))
            .route("/play", web::post().to(play_from_url))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}
