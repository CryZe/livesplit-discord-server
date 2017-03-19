use rocket::request::State;
use rocket::config::{Config, Environment};
use rocket_contrib::JSON;
use std::sync::Arc;
use {LSState, Layout, rocket};
use std::thread::spawn;
use dotenv::var;
use std::path::{Path, PathBuf};
use rocket::response::NamedFile;

#[get("/split")]
fn split(state: State<Arc<LSState>>) -> JSON<Layout> {
    let mut user = state.user(0, "REST User");
    user.timer.split();
    user.timer.start();
    JSON(user.eval_layout())
}

#[get("/reset")]
fn reset(state: State<Arc<LSState>>) -> JSON<Layout> {
    let mut user = state.user(0, "REST User");
    user.timer.reset(true);
    JSON(user.eval_layout())
}

#[get("/state")]
fn get_state(state: State<Arc<LSState>>) -> JSON<Layout> {
    let mut user = state.user(0, "REST User");
    JSON(user.eval_layout())
}

#[get("/botw/bingo/<file..>?<params>")]
fn botw_bingo_params(file: PathBuf, params: &str) -> Option<NamedFile> {
    drop(params);
    NamedFile::open(Path::new("static/botw-bingo").join(file)).ok()
}

#[get("/botw/bingo/<file..>")]
fn botw_bingo(file: PathBuf) -> Option<NamedFile> {
    NamedFile::open(Path::new("static/botw-bingo").join(file)).ok()
}

pub fn start(state: Arc<LSState>) {
    spawn(|| {
        let mut config = Config::new(Environment::active().unwrap()).unwrap();

        config.set_address("0.0.0.0").unwrap();

        if let Ok(Ok(port)) = var("PORT").map(|p| p.parse()) {
            config.set_port(port);
        }

        rocket::custom(config, true)
            .mount("/",
                   routes![split, reset, get_state, botw_bingo, botw_bingo_params])
            .manage(state)
            .launch();
    });
}
