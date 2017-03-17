use rocket::request::State;
use rocket::config::{Config, Environment};
use rocket_contrib::JSON;
use std::sync::Arc;
use {LSState, Layout, rocket};
use std::thread::spawn;
use std::env;

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

pub fn start(state: Arc<LSState>) {
    spawn(|| {
        let mut config = Config::build(Environment::active().unwrap());
        if let Ok(Ok(port)) = env::var("PORT").map(|p| p.parse()) {
            config = config.port(port);
        }
        let config = config.finalize().unwrap();
        rocket::custom(config, true)
            .mount("/", routes![split, reset, get_state])
            .manage(state)
            .launch();
    });
}
