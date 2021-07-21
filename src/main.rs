use actix_web::{web, App, HttpServer, Responder};
use mpvipc::*;
use std::{thread, time};

async fn play_test() -> impl Responder {
    let mpv = get_mpv();
    mpv.playlist_clear().unwrap();
    mpv.playlist_add("http://rem.lan/1qweww45//home/pi/drives/X/Animu/Air%20(2005)%20%5BDoki%5D%5B1280x720%20Hi10P%20BD%20FLAC%5D/%5BDoki%5D%20Air%20-%2002%20(1280x720%20Hi10P%20BD%20FLAC)%20%5BFEECD6B6%5D.mkv", PlaylistAddTypeOptions::File, PlaylistAddOptions::Append).unwrap();
    mpv.playlist_play_id(0_usize).unwrap();
    "Video playback should've started."
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
    HttpServer::new(move || App::new().route("/", web::get().to(play_test)))
        .bind(("127.0.0.1", 8080))?
        .run()
        .await
}
