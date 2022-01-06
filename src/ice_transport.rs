use std::unimplemented;
use std::sync::Arc;
use std::convert::Infallible;
use webrtc::api::APIBuilder;
use webrtc::data_channel::data_channel_message::DataChannelMessage;
use webrtc::data_channel::RTCDataChannel;
use webrtc::ice_transport::ice_server::RTCIceServer;
use webrtc::peer_connection::configuration::RTCConfiguration;
use webrtc::peer_connection::peer_connection_state::RTCPeerConnectionState;
use webrtc::peer_connection::sdp::session_description::RTCSessionDescription;
use webrtc::peer_connection::RTCPeerConnection;
use webrtc::peer_connection::OnSignalingStateChangeHdlrFn;
use webrtc::peer_connection::OnPeerConnectionStateChangeHdlrFn;
use webrtc::peer_connection::OnDataChannelHdlrFn;
use webrtc::ice_transport::ice_gatherer::OnLocalCandidateHdlrFn;

pub async fn new_peer_connection() -> Option<RTCPeerConnection> {
    let api = APIBuilder::new().build();
    let config = RTCConfiguration {
        ice_servers: vec![RTCIceServer {
            urls: vec!["stun:stun.l.google.com:19302".to_owned()],
            ..Default::default()
        }],
        ..Default::default()
    };
    api.new_peer_connection(config).await.ok()

}


pub struct IceTransport {
    pub candidate: Option<String>,
    pub connection: Option<RTCPeerConnection>
}

impl IceTransport {
    pub async fn new() -> Self {
        unimplemented!();
    }

    pub async fn new_peer_connection() -> Self {
        let api = APIBuilder::new().build();
        let config = RTCConfiguration {
            ice_servers: vec![RTCIceServer {
                urls: vec!["stun:stun.l.google.com:19302".to_owned()],
                ..Default::default()
            }],
            ..Default::default()
        };

        let conn = api.new_peer_connection(config).await.ok();

        return  Self {
            candidate: None,
            connection: conn
        }
    }

    pub async fn on_peer_state_change(&mut self, f: OnPeerConnectionStateChangeHdlrFn) ->  Result<(), ()> {
        match &self.connection {
            Some(c) => {
                c.on_peer_connection_state_change(f).await;
                Ok(())
            },
            _ => Err(())
        }
    }

    pub async fn on_data_channel(&mut self, f: OnDataChannelHdlrFn) -> Result<(), ()>{
        match &self.connection {
            Some(c) => {
                c.on_data_channel(f).await;
                Ok(())
            },
            _ => Err(())
        }
    }


    pub async fn on_candiate(&mut self, f: OnLocalCandidateHdlrFn) -> Result<(), ()>{
        match &self.connection {
            Some(c) => {
                c.on_ice_candidate(f).await;
                Ok(())
            },
            _ => Err(())
        }
    }




    pub async fn candidate(&self) -> Option<String> {
        unimplemented!();
    }

}
