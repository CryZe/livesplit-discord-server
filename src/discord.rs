use std::borrow::Cow;
use std::fmt::Write;
use std::io::{Read, Cursor};
use std::sync::Arc;
use {LSState, User, Race};
use chashmap::WriteGuard;
use hyper_rustls::TlsClient;
use hyper::Client as HyperClient;
use hyper::net::HttpsConnector;
use livesplit_core::{Color, Timer};
use livesplit_core::parser::composite;
use serenity::Client;
use serenity::client::Context;
use serenity::model::Message;
use serenity::utils::builder::CreateEmbed;
use serenity::utils::Colour;
use std::{thread, time};
use dotenv::var;
use speedrun_bingo::{Mode, Template};
use rand::{Rng, thread_rng};

fn send_embed_message<F>(message: &Message, create: F) -> Result<(), String>
    where F: FnOnce(CreateEmbed) -> CreateEmbed
{
    message.channel_id
        .send_message(|m| m.embed(create))
        .map_err(|_| String::from("Couldn't send message"))?;
    Ok(())
}

fn send_editable_text_message(message: &Message, text: &str) -> Result<Message, String> {
    message.channel_id
        .send_message(|m| m.content(text))
        .map_err(|_| String::from("Couldn't send message"))
}

fn send_text_message(message: &Message, text: &str) -> Result<(), String> {
    send_editable_text_message(message, text)?;
    Ok(())
}

fn layout(user: &mut User, embed: CreateEmbed) -> CreateEmbed {
    let layout = user.eval_layout();
    let (r, g, b) = match layout.timer.color {
        Color::AheadGainingTime => (0x00, 0xcc, 0x4b),
        Color::AheadLosingTime => (0x5c, 0xd6, 0x89),
        Color::BehindGainingTime => (0xd6, 0x5c, 0x5c),
        Color::BehindLosingTime => (0xcc, 0x00, 0x00),
        Color::BestSegment => (0xff, 0xd4, 0x00),
        Color::NotRunning | Color::Default => (0x99, 0x99, 0x99),
        Color::Paused => (0x66, 0x66, 0x66),
        Color::PersonalBest => (0x4d, 0xa6, 0xff),
    };
    let mut embed = embed.author(|a| {
            a.name("LiveSplit")
                .icon_url("https://raw.githubusercontent.\
                           com/LiveSplit/LiveSplit/master/LiveSplit/Resources/Icon.png")
                .url("http://livesplit.org")
        })
        .colour(Colour::from_rgb(r, g, b))
        .description(&format!(r"
**{}{}**

**{}:**   {}
**{}:**   {}
**{}:**   {}
**Attempts:**   {}",
                              layout.timer.time,
                              layout.timer.fraction,
                              layout.previous_segment.text,
                              layout.previous_segment.time,
                              layout.sum_of_best.text,
                              layout.sum_of_best.time,
                              layout.possible_time_save.text,
                              layout.possible_time_save.time,
                              layout.title.attempts))
        .title(&format!("{} - {}", layout.title.game, layout.title.category));

    for segment in &layout.splits.splits {
        embed =
            embed.field(|f| {
                f.name(&segment.name).value(&format!("{}  {}", segment.delta, segment.time))
            });
    }

    embed
}

fn user<'a>(state: &'a LSState, message: &Message) -> WriteGuard<'a, u64, User> {
    state.user(message.author.id.0, message.author.name.as_str())
}

fn split(_: &mut Context,
         message: &Message,
         _: Vec<String>,
         state: &LSState)
         -> Result<(), String> {
    let mut user = user(state, message);
    user.timer.split();
    user.timer.start();
    send_embed_message(message, |m| layout(&mut user, m))
}

fn reset(_: &mut Context,
         message: &Message,
         _: Vec<String>,
         state: &LSState)
         -> Result<(), String> {
    let mut user = user(state, message);
    user.timer.reset(true);
    send_embed_message(message, |m| layout(&mut user, m))
}

fn get_state(_: &mut Context,
             message: &Message,
             _: Vec<String>,
             state: &LSState)
             -> Result<(), String> {
    let mut user = user(state, message);
    send_embed_message(message, |m| layout(&mut user, m))
}


fn load_race_splits(_: &mut Context,
                    message: &Message,
                    _: Vec<String>,
                    state: &LSState)
                    -> Result<(), String> {
    let mut user = user(state, message);
    let race = state.race.read();
    match *race {
        Race::NoRace => send_text_message(message, "There is no race!"),
        Race::InProgress(_) => send_text_message(message, "The Race is already in Progress!"),
        Race::Setup(ref entrants) => {
            let id = entrants[0].0;
            let master = state.users
                .get(&id)
                .ok_or_else(|| String::from("User not found"))?;
            user.timer = master.timer.clone();
            send_embed_message(message, |m| layout(&mut user, m))
        }
    }
}

fn load_splits(_: &mut Context,
               message: &Message,
               params: Vec<String>,
               state: &LSState)
               -> Result<(), String> {
    let mut user = user(state, message);
    if let Some(param) = params.get(0) {
        let ssl = TlsClient::new();
        let connector = HttpsConnector::new(ssl);
        let client = HyperClient::with_connector(connector);
        if let Ok(mut response) =
            client.get(&format!("https://splits.io/{}/download/livesplit", param))
                .send() {
            let mut splits = Vec::new();
            if let Ok(_) = response.read_to_end(&mut splits) {
                if let Ok(run) = composite::parse(Cursor::new(splits), None, false) {
                    user.timer = Timer::new(run);
                }
            }
        }
    }
    send_embed_message(message, |m| layout(&mut user, m))
}

fn create_race(_: &mut Context,
               message: &Message,
               _: Vec<String>,
               state: &LSState)
               -> Result<(), String> {
    let mut race = state.race.write();
    if let Race::NoRace = *race {
        user(state, message); // Make sure the user exists
        *race = Race::Setup(vec![(message.author.id.0, false)]);
        send_text_message(message, "Created a new race!")
    } else {
        send_text_message(message, "There already is an active race!")
    }
}

fn create_bingo(_: &mut Context,
                message: &Message,
                params: Vec<String>,
                _: &LSState)
                -> Result<(), String> {
    let template = include_str!("../bingo-templates/botw.json");
    let template = Template::from_json_str(template).unwrap();

    let (mode, mode_txt) = match params.get(0).map(String::as_ref) {
        Some("short") => (Mode::Short, "short"),
        Some("long") => (Mode::Long, "long"),
        _ => (Mode::Normal, "normal"),
    };

    let mut rng = thread_rng();
    let seed = rng.gen_range(0, 1_000_000);

    let board = template.generate(seed, mode);

    let mut board_text = format!("https://livesplit.herokuapp.com/botw/bingo/index.\
                                  html?seed={}&mode={}\n\n",
                                 seed,
                                 mode_txt);

    for row in &board.cells {
        for (i, goal) in row.iter().enumerate() {
            if i != 0 {
                write!(board_text, " | ").unwrap();
            }
            write!(board_text, "{}", goal).unwrap();
        }
        writeln!(board_text).unwrap();
    }
    send_text_message(message, &board_text)
}

fn entrants(_: &mut Context,
            message: &Message,
            _: Vec<String>,
            state: &LSState)
            -> Result<(), String> {
    let race = state.race.read();
    let text = match *race {
        Race::NoRace => Cow::from("There is no race!"),
        Race::Setup(ref entrants) => {
            let mut message = String::new();
            for &(entrant, status) in entrants {
                if !message.is_empty() {
                    message.push_str(", ");
                } else {
                    message.push_str("Entrants: ");
                }
                write!(message,
                       "{} ({})",
                       state.users
                           .get(&entrant)
                           .ok_or_else(|| String::from("User not found"))?
                           .name,
                       if status { "Ready" } else { "Not Ready" })
                    .unwrap();
            }
            message.into()
        }
        Race::InProgress(ref entrants) => {
            let mut message = String::new();
            for &entrant in entrants {
                if !message.is_empty() {
                    message.push_str(", ");
                } else {
                    message.push_str("Entrants: ");
                }
                write!(message,
                       "{}",
                       state.users
                           .get(&entrant)
                           .ok_or_else(|| String::from("User not found"))?
                           .name)
                    .unwrap();
            }
            message.into()
        }
    };
    send_text_message(message, &text)
}

fn enter(_: &mut Context,
         message: &Message,
         _: Vec<String>,
         state: &LSState)
         -> Result<(), String> {
    let mut race = state.race.write();
    let text = match *race {
        Race::NoRace => "There is no race!",
        Race::Setup(ref mut entrants) => {
            if entrants.iter().any(|&(id, _)| id == message.author.id.0) {
                "You already entered the race!"
            } else {
                user(state, message); // Make sure the user exists
                entrants.push((message.author.id.0, false));
                "You successfully entered the race!"
            }
        }
        Race::InProgress(_) => "The Race is already in Progress!",
    };
    send_text_message(message, text)
}

fn ready(_: &mut Context,
         message: &Message,
         _: Vec<String>,
         state: &LSState)
         -> Result<(), String> {
    let mut race = state.race.write();
    let new_state = match *race {
        Race::NoRace => return send_text_message(message, "There is no race!"),
        Race::InProgress(_) => {
            return send_text_message(message, "The Race is already in Progress!")
        }
        Race::Setup(ref mut entrants) => {
            let response = if let Some(&mut (_, ref mut status)) =
                entrants.iter_mut().filter(|&&mut (id, _)| id == message.author.id.0).next() {
                if *status {
                    "You are already ready!"
                } else {
                    *status = true;
                    "You are now ready for the race!"
                }
            } else {
                "You didn't enter the race!"
            };
            send_text_message(message, response)?;

            let all_ready = entrants.iter().all(|&(_, status)| status);
            if all_ready {
                let mut message = send_editable_text_message(message, "All entrants are ready!")?;

                let entrants = entrants.iter().map(|&(id, _)| id).collect();

                thread::sleep(time::Duration::from_secs(3));
                for remaining_seconds in (1..11).rev() {
                    message.edit(&format!("**{}**", remaining_seconds), |x| x)
                        .map_err(|_| String::from("Couldn't edit message"))?;
                    thread::sleep(time::Duration::from_secs(1));
                }

                message.edit("**Go!**", |x| x).map_err(|_| String::from("Couldn't edit message"))?;

                for entrant in &entrants {
                    let mut user = state.users
                        .get_mut(entrant)
                        .ok_or_else(|| String::from("User not found"))?;
                    user.timer.split();
                    user.timer.start();
                }

                Race::InProgress(entrants)
            } else {
                return Ok(());
            }
        }
    };
    *race = new_state;
    Ok(())
}

pub fn start(state: Arc<LSState>) {
    let token = var("DISCORD_TOKEN").expect("Expected DISCORD_TOKEN Environment Variable");
    let mut client = Client::login_bot(&token);

    client.with_framework(move |f| {
        let split_state = state.clone();
        let reset_state = state.clone();
        let load_splits_state = state.clone();
        let load_race_splits_state = state.clone();
        let create_race_state = state.clone();
        let entrants_state = state.clone();
        let enter_state = state.clone();
        let ready_state = state.clone();
        let create_bingo_state = state.clone();

        f.configure(|c| c.prefix("!"))
            .on("split", move |c, m, v| split(c, m, v, &split_state))
            .on("reset", move |c, m, v| reset(c, m, v, &reset_state))
            .on("load-splits",
                move |c, m, v| load_splits(c, m, v, &load_splits_state))
            .on("load-race-splits",
                move |c, m, v| load_race_splits(c, m, v, &load_race_splits_state))
            .on("create-race",
                move |c, m, v| create_race(c, m, v, &create_race_state))
            .on("entrants",
                move |c, m, v| entrants(c, m, v, &entrants_state))
            .on("enter", move |c, m, v| enter(c, m, v, &enter_state))
            .on("ready", move |c, m, v| ready(c, m, v, &ready_state))
            .on("timer", move |c, m, v| get_state(c, m, v, &state))
            .on("bingo",
                move |c, m, v| create_bingo(c, m, v, &create_bingo_state))
    });

    client.start().unwrap();
}
