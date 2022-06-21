//! Error of rings_core

#[derive(thiserror::Error, Debug)]
#[non_exhaustive]
pub enum Error {
    #[error("InvalidPublicKey")]
    InvalidPublicKey,

    #[error("Address of Vritual Peer not equal")]
    AddressNotEqual,

    #[error("Encode a byte vector into a base58-check string, adds 4 bytes checksum")]
    Encode,

    #[error("Decode base58-encoded with 4 bytes checksum string into a byte vector")]
    Decode,

    #[error("Couldn't decode data as UTF-8.")]
    Utf8Encoding(#[from] std::string::FromUtf8Error),

    #[error("Invalid hexadecimal id in directory cache")]
    BadHexInCache(#[from] hex::FromHexError),

    #[error("Invalid rustc hexadecimal id in directory cache")]
    BadCHexInCache,

    #[error("URL parse error")]
    URLParse(#[from] url::ParseError),

    #[error("Invalid hexadecimal id in directory cache")]
    BadArrayInCache(#[from] std::array::TryFromSliceError),

    #[error("JSON serialize toString error")]
    SerializeToString,

    #[error("JSON serialization error")]
    Serialize(#[source] serde_json::Error),

    #[error("JSON deserialization error")]
    Deserialize(#[source] serde_json::Error),

    #[error("Bincode serialization error")]
    BincodeSerialize(#[source] bincode::Error),

    #[error("Bincode deserialization error")]
    BincodeDeserialize(#[source] bincode::Error),

    #[error("Failed on verify message signature")]
    VerifySignatureFailed,

    #[error("Gzip encode error.")]
    GzipEncode,

    #[error("Gzip decode error.")]
    GzipDecode,

    #[error("Failed on promise, state not successed")]
    PromiseStateFailed,

    #[error("Ice server scheme {0} has not supported yet")]
    IceServerSchemeNotSupport(String),

    #[error("Ice server get url without host")]
    IceServerURLMissHost,

    #[error("SecretKey parse error, {0}")]
    Libsecp256k1SecretKeyParse(String),

    #[error("Signature standard parse failed, {0}")]
    Libsecp256k1SignatureParseStandard(String),

    #[error("RecoverId parse failed, {0}")]
    Libsecp256k1RecoverIdParse(String),

    #[error("Libsecp256k1 recover failed")]
    Libsecp256k1Recover,

    #[error("Unsupport message type, {0}")]
    MessageHandlerUnsupportMessageType(String),

    #[error("Cannot find next node by local DHT")]
    MessageHandlerMissNextNode,

    #[error("Receive `AlreadyConnected`` but cannot get transport")]
    MessageHandlerMissTransportAlreadyConnected,

    #[error("Cannot get trans when handle connect node response")]
    MessageHandlerMissTransportConnectedNode,

    #[error("Send message through channel failed")]
    ChannelSendMessageFailed,

    #[error("Recv message through channel failed")]
    ChannelRecvMessageFailed,

    #[error("Invalid PeerRingAction")]
    PeerRingInvalidAction,

    #[error("Failed on TryInto VNode")]
    PeerRingInvalidVNode,

    #[error("Unexpected PeerRingAction, {0:?}")]
    PeerRingUnexpectedAction(crate::dht::PeerRingAction),

    #[error("PeerRing findsuccessor error, {0}")]
    PeerRingFindSuccessor(String),

    #[error("PeerRing cannot find cloest preceding node")]
    PeerRingNotFindCloestNode,

    #[error("PeerRing RWLock unlock failed")]
    PeerRingUnlockFailed,

    #[error("Cannot seek address in swarm table")]
    SwarmMissAddressInTable,

    #[error("Cannot get transport from address: {0}")]
    SwarmMissTransport(web3::types::Address),

    #[error("Load message failed with message: {0}")]
    SwarmLoadMessageRecvFailed(String),

    #[error("Default transport is not connected")]
    SwarmDefaultTransportNotConnected,

    #[error("call lock() failed")]
    SwarmPendingTransTryLockFailed,

    #[error("transport not found")]
    SwarmPendingTransNotFound,

    #[error("failed to close previous when registering, {0}")]
    SwarmToClosePrevTransport(String),

    #[error("call lock() failed")]
    SessionTryLockFailed,

    #[error("Invalid peer type")]
    InvalidPeerType,

    #[cfg(not(feature = "wasm"))]
    #[error("RTC new peer connection failed")]
    RTCPeerConnectionCreateFailed(#[source] webrtc::Error),

    #[error("RTC peer_connection not establish")]
    RTCPeerConnectionNotEstablish,

    #[cfg(not(feature = "wasm"))]
    #[error("RTC peer_connection fail to create offer")]
    RTCPeerConnectionCreateOfferFailed(#[source] webrtc::Error),

    #[cfg(feature = "wasm")]
    #[error("RTC peer_connection fail to create offer")]
    RTCPeerConnectionCreateOfferFailed(String),

    #[cfg(not(feature = "wasm"))]
    #[error("RTC peer_connection fail to create answer")]
    RTCPeerConnectionCreateAnswerFailed(#[source] webrtc::Error),

    #[cfg(feature = "wasm")]
    #[error("RTC peer_connection fail to create answer")]
    RTCPeerConnectionCreateAnswerFailed(String),

    #[error("DataChannel message size not match, {0} < {1}")]
    RTCDataChannelMessageIncomplete(usize, usize),

    #[cfg(not(feature = "wasm"))]
    #[error("DataChannel send text message failed")]
    RTCDataChannelSendTextFailed(#[source] webrtc::Error),

    #[cfg(feature = "wasm")]
    #[error("DataChannel send text message failed, {0}")]
    RTCDataChannelSendTextFailed(String),

    #[error("DataChannel not ready")]
    RTCDataChannelNotReady,

    #[error("DataChannel state not open")]
    RTCDataChannelStateNotOpen,

    #[cfg(not(feature = "wasm"))]
    #[error("RTC peer_connection add ice candidate error")]
    RTCPeerConnectionAddIceCandidateError(#[source] webrtc::Error),

    #[cfg(feature = "wasm")]
    #[error("RTC peer_connection add ice candidate error")]
    RTCPeerConnectionAddIceCandidateError(String),

    #[cfg(not(feature = "wasm"))]
    #[error("RTC peer_connection set local description failed")]
    RTCPeerConnectionSetLocalDescFailed(#[source] webrtc::Error),

    #[cfg(feature = "wasm")]
    #[error("RTC peer_connection set local description failed")]
    RTCPeerConnectionSetLocalDescFailed(String),

    #[cfg(not(feature = "wasm"))]
    #[error("RTC peer_connection set remote description failed")]
    RTCPeerConnectionSetRemoteDescFailed(#[source] webrtc::Error),

    #[cfg(feature = "wasm")]
    #[error("RTC peer_connection set remote description failed")]
    RTCPeerConnectionSetRemoteDescFailed(String),

    #[cfg(not(feature = "wasm"))]
    #[error("RTC peer_connection failed to close it")]
    RTCPeerConnectionCloseFailed(#[source] webrtc::Error),

    #[error("RTC unsupport sdp type")]
    RTCSdpTypeNotMatch,

    #[error("Transport not Found")]
    TransportNotFound,

    #[error("Invalid Transport Id")]
    InvalidTransportUuid,

    #[error("Unexpected encrypted data")]
    UnexpectedEncryptedData,

    #[error("Failed to decrypt data")]
    DecryptionError,

    #[error("Current node is not the next hop of message")]
    InvalidNextHop,

    #[error("Adjacent elements in path cannot be equal")]
    InvalidRelayPath,

    #[error("The destination of report message should always be the first element of path")]
    InvalidRelayDestination,

    #[error("Cannot infer next hop")]
    CannotInferNextHop,

    #[error("Cannot get next hop when sending message")]
    NoNextHop,

    #[error("To generate REPORT, you should provide SEND")]
    ReportNeedSend,

    #[error("Only SEND message can reset destination")]
    ResetDestinationNeedSend,

    #[cfg(feature = "wasm")]
    #[error("IndexedDB error, {0}")]
    IDBError(rexie::Error),

    #[error("Invalid capacity value")]
    InvalidCapacity,

    #[cfg(feature = "default")]
    #[error("Sled error, {0}")]
    SledError(sled::Error),

    #[error("entry not found")]
    EntryNotFound,

    #[error("Redis Miss")]
    RedisMiss,

    #[error("Redis Invalid Kind")]
    RedisInvalidKind,

    #[cfg(feature = "default")]
    #[error("Redis Error")]
    RedisError(#[from] redis::RedisError),
}

pub type Result<T> = std::result::Result<T, Error>;
