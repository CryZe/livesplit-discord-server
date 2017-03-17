use rocket::request::State;
use rocket_contrib::JSON;
use std::sync::Arc;
use {LSState, Layout, rocket};
use std::thread::spawn;

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
    spawn(|| rocket::ignite().mount("/", routes![split, reset, get_state]).manage(state).launch());
}
