use anyhow::Result;
use libp2p::futures::StreamExt;
use libp2p::request_response::{self, ProtocolSupport};
use libp2p::{
    gossipsub, identify, identity, kad, quic, swarm::Config as SwarmConfig,
    swarm::NetworkBehaviour, swarm::SwarmEvent, Multiaddr, PeerId, Swarm,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

const PEER_SCORE_BLACKLIST_THRESHOLD: i32 = -25;
const PEER_SCORE_INVALID_MSG: i32 = -5;
const PEER_SCORE_VALID_MSG: i32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncRequest {
    BlockByHeight(u64),
    ZkProofByBlock([u8; 32]),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncResponse {
    Block(Option<Vec<u8>>),
    ZkProof(Option<Vec<u8>>),
    Error(String),
}

#[derive(NetworkBehaviour)]
struct LuminaBehaviour {
    gossipsub: gossipsub::Behaviour,
    identify: identify::Behaviour,
    kademlia: kad::Behaviour<kad::store::MemoryStore>,
    req_res: request_response::cbor::Behaviour<SyncRequest, SyncResponse>,
}

pub struct P2PNetwork {
    swarm: Swarm<LuminaBehaviour>,
    command_receiver: mpsc::Receiver<NetworkCommand>,
    event_sender: mpsc::Sender<NetworkEvent>,
    peer_scores: HashMap<PeerId, i32>,
    blacklisted_peers: HashSet<PeerId>,
    block_topic: gossipsub::IdentTopic,
    tx_topic: gossipsub::IdentTopic,
}

pub enum NetworkCommand {
    BroadcastBlock(Vec<u8>),
    BroadcastTx(Vec<u8>),
    RequestBlock {
        peer: PeerId,
        height: u64,
    },
    RequestZkProof {
        peer: PeerId,
        block_hash: [u8; 32],
    },
    RespondSync {
        channel: request_response::ResponseChannel<SyncResponse>,
        response: SyncResponse,
    },
    AddBootstrapPeer(Multiaddr),
}

pub enum NetworkEvent {
    BlockReceived(Vec<u8>, PeerId),
    TxReceived(Vec<u8>, PeerId),
    PeerDiscovered(PeerId),
    SyncRequest {
        peer: PeerId,
        request: SyncRequest,
        channel: request_response::ResponseChannel<SyncResponse>,
    },
    SyncResponse {
        peer: PeerId,
        response: SyncResponse,
    },
    PeerBlacklisted(PeerId),
}

impl P2PNetwork {
    pub async fn new(
        command_receiver: mpsc::Receiver<NetworkCommand>,
        event_sender: mpsc::Sender<NetworkEvent>,
    ) -> Result<Self> {
        let id_keys = identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(id_keys.public());
        info!(%peer_id, "Local Peer ID");

        // QUIC on libp2p enforces TLS 1.3 for secure transport.
        let quic_transport = quic::tokio::Transport::new(quic::Config::new(&id_keys));

        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .mesh_n(8)
            .mesh_n_low(6)
            .mesh_n_high(12)
            .build()
            .expect("valid gossipsub config");

        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(id_keys.clone()),
            gossipsub_config,
        )
        .map_err(|e| anyhow::anyhow!(e))?;

        let identify = identify::Behaviour::new(identify::Config::new(
            "/lumina/1.0.0".to_owned(),
            id_keys.public(),
        ));
        let kademlia = kad::Behaviour::new(peer_id, kad::store::MemoryStore::new(peer_id));
        let req_res = request_response::cbor::Behaviour::new(
            [(
                libp2p::StreamProtocol::new("/lumina/sync/1"),
                ProtocolSupport::Full,
            )],
            request_response::Config::default(),
        );

        let behaviour = LuminaBehaviour {
            gossipsub,
            identify,
            kademlia,
            req_res,
        };

        let swarm = Swarm::new(
            quic_transport,
            behaviour,
            peer_id,
            SwarmConfig::with_tokio_executor(),
        );

        Ok(Self {
            swarm,
            command_receiver,
            event_sender,
            peer_scores: HashMap::new(),
            blacklisted_peers: HashSet::new(),
            block_topic: gossipsub::IdentTopic::new("lumina-blocks"),
            tx_topic: gossipsub::IdentTopic::new("lumina-txs"),
        })
    }

    fn adjust_peer_score(&mut self, peer: PeerId, delta: i32) {
        if self.blacklisted_peers.contains(&peer) {
            return;
        }

        let score = self.peer_scores.entry(peer).or_insert(0);
        *score += delta;
        if *score <= PEER_SCORE_BLACKLIST_THRESHOLD {
            self.blacklisted_peers.insert(peer);
            warn!(%peer, score = *score, "Peer blacklisted due to low score");
            let _ = self
                .event_sender
                .try_send(NetworkEvent::PeerBlacklisted(peer));
        }
    }

    fn should_ignore_peer(&self, peer: PeerId) -> bool {
        self.blacklisted_peers.contains(&peer)
    }

    pub async fn run(mut self) {
        if let Err(e) = self
            .swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.block_topic)
        {
            error!(?e, "Failed to subscribe to block topic");
        }
        if let Err(e) = self
            .swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.tx_topic)
        {
            error!(?e, "Failed to subscribe to tx topic");
        }

        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(LuminaBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                        propagation_source: peer_id,
                        message,
                        ..
                    })) => {
                        if self.should_ignore_peer(peer_id) {
                            continue;
                        }
                        let topic = message.topic.clone();
                        if topic == self.block_topic.hash() {
                            let _ = self.event_sender.send(NetworkEvent::BlockReceived(message.data, peer_id)).await;
                            self.adjust_peer_score(peer_id, PEER_SCORE_VALID_MSG);
                        } else if topic == self.tx_topic.hash() {
                            let _ = self.event_sender.send(NetworkEvent::TxReceived(message.data, peer_id)).await;
                            self.adjust_peer_score(peer_id, PEER_SCORE_VALID_MSG);
                        } else {
                            self.adjust_peer_score(peer_id, PEER_SCORE_INVALID_MSG);
                        }
                    },
                    SwarmEvent::Behaviour(LuminaBehaviourEvent::Identify(identify::Event::Received { peer_id, info, .. })) => {
                        if self.should_ignore_peer(peer_id) {
                            continue;
                        }
                        for addr in info.listen_addrs {
                            self.swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                        }
                        let _ = self.event_sender.send(NetworkEvent::PeerDiscovered(peer_id)).await;
                    },
                    SwarmEvent::Behaviour(LuminaBehaviourEvent::ReqRes(request_response::Event::Message { peer, message, .. })) => {
                        if self.should_ignore_peer(peer) {
                            continue;
                        }
                        match message {
                            request_response::Message::Request { request, channel, .. } => {
                                let _ = self.event_sender.send(NetworkEvent::SyncRequest { peer, request, channel }).await;
                            }
                            request_response::Message::Response { response, .. } => {
                                let _ = self.event_sender.send(NetworkEvent::SyncResponse { peer, response }).await;
                            }
                        }
                        self.adjust_peer_score(peer, PEER_SCORE_VALID_MSG);
                    }
                    SwarmEvent::Behaviour(LuminaBehaviourEvent::ReqRes(request_response::Event::OutboundFailure { peer, error, .. })) => {
                        warn!(%peer, ?error, "Outbound sync request failed");
                        self.adjust_peer_score(peer, PEER_SCORE_INVALID_MSG);
                    }
                    SwarmEvent::Behaviour(LuminaBehaviourEvent::ReqRes(request_response::Event::InboundFailure { peer, error, .. })) => {
                        warn!(%peer, ?error, "Inbound sync request failed");
                        self.adjust_peer_score(peer, PEER_SCORE_INVALID_MSG);
                    }
                    SwarmEvent::NewListenAddr { address, .. } => {
                        info!(%address, "Listening on QUIC/TLS 1.3");
                    },
                    _ => {}
                },
                command = self.command_receiver.recv() => match command {
                    Some(NetworkCommand::BroadcastBlock(data)) => {
                        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(self.block_topic.clone(), data) {
                            error!(?e, "Block publish error");
                        }
                    },
                    Some(NetworkCommand::BroadcastTx(data)) => {
                        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(self.tx_topic.clone(), data) {
                            error!(?e, "Tx publish error");
                        }
                    },
                    Some(NetworkCommand::RequestBlock { peer, height }) => {
                        if !self.should_ignore_peer(peer) {
                            self.swarm.behaviour_mut().req_res.send_request(&peer, SyncRequest::BlockByHeight(height));
                        }
                    }
                    Some(NetworkCommand::RequestZkProof { peer, block_hash }) => {
                        if !self.should_ignore_peer(peer) {
                            self.swarm.behaviour_mut().req_res.send_request(&peer, SyncRequest::ZkProofByBlock(block_hash));
                        }
                    }
                    Some(NetworkCommand::RespondSync { channel, response }) => {
                        if let Err(e) = self.swarm.behaviour_mut().req_res.send_response(channel, response) {
                            error!(?e, "Sync response send error");
                        }
                    }
                    Some(NetworkCommand::AddBootstrapPeer(addr)) => {
                        if let Err(e) = self.swarm.dial(addr.clone()) {
                            warn!(%addr, ?e, "Failed dialing bootstrap peer");
                        }
                    }
                    None => break,
                }
            }
        }
    }
}

pub async fn start_p2p() -> Result<(mpsc::Sender<NetworkCommand>, mpsc::Receiver<NetworkEvent>)> {
    let (cmd_tx, cmd_rx) = mpsc::channel(100);
    let (event_tx, event_rx) = mpsc::channel(100);

    let mut network = P2PNetwork::new(cmd_rx, event_tx).await?;
    network
        .swarm
        .listen_on("/ip4/0.0.0.0/udp/0/quic-v1".parse()?)?;

    if let Ok(bootstrap) = std::env::var("LUMINA_BOOTSTRAP_PEERS") {
        for addr in bootstrap.split(',').filter(|s| !s.trim().is_empty()) {
            if let Ok(ma) = addr.trim().parse::<Multiaddr>() {
                if let Err(e) = network.swarm.dial(ma.clone()) {
                    warn!(%ma, ?e, "Failed dialing bootstrap peer");
                }
            }
        }
    }

    info!("Starting P2P Network (QUIC + TLS 1.3)...");
    tokio::spawn(async move {
        network.run().await;
    });

    Ok((cmd_tx, event_rx))
}
