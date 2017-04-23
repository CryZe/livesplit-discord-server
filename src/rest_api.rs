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

#[get("/botw/bingo/<board>/<file..>?<params>", rank = 3)]
fn botw_bingo_params(board: &str, file: PathBuf, params: &str) -> Option<NamedFile> {
    drop(params);
    drop(board);
    NamedFile::open(Path::new("static/botw-bingo").join(file)).ok()
}

#[get("/botw/bingo/<board>/<file..>", rank = 2)]
fn botw_bingo(board: &str, file: PathBuf) -> Option<NamedFile> {
    drop(board);
    NamedFile::open(Path::new("static/botw-bingo").join(file)).ok()
}

#[get("/botw/bingo/<board>/tables/board.js", rank = 1)]
fn botw_bingo_board(board: &str) -> String {
    let board = match board {
        "normal" => include_str!("../bingo-templates/botw.json"),
        "korok" => include_str!("../bingo-templates/botw-korok.json"),
        "shrine" => include_str!("../bingo-templates/botw-shrine.json"),
        "plateau" => include_str!("../bingo-templates/botw-plateau.json"),
        _ => return "Nope".to_string(),
    };
    format!(r#"var bingoList = {}; $(function () {{ srl.bingo(bingoList, 5); }});"#,
            board)
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
                   routes![split,
                           reset,
                           get_state,
                           botw_bingo,
                           botw_bingo_params,
                           botw_bingo_board])
            .manage(state)
            .launch();
    });
}
