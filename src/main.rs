// github token
// id: ingubae
// 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


use libp2p::{
    core::upgrade,
    futures::StreamExt,
    mplex,
    noise::{Keypair, NoiseConfig, X25519Spec},
    swarm::{Swarm, SwarmBuilder},
    tcp::TokioTcpConfig,
    Transport,
};
use log::{error, info, debug};
use std::time::Duration;
use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    sync::mpsc,
    time::sleep,
};

use crate::block::App;

mod p2p;
mod block;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    println!("Hello, world!");

    info!("Peer ID: {}", p2p::PEER_ID.clone());
    let (resp_sender, mut resp_recv) = mpsc::unbounded_channel();
    let (init_sender, mut init_recv) = mpsc::unbounded_channel();

    let auth_keys = Keypair::<X25519Spec>::new()
        .into_authentic(&p2p::KEYS)
        .expect("can create auth keys");
    let transp = TokioTcpConfig::new()
        .upgrade(upgrade::Version::V1)
        .authenticate(NoiseConfig::xx(auth_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();
    let behaviour = p2p::AppBehaviour::new(App::new(), resp_sender, init_sender.clone()).await;

    let mut swarm = SwarmBuilder::new(transp, behaviour, *p2p::PEER_ID)
        .executor(Box::new(|fut| {tokio::spawn(fut); }))
        .build();

    let mut stdin = BufReader::new(stdin()).lines();

    Swarm::listen_on(
        &mut swarm, 
        "/ip4/0.0.0.0/tcp/0".parse().expect("can get a local socket")
    )
    .expect("swarm can be started");

    tokio::spawn(async move {
        sleep(Duration::from_secs(1)).await;
        info!("Sending init event");
        init_sender.send(true).expect("can send init event");
    });

    loop {
        let evt = {
            tokio::select! {
                _init = init_recv.recv() => {
                    Some(p2p::EventType::Init)
                }
                line = stdin.next_line() => {
                    Some(p2p::EventType::Input(line.expect("can get line").expect("can read line")))
                }
                resp = resp_recv.recv() => {
                    Some(p2p::EventType::LocalChainResponse(resp.expect("response exists")))
                }
                event = swarm.select_next_some() => {
                    debug!("Unhandled Swarm Event: {:?}", event);
                    None
                }
            }
        };

        if let Some(event) = evt {
            match event {
                p2p::EventType::Init => {
                    let peers = p2p::get_list_peers(&swarm);
                    swarm.behaviour_mut().app.genesis();

                    info!("connected nodes: {}", peers.len());
                    if !peers.is_empty() {
                        let req = p2p::LocalChainRequest {
                            from_peer_id: peers
                                .iter()
                                .last()
                                .expect("at least one peer")
                                .to_string(),
                        };

                        let json = serde_json::to_string(&req).expect("can jsonify request");
                        swarm
                            .behaviour_mut()
                            .floodsub
                            .publish(p2p::CHAIN_TOPIC.clone(), json.as_bytes());
                    }
                }
                p2p::EventType::Input(line) => {
                    match line.as_str() {
                        "ls p" => p2p::handle_print_peers(&swarm),
                        cmd if cmd.starts_with("ls c") => p2p::handle_print_chain(&swarm),
                        cmd if cmd.starts_with("create b") => p2p::handle_create_block(cmd, &mut swarm),
                        _ => error!("unknown command"),
                    }
                }
                p2p::EventType::LocalChainResponse(resp) => {
                    let json = serde_json::to_string(&resp).expect("can jsonify request"); 
                    swarm
                        .behaviour_mut()
                        .floodsub
                        .publish(p2p::CHAIN_TOPIC.clone(), json.as_bytes());
                }
            }
        }
    }
}
