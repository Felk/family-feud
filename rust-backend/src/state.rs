use futures_signals::signal::Mutable;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Answer {
    id: u32,
    text: String,
    votes: u32,
    pub(crate) shown: bool,
}

// If you add or change something here, don't forget to also edit handle_client in main.rs
#[derive(Clone)]
pub struct State {
    pub title: Mutable<String>,
    pub answers: Mutable<Vec<Answer>>,
}

impl State {
    pub fn new() -> State {
        State {
            title: Mutable::new("Default Title".to_string()),
            answers: Mutable::new(vec![
                Answer {
                    id: 1,
                    text: "Popular Answer".to_string(),
                    votes: 80,
                    shown: false,
                },
                Answer {
                    id: 2,
                    text: "Unpopular Answer".to_string(),
                    votes: 20,
                    shown: true,
                },
            ]),
        }
    }
}
