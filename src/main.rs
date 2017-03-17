#![feature(plugin)]
#![plugin(rocket_codegen)]

extern crate rocket;
extern crate rocket_contrib;
extern crate livesplit_core;
extern crate parking_lot;
#[macro_use]
extern crate serde_derive;
extern crate serenity;
extern crate hyper;
extern crate hyper_rustls;
extern crate chashmap;
#[macro_use]
extern crate log;

use chashmap::{CHashMap, WriteGuard};
use livesplit_core::{Timer, Run, Segment};
use livesplit_core::component::{title, splits, timer, previous_segment, sum_of_best,
                                possible_time_save};
use std::sync::Arc;
use parking_lot::RwLock;

mod discord;
mod rest_api;

#[derive(Serialize)]
pub struct Layout {
    title: title::State,
    splits: splits::State,
    timer: timer::State,
    previous_segment: previous_segment::State,
    sum_of_best: sum_of_best::State,
    possible_time_save: possible_time_save::State,
}

pub struct LSState {
    users: CHashMap<u64, User>,
    race: RwLock<Race>,
}

pub enum Race {
    NoRace,
    Setup(Vec<(u64, bool)>),
    InProgress(Vec<u64>),
}

pub struct User {
    name: String,
    components: Components,
    timer: Timer,
}

pub struct Components {
    title: title::Component,
    splits: splits::Component,
    timer: timer::Component,
    previous_segment: previous_segment::Component,
    sum_of_best: sum_of_best::Component,
    possible_time_save: possible_time_save::Component,
}

impl LSState {
    fn user<S>(&self, id: u64, name: S) -> WriteGuard<u64, User>
        where S: AsRef<str>
    {
        loop {
            if let Some(user) = self.users.get_mut(&id) {
                return user;
            }
            info!("New User {}", name.as_ref());
            let mut run = Run::new(vec![Segment::new("First"),
                                        Segment::new("Second"),
                                        Segment::new("Third"),
                                        Segment::new("End")]);
            run.set_game_name("Wind Waker");
            run.set_category_name("Any%");
            let timer = Timer::new(run);
            self.users.insert(id,
                              User {
                                  name: name.as_ref().to_owned(),
                                  components: Components {
                                      title: title::Component::new(),
                                      splits: splits::Component::new(),
                                      timer: timer::Component::new(),
                                      previous_segment: previous_segment::Component::new(),
                                      sum_of_best: sum_of_best::Component::new(),
                                      possible_time_save: possible_time_save::Component::new(),
                                  },
                                  timer: timer,
                              });
        }
    }
}

impl User {
    fn eval_layout(&mut self) -> Layout {
        Layout {
            title: self.components.title.state(&mut self.timer),
            splits: self.components.splits.state(&mut self.timer),
            timer: self.components.timer.state(&mut self.timer),
            previous_segment: self.components.previous_segment.state(&mut self.timer),
            sum_of_best: self.components.sum_of_best.state(&mut self.timer),
            possible_time_save: self.components.possible_time_save.state(&mut self.timer),
        }
    }
}

fn main() {
    let state = Arc::new(LSState {
        users: CHashMap::new(),
        race: RwLock::new(Race::NoRace),
    });

    rest_api::start(state.clone());
    discord::start(state);
}
