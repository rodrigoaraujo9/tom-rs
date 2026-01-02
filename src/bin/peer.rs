use color_print::cprintln;
use rand::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::env::args;
use std::error::Error;
use std::net::SocketAddr;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Notify, RwLock};
use tokio::time::sleep;
use tonic::{transport::Server, Request, Response, Status};

pub mod message {
    tonic::include_proto!("message");
}

use message::message_service_client::MessageServiceClient;
use message::message_service_server::{MessageService as MessageServiceTrait, MessageServiceServer};
use message::{SendMessageRequest, SendMessageResponse};
use crate::message::LClock;

const MAX_TIMESTAMP_DRIFT: u64 = 100;

#[derive(Debug)]
pub struct State {
    index: u32,
    tick: Arc<RwLock<u64>>,
    words: Arc<Vec<String>>,
    messages: Arc<RwLock<VecDeque<SendMessageRequest>>>,
    n_peers: u32,
    peers: Arc<RwLock<HashMap<u32, MessageServiceClient<tonic::transport::Channel>>>>,
    latest: Arc<RwLock<HashMap<u32, u64>>>,
    notifier: Arc<Notify>,
}

pub struct MessageService {
    state: Arc<State>,
}

impl MessageService {
    async fn new(index: u32, peers: HashMap<u32, MessageServiceClient<tonic::transport::Channel>>,
        n_peers: u32, filename: &str) -> Result<Self, Box<dyn Error>> {
        Ok(Self {
            state: Arc::new(State {
                index,
                tick: Arc::new(RwLock::new(0)),
                words: Arc::new(load_words(filename).await?),
                messages: Arc::new(RwLock::new(VecDeque::new())),
                peers: Arc::new(RwLock::new(peers)),
                latest: Arc::new(RwLock::new(HashMap::new())),
                n_peers,
                notifier: Arc::new(Notify::new()),
            })
        })
    }
}

#[tonic::async_trait]
impl MessageServiceTrait for MessageService {
    async fn send_message(&self, request: Request<SendMessageRequest>) -> Result<Response<SendMessageResponse>, Status> {
        let msg = request.into_inner();
        let clock = msg.clock.as_ref().ok_or_else(|| Status::invalid_argument("missing clock"))?;

        {
            let latest = self.state.latest.read().await;
            if let Some(&last_ts) = latest.get(&clock.sender) {
                if clock.timestamp <= last_ts {
                    cprintln!("<yellow>*attack*</yellow> peer {} tried to reuse timestamp {} (latest: {})",
                             clock.sender, clock.timestamp, last_ts);
                    return Err(Status::invalid_argument("timestamp reuse detected"));
                }
            }
        }

        {
            let latest = self.state.latest.read().await;
            let max_observed = latest.values().max().copied().unwrap_or(0);

            if clock.timestamp > max_observed + MAX_TIMESTAMP_DRIFT {
                cprintln!("<yellow>*attack*</yellow> peer {} using future timestamp {} (max observed: {})",
                         clock.sender, clock.timestamp, max_observed);
                return Err(Status::invalid_argument("future timestamp detected"));
            }
        }

        {
            let mut t = self.state.tick.write().await;
            *t = (*t).max(clock.timestamp) + 1;
        }

        {
            let mut latest = self.state.latest.write().await;
            latest.insert(clock.sender, clock.timestamp);
        }

        {
            let mut mss = self.state.messages.write().await;
            mss.push_back(msg);
            mss.make_contiguous().sort_by_key(|m| {
                let c = m.clock.as_ref().unwrap();
                (c.timestamp, c.sender)
            });
        }

        self.state.notifier.notify_waiters();
        Ok(Response::new(SendMessageResponse { success: true }))
    }
}

async fn process_messages(state: Arc<State>) -> Result<(), Box<dyn Error>> {
    loop {
        state.notifier.notified().await;

        loop {
            let can_deliver = {
                let mss = state.messages.read().await;
                if mss.is_empty() {
                    break;
                }

                let front_clock = &mss.front().unwrap().clock.as_ref().unwrap();

                (0..=state.n_peers).all(|peer_id| {
                    if peer_id == front_clock.sender {
                        return true;
                    }
                    state.latest.try_read()
                        .ok()
                        .and_then(|latest| latest.get(&peer_id).copied())
                        .map(|ts| ts > front_clock.timestamp)
                        .unwrap_or(false)
                })
            };

            if !can_deliver {
                break;
            }

            if let Some(msg) = state.messages.write().await.pop_front() {
                let c = msg.clock.unwrap();
                cprintln!("<green>*{}:{}*</green> {}", c.sender, c.timestamp, msg.content);
            }
        }
    }
}

async fn broadcast_random_word(state: Arc<State>) -> Result<(), Box<dyn Error>> {
    let lambda = 1.0;
    let mut rng = SmallRng::from_rng(&mut rand::rng());

    loop {
        let u: f64 = rng.random_range(std::f64::EPSILON..1.0);
        let wait = -u.ln() / lambda;
        let i = rng.random_range(0..state.words.len());
        let word = state.words[i].clone();

        let msg = {
            let mut t = state.tick.write().await;
            *t += 1;
            SendMessageRequest {
                clock: Some(LClock {
                    sender: state.index,
                    timestamp: *t,
                }),
                content: word,
            }
        };

        {
            let mut latest = state.latest.write().await;
            latest.insert(state.index, msg.clock.as_ref().unwrap().timestamp);
        }

        {
            let mut mss = state.messages.write().await;
            mss.push_back(msg.clone());
            mss.make_contiguous().sort_by_key(|m| {
                let c = m.clock.as_ref().unwrap();
                (c.timestamp, c.sender)
            });
        }

        {
            let peers = state.peers.read().await;
            for (_, client) in peers.iter() {
                let msg = msg.clone();
                let mut c = client.clone();
                tokio::spawn(async move {
                    let _ = c.send_message(msg).await;
                });
            }
        }

        state.notifier.notify_waiters();
        sleep(Duration::from_secs_f64(wait)).await;
    }
}

async fn load_words(filename: &str) -> Result<Vec<String>, Box<dyn Error>> {
    let mut rcsv = csv::Reader::from_path(filename)?;
    let mut w = Vec::new();
    for result in rcsv.records() {
        let r = result?;
        if r.len() >= 4 {
            w.push(r[1].to_string());
            w.push(r[3].to_string());
        }
    }
    Ok(w)
}

async fn connect_to_peers(addrs: &[(u32, String)], idx: u32) -> HashMap<u32, MessageServiceClient<tonic::transport::Channel>> {
    let mut peers = HashMap::new();
    for (i, addr) in addrs {
        if *i == idx {
            continue;
        }
        loop {
            let url = if addr.starts_with("http://") {
                addr.clone()
            } else {
                format!("http://{}", addr)
            };
            match MessageServiceClient::connect(url).await {
                Ok(c) => {
                    cprintln!("<blue>*conn*</blue> connected to peer {}", i);
                    peers.insert(*i, c);
                    break;
                }
                Err(e) => {
                    cprintln!("<red>*err*</red> waiting to connect to peer {}: {}", i, e);
                    sleep(Duration::from_secs(2)).await;
                }
            }
        }
    }
    peers
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = args().collect();
    if args.len() < 3 {
        exit(1);
    }

    let idx: u32 = args[1].parse().expect("invalid index");
    let addrs: Vec<(u32, String)> = args[2..]
        .iter()
        .enumerate()
        .map(|(i, addr)| (i as u32, addr.clone()))
        .collect();

    let n = addrs.len() as u32 - 1;
    let my_addr = addrs[idx as usize].1.clone();

    let addr: SocketAddr = my_addr.parse()?;
    let service = MessageService::new(idx, HashMap::new(), n, "data/data.csv").await?;
    let state = service.state.clone();

    let s = state.clone();
    tokio::spawn(async move {
        if let Err(e) = Server::builder()
            .add_service(MessageServiceServer::new(MessageService { state: s }))
            .serve(addr)
            .await
        {
            cprintln!("<red>*err*</red> server error: {}", e);
        }
    });

    sleep(Duration::from_secs(2)).await;
    cprintln!("<blue>*conn*</blue> connecting to other peers");
    let peers = connect_to_peers(&addrs, idx).await;
    cprintln!("<blue>*conn*</blue> all peers connected!");

    {
        let mut p = state.peers.write().await;
        *p = peers;
    }

    let s = state.clone();
    tokio::spawn(async move {
        if let Err(e) = process_messages(s).await {
            cprintln!("<red>*err*</red> message processor error: {}", e);
        }
    });

    let s = state.clone();
    tokio::spawn(async move {
        if let Err(e) = broadcast_random_word(s).await {
            cprintln!("<red>*err*</red> broadcaster error: {}", e);
        }
    });

    loop {
        sleep(Duration::from_secs(3600)).await;
    }
}
