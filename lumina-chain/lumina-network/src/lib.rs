use libp2p::{
    gossipsub, noise, swarm::Config as SwarmConfig, swarm::NetworkBehaviour, swarm::SwarmEvent, tcp, yamux, Multiaddr, PeerId, Swarm, Transport,
};
use libp2p::futures::StreamExt;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{info, error};
use anyhow::Result;

// Define the NetworkBehaviour
#[derive(NetworkBehaviour)]
struct LuminaBehaviour {
    gossipsub: gossipsub::Behaviour,
    // request_response: request_response::Behaviour<...>, // For block sync
}

pub struct P2PNetwork {
    swarm: Swarm<LuminaBehaviour>,
    command_receiver: mpsc::Receiver<NetworkCommand>,
    event_sender: mpsc::Sender<NetworkEvent>,
}

pub enum NetworkCommand {
    BroadcastBlock(Vec<u8>),
    BroadcastTx(Vec<u8>),
}

pub enum NetworkEvent {
    BlockReceived(Vec<u8>, PeerId),
    TxReceived(Vec<u8>, PeerId),
}

impl P2PNetwork {
    pub async fn new(
        command_receiver: mpsc::Receiver<NetworkCommand>,
        event_sender: mpsc::Sender<NetworkEvent>,
    ) -> Result<Self> {
        let id_keys = libp2p::identity::Keypair::generate_ed25519();
        let peer_id = PeerId::from(id_keys.public());
        info!("Local Peer ID: {}", peer_id);

        let tcp_transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true))
            .upgrade(libp2p::core::upgrade::Version::V1)
            .authenticate(noise::Config::new(&id_keys)?)
            .multiplex(yamux::Config::default())
            .boxed();

        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(1))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .build()
            .expect("Valid config");

        let gossipsub = gossipsub::Behaviour::new(
            gossipsub::MessageAuthenticity::Signed(id_keys),
            gossipsub_config,
        )
        .map_err(|e| anyhow::anyhow!(e))?;

        let behaviour = LuminaBehaviour { gossipsub };

        let swarm = Swarm::new(tcp_transport, behaviour, peer_id, SwarmConfig::with_tokio_executor());

        Ok(Self {
            swarm,
            command_receiver,
            event_sender,
        })
    }

    pub async fn run(mut self) {
        // Subscribe to topics
        let block_topic = gossipsub::IdentTopic::new("lumina-blocks");
        let tx_topic = gossipsub::IdentTopic::new("lumina-txs");
        
        if let Err(e) = self.swarm.behaviour_mut().gossipsub.subscribe(&block_topic) {
             error!("Subscription error: {:?}", e);
        }
        if let Err(e) = self.swarm.behaviour_mut().gossipsub.subscribe(&tx_topic) {
             error!("Subscription error: {:?}", e);
        }

        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(LuminaBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                        propagation_source: peer_id,
                        message_id: _,
                        message,
                    })) => {
                        let topic = message.topic.clone();
                        if topic == block_topic.hash() {
                            let _ = self.event_sender.send(NetworkEvent::BlockReceived(message.data, peer_id)).await;
                        } else if topic == tx_topic.hash() {
                             let _ = self.event_sender.send(NetworkEvent::TxReceived(message.data, peer_id)).await;
                        }
                    },
                    SwarmEvent::NewListenAddr { address, .. } => {
                        info!("Listening on {:?}", address);
                    },
                    _ => {}
                },
                command = self.command_receiver.recv() => match command {
                    Some(NetworkCommand::BroadcastBlock(data)) => {
                        if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(block_topic.clone(), data) {
                             error!("Publish error: {:?}", e);
                        }
                    },
                    Some(NetworkCommand::BroadcastTx(data)) => {
                         if let Err(e) = self.swarm.behaviour_mut().gossipsub.publish(tx_topic.clone(), data) {
                             error!("Publish error: {:?}", e);
                        }
                    },
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
    
    // Listen on all interfaces
    network.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    info!("Starting P2P Network...");
    tokio::spawn(async move {
        network.run().await;
    });

    Ok((cmd_tx, event_rx))
}
