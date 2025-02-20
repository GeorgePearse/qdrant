use crate::consensus;
use api::grpc::qdrant::{
    raft_server::Raft, AddPeerToKnownMessage, AllPeers, Peer, PeerId,
    RaftMessage as RaftMessageBytes, Uri as UriStr,
};
use itertools::Itertools;
use raft::eraftpb::{ConfChangeType, ConfChangeV2, Message as RaftMessage};
use std::sync::{mpsc::SyncSender, Arc, Mutex};
use storage::content_manager::{consensus_ops::ConsensusOperations, toc::TableOfContent};
use tonic::transport::Uri;
use tonic::{async_trait, Request, Response, Status};

pub struct RaftService {
    message_sender: Mutex<SyncSender<consensus::Message>>,
    toc: Arc<TableOfContent>,
}

impl RaftService {
    pub fn new(sender: SyncSender<consensus::Message>, toc: Arc<TableOfContent>) -> Self {
        Self {
            message_sender: Mutex::new(sender),
            toc,
        }
    }
}

#[async_trait]
impl Raft for RaftService {
    async fn send(&self, mut request: Request<RaftMessageBytes>) -> Result<Response<()>, Status> {
        let message = <RaftMessage as prost::Message>::decode(&request.get_mut().message[..])
            .map_err(|err| {
                Status::invalid_argument(format!("Failed to parse raft message: {err}"))
            })?;
        self.message_sender
            .lock()
            .map_err(|_| Status::internal("Can't capture the Raft message sender lock"))?
            .send(consensus::Message::FromPeer(Box::new(message)))
            .map_err(|_| Status::internal("Can't send Raft message over channel"))?;
        Ok(Response::new(()))
    }

    async fn who_is(
        &self,
        request: tonic::Request<PeerId>,
    ) -> Result<tonic::Response<UriStr>, tonic::Status> {
        let addresses = self
            .toc
            .peer_address_by_id()
            .map_err(|err| Status::internal(format!("Can't get peer addresses: {err}")))?;
        let uri = addresses
            .get(&request.get_ref().id)
            .ok_or_else(|| Status::internal("Peer not found"))?;
        Ok(Response::new(UriStr {
            uri: uri.to_string(),
        }))
    }

    async fn add_peer_to_known(
        &self,
        request: tonic::Request<AddPeerToKnownMessage>,
    ) -> Result<tonic::Response<AllPeers>, tonic::Status> {
        let peer = request.get_ref();
        let uri_string = if let Some(uri) = &peer.uri {
            uri.clone()
        } else {
            let ip = request
                .remote_addr()
                .ok_or_else(|| {
                    Status::failed_precondition("Remote address unavailable due to the used IO")
                })?
                .ip();
            let port = peer
                .port
                .ok_or_else(|| Status::invalid_argument("URI or port should be supplied"))?;
            format!("http://{ip}:{port}")
        };
        let uri: Uri = uri_string
            .parse()
            .map_err(|err| Status::internal(format!("Failed to parse uri: {err}")))?;
        let peer = request.into_inner();
        // the consensus operation can take up to DEFAULT_META_OP_WAIT
        self.toc
            .propose_consensus_op(ConsensusOperations::AddPeer(peer.id, uri_string), None)
            .await
            .map_err(|err| Status::internal(format!("Failed to add peer: {err}")))?;
        let addresses = self
            .toc
            .peer_address_by_id()
            .map_err(|err| Status::internal(format!("Can't get peer addresses: {err}")))?;
        // Make sure that the new peer is now present in the known addresses
        if !addresses.values().contains(&uri) {
            return Err(Status::internal(format!(
                "Failed to add peer after consensus: {uri}"
            )));
        }
        Ok(Response::new(AllPeers {
            all_peers: addresses
                .into_iter()
                .map(|(id, uri)| Peer {
                    id,
                    uri: uri.to_string(),
                })
                .collect(),
        }))
    }

    async fn add_peer_as_participant(
        &self,
        request: tonic::Request<PeerId>,
    ) -> Result<tonic::Response<()>, tonic::Status> {
        let mut change = ConfChangeV2::default();
        change.set_changes(vec![raft_proto::new_conf_change_single(
            request.get_ref().id,
            ConfChangeType::AddNode,
        )]);
        self.message_sender
            .lock()
            .map_err(|_| Status::internal("Can't capture the Raft message sender lock"))?
            .send(consensus::Message::ConfChange(change))
            .map_err(|_| Status::internal("Can't send Raft message over channel"))?;
        Ok(Response::new(()))
    }
}
