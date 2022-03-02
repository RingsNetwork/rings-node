use crate::channels::default::AcChannel;
use crate::dht::message::Procedures;
use crate::ecc::SecretKey;
use crate::encoder::Encoded;
use crate::msg::SignedMsg;
use crate::types::channel::Channel;
use crate::types::channel::Events;
use crate::types::ice_transport::IceTransport;
use crate::types::ice_transport::IceTransportCallback;
use crate::types::ice_transport::IceTrickleScheme;
use anyhow::anyhow;
use anyhow::Result;
use async_trait::async_trait;
use futures::future::join_all;
use serde::Deserialize;
use serde::Serialize;
use serde_json;
use std::sync::Arc;
use tokio::sync::Mutex;

use web3::types::Address;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_candidate::{RTCIceCandidate, RTCIceCandidateInit};
use webrtc::ice_transport::ice_connection_state::RTCIceConnectionState;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;

use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::sdp_type::RTCSdpType;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;

#[derive(Clone)]
pub struct DefaultTransport {
    pub connection: Arc<Mutex<Option<Arc<RTCPeerConnection>>>>,
    pub pending_candidates: Arc<Mutex<Vec<RTCIceCandidate>>>,
    pub data_channel: Arc<Mutex<Option<Arc<RTCDataChannel>>>>,
    pub signaler: Arc<AcChannel>,
    pub procedures: Arc<Procedures>,
}

#[async_trait]
impl IceTransport<AcChannel> for DefaultTransport {
    type Connection = RTCPeerConnection;
    type Candidate = RTCIceCandidate;
    type Sdp = RTCSessionDescription;
    type DataChannel = RTCDataChannel;
    type IceConnectionState = RTCIceConnectionState;
    type ConnectionState = RTCPeerConnectionState;
    type Msg = DataChannelMessage;

    fn new(ch: Arc<AcChannel>, proc: Arc<Procedures>) -> Self {
        Self {
            connection: Arc::new(Mutex::new(None)),
            pending_candidates: Arc::new(Mutex::new(vec![])),
            data_channel: Arc::new(Mutex::new(None)),
            signaler: Arc::clone(&ch),
            procedures: Arc::clone(&proc),
        }
    }

    fn signaler(&self) -> Arc<AcChannel> {
        Arc::clone(&self.signaler)
    }

    async fn start(&mut self, stun: String) -> Result<()> {
        let config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: vec![stun.to_owned()],
                ..Default::default()
            }],
            ice_candidate_pool_size: 100,
            ..Default::default()
        };

        let api = APIBuilder::new().build();
        match api.new_peer_connection(config).await {
            Ok(c) => {
                let mut conn = self.connection.lock().await;
                *conn = Some(Arc::new(c));
                Ok(())
            }
            Err(e) => Err(anyhow!(e)),
        }?;

        self.setup_channel("bns").await?;
        self.on_ice_candidate(self.on_ice_candidate_callback().await)
            .await?;
        self.on_peer_connection_state_change(self.on_peer_connection_state_change_callback().await)
            .await?;
        self.on_data_channel(self.on_data_channel_callback().await)
            .await?;

        Ok(())
    }

    async fn close(&self) -> Result<()> {
        if let Some(pc) = self.get_peer_connection().await {
            pc.close().await?;
        }

        Ok(())
    }

    async fn ice_connection_state(&self) -> Option<Self::IceConnectionState> {
        self.get_peer_connection()
            .await
            .map(|pc| pc.ice_connection_state())
    }

    async fn get_peer_connection(&self) -> Option<Arc<RTCPeerConnection>> {
        self.connection.lock().await.clone()
    }

    async fn get_pending_candidates(&self) -> Vec<RTCIceCandidate> {
        self.pending_candidates.lock().await.to_vec()
    }

    async fn get_answer(&self) -> Result<RTCSessionDescription> {
        match self.get_peer_connection().await {
            Some(peer_connection) => {
                // wait gather candidates
                let mut gather_complete = peer_connection.gathering_complete_promise().await;
                let answer = peer_connection.create_answer(None).await?;
                self.set_local_description(answer.to_owned()).await?;
                let _ = gather_complete.recv().await;
                Ok(answer)
            }
            None => Err(anyhow!("cannot get answer")),
        }
    }

    async fn get_answer_str(&self) -> Result<String> {
        Ok(self.get_answer().await?.sdp)
    }

    async fn get_offer(&self) -> Result<RTCSessionDescription> {
        match self.get_peer_connection().await {
            Some(peer_connection) => {
                // wait gather candidates
                let mut gather_complete = peer_connection.gathering_complete_promise().await;
                match peer_connection.create_offer(None).await {
                    Ok(offer) => {
                        self.set_local_description(offer.to_owned()).await?;
                        let _ = gather_complete.recv().await;
                        Ok(offer)
                    }
                    Err(e) => {
                        log::error!("{}", e);
                        Err(anyhow!(e))
                    }
                }
            }
            None => Err(anyhow!("cannot get offer")),
        }
    }

    async fn get_offer_str(&self) -> Result<String> {
        Ok(self.get_offer().await?.sdp)
    }

    async fn get_data_channel(&self) -> Option<Arc<RTCDataChannel>> {
        self.data_channel.lock().await.clone()
    }

    async fn send_message<T>(&self, msg: T) -> Result<()>
    where
        T: Serialize + Send,
    {
        let data = serde_json::to_string(&msg)?;
        match self.get_data_channel().await {
            Some(cnn) => match cnn.send_text(data.to_owned()).await {
                Ok(s) => {
                    if !s == data.to_owned().len() {
                        Err(anyhow!("msg is not complete, {:?}!= {:?}", s, data.len()))
                    } else {
                        Ok(())
                    }
                }
                Err(e) => Err(anyhow!(e)),
            },
            None => Err(anyhow!("data channel may not ready")),
        }
    }

    async fn add_ice_candidate(&self, candidate: String) -> Result<()> {
        match self.get_peer_connection().await {
            Some(peer_connection) => peer_connection
                .add_ice_candidate(RTCIceCandidateInit {
                    candidate,
                    ..Default::default()
                })
                .await
                .map_err(|e| anyhow!(e)),
            None => Err(anyhow!("cannot add ice candidate")),
        }
    }

    async fn set_local_description<T>(&self, desc: T) -> Result<()>
    where
        T: Into<RTCSessionDescription> + Send,
    {
        match self.get_peer_connection().await {
            Some(peer_connection) => peer_connection
                .set_local_description(desc.into())
                .await
                .map_err(|e| anyhow!(e)),
            None => Err(anyhow!("cannot get local description")),
        }
    }

    async fn set_remote_description<T>(&self, desc: T) -> Result<()>
    where
        T: Into<RTCSessionDescription> + Send,
    {
        match self.get_peer_connection().await {
            Some(peer_connection) => peer_connection
                .set_remote_description(desc.into())
                .await
                .map_err(|e| anyhow!(e)),
            None => Err(anyhow!("connection is not setup")),
        }
    }
}

impl DefaultTransport {
    pub async fn setup_channel(&mut self, name: &str) -> Result<()> {
        match self.get_peer_connection().await {
            Some(peer_connection) => {
                let channel = peer_connection.create_data_channel(name, None).await;
                match channel {
                    Ok(ch) => {
                        let mut channel = self.data_channel.lock().await;
                        *channel = Some(ch);
                        Ok(())
                    }
                    Err(e) => Err(anyhow!(e)),
                }
            }
            None => Err(anyhow!("cannot get data channel")),
        }
    }
}

#[async_trait]
impl IceTransportCallback<AcChannel> for DefaultTransport {
    async fn on_ice_candidate(&self, f: Self::OnLocalCandidateHdlrFn) -> Result<()> {
        match self.get_peer_connection().await {
            Some(peer_connection) => {
                peer_connection.on_ice_candidate(f).await;
                Ok(())
            }
            None => Err(anyhow!("connection is not setup")),
        }
    }

    async fn on_peer_connection_state_change(
        &self,
        f: Self::OnPeerConnectionStateChangeHdlrFn,
    ) -> Result<()> {
        match self.get_peer_connection().await {
            Some(peer_connection) => {
                peer_connection.on_peer_connection_state_change(f).await;
            }
            None => panic!("Connection Failed."),
        }
        Ok(())
    }

    async fn on_data_channel(&self, f: Self::OnDataChannelHdlrFn) -> Result<()> {
        match self.get_peer_connection().await {
            Some(peer_connection) => {
                peer_connection.on_data_channel(f).await;
            }
            None => panic!("Connection Failed."),
        }
        Ok(())
    }

    async fn on_ice_candidate_callback(&self) -> Self::OnLocalCandidateHdlrFn {
        let peer_connection = self.get_peer_connection().await;
        let pending_candidates = Arc::clone(&self.pending_candidates);

        box move |c: Option<<Self as IceTransport<AcChannel>>::Candidate>| {
            let peer_connection = peer_connection.clone();
            let pending_candidates = Arc::clone(&pending_candidates);
            Box::pin(async move {
                if let Some(candidate) = c {
                    if let Some(peer_connection) = peer_connection {
                        let desc = peer_connection.remote_description().await;
                        if desc.is_none() {
                            let mut candidates = pending_candidates.lock().await;
                            candidates.push(candidate.clone());
                            println!("Candidates Number: {:?}", candidates.len());
                        }
                    }
                }
            })
        }
    }

    async fn on_peer_connection_state_change_callback(
        &self,
    ) -> Self::OnPeerConnectionStateChangeHdlrFn {
        let sender = self.signaler().sender();
        box move |s: RTCPeerConnectionState| {
            if s == RTCPeerConnectionState::Failed {
                let _ = sender.send(Events::ConnectFailed);
            }
            Box::pin(async move {
                log::debug!("Connect State changed to {:?}", s);
            })
        }
    }

    async fn on_data_channel_callback(&self) -> Self::OnDataChannelHdlrFn {
        let channel = self.get_data_channel().await.unwrap();
        let signaler = self.signaler();
        let procedures = Arc::clone(&self.procedures);

        box move |d: Arc<RTCDataChannel>| {
            let channel = Arc::clone(&channel);
            let on_open_signaler = Arc::clone(&signaler);
            let on_message_signaler = Arc::clone(&signaler);
            let on_open_procedures = Arc::clone(&procedures);
            let on_message_procedures = Arc::clone(&procedures);
            Box::pin(async move {
                d.on_open(Box::new(move || {
                    let _channel = Arc::clone(&channel);
                    Box::pin(async move {
                        // do nothing here
                    })
                }))
                .await;

                d.on_message(Box::new(move |msg: DataChannelMessage| {
                    log::debug!("Message from DataChannel: '{:?}'", msg);
                    let signaler = Arc::clone(&on_message_signaler);
                    let procedures = Arc::clone(&on_open_procedures);
                    Box::pin(async move {
                        if signaler
                            .send(Events::ReceiveMsg(msg.data.to_vec()))
                            .await
                            .is_err()
                        {
                            log::error!("Failed on handle msg")
                        };
                    })
                }))
                .await;
            })
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct TricklePayload {
    pub sdp: String,
    pub candidates: Vec<RTCIceCandidateInit>,
}

#[async_trait]
impl IceTrickleScheme<AcChannel> for DefaultTransport {
    // https://datatracker.ietf.org/doc/html/rfc5245
    // 1. Send (SdpOffer, IceCandidates) to remote
    // 2. Recv (SdpAnswer, IceCandidate) From Remote

    type SdpType = RTCSdpType;

    async fn get_handshake_info(&self, key: SecretKey, kind: RTCSdpType) -> Result<Encoded> {
        log::trace!("prepareing handshake info {:?}", kind);
        let sdp = match kind {
            RTCSdpType::Answer => self.get_answer().await?,
            RTCSdpType::Offer => self.get_offer().await?,
            kind => {
                let mut sdp = self.get_offer().await?;
                sdp.sdp_type = kind;
                sdp
            }
        };
        let local_candidates_json = join_all(
            self.get_pending_candidates()
                .await
                .iter()
                .map(async move |c| c.clone().to_json().await.unwrap()),
        )
        .await;
        let data = TricklePayload {
            sdp: serde_json::to_string(&sdp).unwrap(),
            candidates: local_candidates_json,
        };
        log::trace!("prepared hanshake info :{:?}", data);
        let resp = SignedMsg::new(data, &key, None)?;
        Ok(resp.try_into()?)
    }

    async fn register_remote_info(&self, data: Encoded) -> anyhow::Result<Address> {
        let data: SignedMsg<TricklePayload> = data.try_into()?;
        log::trace!("register remote info: {:?}", data);

        match data.verify() {
            true => {
                let sdp = serde_json::from_str::<RTCSessionDescription>(&data.data.sdp)?;
                log::trace!("setting remote sdp: {:?}", sdp);
                self.set_remote_description(sdp).await?;
                log::trace!("setting remote candidate");
                for c in data.data.candidates {
                    log::trace!("add candiates: {:?}", c);
                    self.add_ice_candidate(c.candidate.clone()).await?;
                }
                Ok(data.addr)
            }
            _ => {
                log::error!("cannot verify message sig");
                return Err(anyhow!("failed on verify message sigature"));
            }
        }
    }
}
