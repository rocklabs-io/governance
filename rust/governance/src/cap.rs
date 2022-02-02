use cap_sdk::{DetailsBuilder, IndefiniteEvent, IndefiniteEventBuilder};
use cap_sdk::DetailValue::Slice;
use ic_kit::candid::Nat;
use ic_kit::Principal;
use crate::VoteType;

pub trait GovEvent {
    fn to_indefinite_event(&self) -> IndefiniteEvent;
}

pub struct ProposeEvent {
    caller: Principal,
    id: u64,
    title: String,
    description: String,
    target: Principal,
    method: String,
    arguments: Vec<u8>,
    cycles: u64,
}

impl ProposeEvent {
    pub(crate) fn new(
        caller: Principal,
        id: u64,
        title: String,
        description: String,
        target: Principal,
        method: String,
        arguments: Vec<u8>,
        cycles: u64,
    ) -> Self {
        Self {
            caller,
            id,
            title,
            description,
            target,
            method,
            arguments,
            cycles,
        }
    }
}

impl GovEvent for ProposeEvent {
    fn to_indefinite_event(&self) -> IndefiniteEvent {
        IndefiniteEventBuilder::new()
            .caller(self.caller)
            .operation("propose".to_string())
            .details(
                DetailsBuilder::new()
                    .insert("id", self.id)
                    .insert("title", self.title.to_owned())
                    .insert("description", self.description.to_owned())
                    .insert("target", self.target)
                    .insert("method", self.method.to_owned())
                    .insert("arguments", Slice(self.arguments.to_owned()))
                    .insert("cycles", self.cycles)
                    .build()
            )
            .build()
            .unwrap()
    }
}

pub struct QueueEvent {
    caller: Principal,
    proposal_id: u64,
    eta: u64,
}

impl QueueEvent {
    pub(crate) fn new(caller: Principal, id: u64, eta: u64) -> Self {
        Self {
            caller,
            proposal_id: id,
            eta
        }
    }
}

impl GovEvent for  QueueEvent {
    fn to_indefinite_event(&self) -> IndefiniteEvent {
        IndefiniteEventBuilder::new()
            .caller(self.caller)
            .operation("queue".to_string())
            .details(
                DetailsBuilder::new()
                    .insert("proposalId", self.proposal_id)
                    .insert("eta", self.eta)
                    .build()
            )
            .build()
            .unwrap()
    }
}

pub struct CancelEvent {
    caller: Principal,
    proposal_id: u64,
}

impl CancelEvent {
    pub(crate) fn new(caller: Principal, id: u64) -> Self {
        Self {
            caller,
            proposal_id: id
        }
    }
}

impl GovEvent for CancelEvent {
    fn to_indefinite_event(&self) -> IndefiniteEvent {
        IndefiniteEventBuilder::new()
            .caller(self.caller)
            .operation("cancel".to_string())
            .details(
                DetailsBuilder::new()
                    .insert("proposalId", self.proposal_id)
                    .build()
            )
            .build()
            .unwrap()
    }
}

pub struct ExecuteEvent {
    caller: Principal,
    proposal_id: u64,
    result: Vec<u8>,
}

impl ExecuteEvent {
    pub(crate) fn new(caller: Principal, id: u64, result: Vec<u8>) -> Self {
        Self {
            caller,
            proposal_id: id,
            result
        }
    }
}

impl GovEvent for ExecuteEvent {
    fn to_indefinite_event(&self) -> IndefiniteEvent {
        IndefiniteEventBuilder::new()
            .caller(self.caller)
            .operation("execute".to_string())
            .details(
                DetailsBuilder::new()
                    .insert("proposalId", self.proposal_id)
                    .build()
            )
            .build()
            .unwrap()
    }
}

pub struct VoteEvent {
    caller: Principal,
    proposal_id: u64,
    votes: Nat,
    vote_type: VoteType,
}

impl VoteEvent {
    pub(crate) fn new(caller: Principal, proposal_id: u64, votes: Nat, vote_type: VoteType) -> Self {
        Self {
            caller,
            proposal_id,
            votes,
            vote_type
        }
    }
}

impl GovEvent for VoteEvent {
    fn to_indefinite_event(&self) -> IndefiniteEvent {
        let vote_type = match self.vote_type {
            VoteType::Support => { "support" }
            VoteType::Against => { "against" }
            VoteType::Abstain => { "abstain" }
        };
        IndefiniteEventBuilder::new()
            .caller(self.caller)
            .operation("vote")
            .details(
                DetailsBuilder::new()
                    .insert("proposalId", self.proposal_id)
                    .insert("votes", self.votes.clone())
                    .insert("voteType", vote_type.to_string())
                    .build()
            )
            .build()
            .unwrap()
    }
}

pub struct SetPendingAdminEvent {
    caller: Principal,
    pending_admin: Principal,
}

impl SetPendingAdminEvent {
    pub(crate) fn new(caller: Principal, pending_admin: Principal) -> Self {
        Self {
            caller,
            pending_admin
        }
    }
}

impl GovEvent for SetPendingAdminEvent {
    fn to_indefinite_event(&self) -> IndefiniteEvent {
        IndefiniteEventBuilder::new()
            .caller(self.caller)
            .operation("setPendingAdmin".to_string())
            .details(
                DetailsBuilder::new()
                    .insert("pendingAdmin", self.pending_admin)
                    .build()
            )
            .build()
            .unwrap()
    }
}

pub struct AcceptAdminEvent {
    caller: Principal,
}

impl AcceptAdminEvent {
    pub(crate) fn new(caller: Principal) -> Self {
        Self {
            caller
        }
    }
}

impl GovEvent for AcceptAdminEvent {
    fn to_indefinite_event(&self) -> IndefiniteEvent {
        IndefiniteEventBuilder::new()
            .caller(self.caller)
            .operation("acceptAdmin".to_string())
            .build()
            .unwrap()
    }
}