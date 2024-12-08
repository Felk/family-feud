use std::collections::HashMap;
use std::ops::DerefMut;

use async_std;
use async_std::channel::Sender;
use async_std::net::TcpStream;
use async_std::path::Path;
use async_std::task::JoinHandle;
use async_std::{net, task};
use async_tungstenite;
use async_tungstenite::tungstenite::Message;
use async_tungstenite::WebSocketStream;
use futures::{SinkExt, StreamExt};
use futures_signals::signal::{Mutable, MutableLockMut, SignalExt};
use log::{debug, error, info, LevelFilter};
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::{Error, Value};
use tide::http::Mime;
use tide::{Body, StatusCode};

use crate::state::{Answer, State};

mod state;

#[derive(RustEmbed)]
#[folder = "../svelte-frontend/dist/"]
struct Asset;

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", content = "data")]
enum Payload {
    #[serde(rename = "update_property")]
    UpdateProperty { name: String, data: Value },
}

fn observe<T>(
    key: &'static str,
    mutable: &Mutable<T>,
    sender: Sender<String>,
) -> JoinHandle<()>
    where T: Serialize + Send + Sync + 'static
{
    task::spawn(mutable
        .signal_ref(move |val| {
            Payload::UpdateProperty {
                name: key.to_string(),
                data: serde_json::to_value(&val).unwrap(),
            }
        })
        .for_each(move |payload| {
            let sender_cloned = sender.clone();
            async move {
                sender_cloned.send(serde_json::to_string(&payload).unwrap()).await.expect("sender should never be closed manually");
            }
        }))
}

trait SetFromJsonValue: Send + Sync {
    fn deser_and_set(&self, v: Value) -> Result<(), Error>;
}

impl<T> SetFromJsonValue for Mutable<T>
    where T: for<'de> Deserialize<'de> + Send + Sync
{
    fn deser_and_set(&self, v: Value) -> Result<(), Error> {
        self.set(serde_json::from_value::<T>(v)?);
        Ok(())
    }
}

async fn handle_client(stream: WebSocketStream<TcpStream>, state: State) {
    let (mut ws_write, mut ws_read) = stream.split();

    // use an async channel as a mux from n possible state change sources to 1 websocket send loop
    let (sender, receiver) = async_std::channel::unbounded();
    
    let mut observe_job_handles = Vec::new();
    let mut setters: HashMap<&str, &dyn SetFromJsonValue> = HashMap::new();
    // There's probably a good way to hook up all the `Mutable`s to the websocket without this
    // boilerplate, using macros or something, maybe. But this does the job for now. The drawback
    // is that if new `Mutable`s get added, one must not forget to extend this boilerplate!
    observe_job_handles.push(observe("title", &state.title, sender.clone()));
    setters.insert("title", &state.title);
    observe_job_handles.push(observe("answers", &state.answers, sender.clone()));
    setters.insert("answers", &state.answers);

    let send_job_handle = task::spawn(async move {
        while let Ok(text) = receiver.recv().await {
            ws_write.send(Message::Text(text)).await.expect("websocket connection died unexpectedly");
        }
    });

    // recv loop
    while let Some(Ok(msg)) = ws_read.next().await {
        if !msg.is_text() { continue; }
        if let Ok(text) = msg.to_text() {
            debug!("message arrived: {}", &text);
            match serde_json::from_str::<Payload>(&text) {
                Ok(payload) => match payload {
                    Payload::UpdateProperty { name, data } => {
                        debug!("received update for prop {}: {}", name, data);
                        if let Some(setter) = setters.get(name.as_str()) {
                            if let Err(err) = setter.deser_and_set(data) {
                                error!("client sent invalid data: {}", err)
                            }
                        } else {
                            error!("No setter found for name '{}'", name);
                        }
                    }
                },
                Err(error) => error!("event could not be read: {}", error)
            }
        }
    }

    for h in observe_job_handles {
        h.cancel().await;
    }
    send_job_handle.cancel().await;

    debug!("WebSocket connection dead");
}

fn get_answer_by_index(index: usize, answers: &mut Vec<Answer>) -> Result<&mut Answer, tide::Error> {
    if index < 1 {
        let error = format!("invalid index '{}', must be at least 1", index);
        return Err(tide::Error::from_str(StatusCode::BadRequest, error));
    }
    let len = answers.len();
    if let Some(answer) = answers.get_mut(index - 1) {
        Ok(answer)
    } else {
        let error = format!("invalid index '{}', must be at most {}", index, len);
        Err(tide::Error::from_str(StatusCode::NotFound, error))
    }
}

#[async_std::main]
async fn main() {
    env_logger::builder()
        .filter_module("tide::log::middleware", LevelFilter::Warn)
        .filter_module("tide::listener::tcp_listener", LevelFilter::Off)
        .filter_level(LevelFilter::Info)
        .init();
    let listener = net::TcpListener::bind("127.0.0.1:8030").await.unwrap();
    info!("Listening on: {}", listener.local_addr().unwrap());

    // TODO fix this: just use State::new() instead of this static lifetime hack.
    //      tide handler methods either need to move, or to borrow for 'static,
    //      need to figure out how to tell rust that tide's lifetime does not exceed main.
    let state: &'static State = Box::leak(Box::new(State::new()));

    let mut app = tide::new();
    app.at("/").get(|_req: tide::Request<()>| async move {
        let response: tide::Response = tide::Redirect::new("/index.html").into();
        Ok(response)
    });
    app.at("/toggle/:index").get(move |req: tide::Request<()>| async move {
        let state_clone: State = (*state).clone();
        let index: usize = req.param("index")?.parse()?;
        let mut answers: MutableLockMut<Vec<Answer>> = state_clone.answers.lock_mut();
        let answer = get_answer_by_index(index, answers.deref_mut())?;
        answer.shown = !answer.shown;
        Ok(tide::Response::new(StatusCode::Ok))
    });
    app.at("/toggle/all").get(move |_req: tide::Request<()>| async move {
        let state_clone: State = (*state).clone();
        let mut answers: MutableLockMut<Vec<Answer>> = state_clone.answers.lock_mut();
        for answer in answers.iter_mut() {
            answer.shown = !answer.shown;
        }
        Ok(tide::Response::new(StatusCode::Ok))
    });
    app.at("/show/:index").get(move |req: tide::Request<()>| async move {
        let state_clone: State = (*state).clone();
        let index: usize = req.param("index")?.parse()?;
        let mut answers: MutableLockMut<Vec<Answer>> = state_clone.answers.lock_mut();
        let answer = get_answer_by_index(index, answers.deref_mut())?;
        answer.shown = true;
        Ok(tide::Response::new(StatusCode::Ok))
    });
    app.at("/show/all").get(move |_req: tide::Request<()>| async move {
        let state_clone: State = (*state).clone();
        let mut answers: MutableLockMut<Vec<Answer>> = state_clone.answers.lock_mut();
        for answer in answers.iter_mut() {
            answer.shown = true;
        }
        Ok(tide::Response::new(StatusCode::Ok))
    });
    app.at("/hide/:index").get(move |req: tide::Request<()>| async move {
        let state_clone: State = (*state).clone();
        let index: usize = req.param("index")?.parse()?;
        let mut answers: MutableLockMut<Vec<Answer>> = state_clone.answers.lock_mut();
        let answer = get_answer_by_index(index, answers.deref_mut())?;
        answer.shown = false;
        Ok(tide::Response::new(StatusCode::Ok))
    });
    app.at("/hide/all").get(move |_req: tide::Request<()>| async move {
        let state_clone: State = (*state).clone();
        let mut answers: MutableLockMut<Vec<Answer>> = state_clone.answers.lock_mut();
        for answer in answers.iter_mut() {
            answer.shown = false;
        }
        Ok(tide::Response::new(StatusCode::Ok))
    });
    app.at("/*").get(|req: tide::Request<()>| async move {
        let path = Path::new(req.url().path().trim_start_matches("/"));
        if let Some(asset) = Asset::get(path.to_str().unwrap()) {
            let data: Vec<u8> = asset.data.into_owned();
            let mut body: Body = tide::Body::from(data);
            if let Some(extension) = path.extension() {
                if let Some(mime) = Mime::from_extension(extension.to_str().unwrap()) {
                    body.set_mime(mime);
                }
            }
            Ok(body)
        } else {
            Err(tide::Error::from_str(StatusCode::NotFound, "").into())
            // let x: Response = tide::Error::from_str(StatusCode::NotFound, "").into();
            // Ok(x)
        }
    });
    const PORT: u16 = 8031;
    let http_server_future = app.listen(format!("127.0.0.1:{}", PORT));

    let http_server_handle = task::spawn(http_server_future);
    if let Err(e) = open::that(format!("http://localhost:{}", PORT)) {
        error!("Failed to open URL in webbrowser: {}", e);
    }

    println!("==================================================");
    println!("===                                            ===");
    println!("===            Started Family Feud!            ===");
    println!("===         Visit http://localhost:{}!       ===", PORT);
    println!("===              Exit with CTRL-C              ===");
    println!("===                                            ===");
    println!("==================================================");

    println!();
    println!("Drag&Drop text files with this format into the browser tab:");
    println!("  Is this the question title?");
    println!();
    println!("  This is the first answer (70)");
    println!("  This is the second answer (20)");
    println!("  This is the third answer (10)");

    println!();
    println!("The following HTTP endpoints are available:");
    println!("  http://localhost:{}/toggle/<index>", PORT);
    println!("  http://localhost:{}/toggle/all", PORT);
    println!("  http://localhost:{}/show/<index>", PORT);
    println!("  http://localhost:{}/show/all", PORT);
    println!("  http://localhost:{}/hide/<index>", PORT);
    println!("  http://localhost:{}/hide/all", PORT);

    while let Ok((stream, peer_addr)) = listener.accept().await {
        let cloned_state = state.clone();
        if let Ok(ws_stream) = async_tungstenite::accept_async(stream).await {
            debug!("New WebSocket connection: {}", peer_addr);
            task::spawn(handle_client(ws_stream, cloned_state));
        }
    }

    http_server_handle.await.unwrap();
}
