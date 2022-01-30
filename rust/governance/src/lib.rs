/**
 * Module     : lib.rs
 * Copyright  : 2021 Rocklabs
 * License    : Apache 2.0 with LLVM Exception
 * Maintainer : Rocklabs <hello@rocklabs.io>
 * Stability  : Experimental
 */

use std::cell::RefCell;
use cap_sdk::{CapEnv, handshake, IndefiniteEventBuilder, insert};
use cap_sdk::DetailValue::U64;
use ic_cdk::api::call::CallResult;
use ic_kit::candid::{export_service, candid_method, Nat};
use ic_kit::{ic, Principal};
use ic_kit::ic::{stable_restore, stable_store};
use ic_kit::macros::*;
use crate::cap::{AcceptAdminEvent, CancelEvent, ExecuteEvent, GovEvent, ProposeEvent, QueueEvent, SetPendingAdminEvent, VoteEvent};
use crate::governance::{GovernorBravo, GovernorBravoInfo, ProposalDigest, ProposalInfo, ProposalState, Receipt, ReceiptDigest, ReceiptInfo, VoteType};
use crate::timelock::{Task};

mod timelock;
mod governance;
mod stable;
mod cap;

thread_local! {
    static BRAVO : RefCell<GovernorBravo> = RefCell::new(GovernorBravo::default());
}

type Response<R> = Result<R, &'static str>;

fn is_admin() -> Result<(), String> {
    BRAVO.with(|bravo| {
        let bravo = bravo.borrow();
        if bravo.admin == ic::caller() {
            Ok(())
        } else {
            Err("Unauthorized".to_string())
        }
    })
}

#[init]
#[candid_method(init)]
fn initialize(
    admin: Principal,
    name: String,
    quorum_votes: u64,
    voting_delay: u64,
    voting_period: u64,
    proposal_threshold: u64,
    timelock_delay: u64,
    gov_token: Principal,
    cap: Principal,
) {
    // assert!(voting_delay >= GovernorBravo::MIN_VOTING_DELAY && voting_delay <= GovernorBravo::MAX_VOTING_DELAY);
    // assert!(voting_period >= GovernorBravo::MIN_VOTING_PERIOD && voting_period <= GovernorBravo::MAX_VOTING_PERIOD);
    // assert!(proposal_threshold >= GovernorBravo::MIN_PROPOSAL_THRESHOLD && proposal_threshold <= GovernorBravo::MAX_PROPOSAL_THRESHOLD);
    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.initialize(
            admin,
            name,
            quorum_votes,
            voting_delay,
            voting_period,
            proposal_threshold,
            timelock_delay,
            gov_token,
        );
    });
    handshake(1_000_000_000_000, Some(cap));
}

#[query(name = "getGovernorBravoInfo")]
#[candid_method(query, rename = "getGovernorBravoInfo")]
fn get_governor_bravo_info() -> Response<GovernorBravoInfo> {
    BRAVO.with(|bravo| {
        let bravo = bravo.borrow();
        Ok(bravo.digest())
    })
}

#[query(name = "getProposal")]
#[candid_method(query, rename = "getProposal")]
fn get_proposal(id: usize) -> Response<(ProposalInfo, ProposalState)> {
    BRAVO.with(|bravo| {
        let bravo = bravo.borrow();
        let proposal = bravo.get_proposal(id)?;
        let state = bravo.get_state(id, ic::time())?;
        Ok((proposal.to_owned(), state))
    })
}

#[query(name = "getProposalState")]
#[candid_method(query, rename = "getProposalState")]
fn get_proposal_state(id: usize) -> Response<ProposalState> {
    BRAVO.with(|bravo| {
        let bravo = bravo.borrow();
        let state = bravo.get_state(id, ic::time())?;
        Ok(state)
    })
}

#[query(name = "getProposals")]
#[candid_method(query, rename = "getProposals")]
fn get_proposals(page: usize, num: usize) -> Response<Vec<(ProposalDigest, ProposalState)>> {
    BRAVO.with(|bravo| {
        let bravo = bravo.borrow();
        let res = bravo.get_proposal_pages(page, num, ic::time())?;
        Ok(res)
    })
}

#[query(name = "getTask")]
#[candid_method(query, rename = "getTask")]
fn get_task(id: usize) -> Response<Task> {
    BRAVO.with(|bravo| {
        let bravo = bravo.borrow();
        let task = bravo.get_task(id)?;
        Ok(task)
    })
}

#[query(name = "getReceipt")]
#[candid_method(query, rename = "getReceipt")]
fn get_receipt(id: usize, voter: Principal) -> Response<ReceiptInfo> {
    BRAVO.with(|bravo| {
        let bravo = bravo.borrow();
        let receipt = bravo.get_receipt(id, voter)?.to_owned();
        Ok(receipt)
    })
}

#[query(name = "getReceipts")]
#[candid_method(query, rename = "getReceipts")]
fn get_receipts(id: usize, page: usize, num: usize) -> Response<Vec<(Principal, ReceiptDigest)>> {
    BRAVO.with(|bravo| {
        let bravo = bravo.borrow();
        let receipts = bravo.get_receipt_pages(id, page, num)?;
        Ok(receipts)
    })
}

#[update(name = "propose")]
#[candid_method(update, rename = "propose")]
async fn propose(
    title: String,
    description: String,
    target: Principal,
    method: String,
    arguments: Vec<u8>,
    cycles: u64,
) -> Response<usize> {
    let caller = ic::caller();
    let gov_token = BRAVO.with(|bravo| {
        let bravo = bravo.borrow();
        bravo.gov_token
    });
    let result : CallResult<(Nat, )> = ic::call(gov_token, "getCurrentVotes", (caller, )).await;
    let proposer_votes : Nat = match result {
        Ok(res) => {
            res.0
        }
        Err(_) => {
            return Err("Error in getting proposer's vote")
        }
    };
    let id = BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.propose(
            caller,
            proposer_votes,
            title.clone(),
            description.clone(),
            target,
            method.clone(),
            arguments.clone(),
            cycles,
            ic::time(),
        )
    })?;
    insert(ProposeEvent::new(
        caller,
        id as u64,
        title,
        description,
        target,
        method,
        arguments,
        cycles
    ).to_indefinite_event()).await;

    Ok(id)
}

#[update(name = "queue")]
#[candid_method(update, rename = "queue")]
async fn queue(id: usize) -> Response<u64> {
    let caller = ic::caller();
    let eta = BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.queue(id, ic::time())

    })?;
    insert(QueueEvent::new(caller, id as u64, eta).to_indefinite_event()).await;
    Ok(eta)
}

#[update(name = "cancel")]
#[candid_method(update, rename = "cancel")]
async fn cancel(id: usize) -> Response<()> {
    let caller = ic::caller();
    let proposer = BRAVO.with(|bravo| {
        let bravo = bravo.borrow();
        match bravo.get_proposal(id) {
            Ok(p) => { Ok(p.to_owned()) }
            Err(msg) => { Err(msg) }
        }
    })?;
    let gov_token = BRAVO.with(|bravo| {
        let bravo = bravo.borrow();
        bravo.gov_token
    });
    let result : CallResult<(Nat, )> = ic::call(gov_token, "getCurrentVotes", (proposer, )).await;
    let proposer_votes : Nat = match result {
        Ok(res) => {
            res.0
        }
        Err(_) => {
            return Err("Error in getting proposer's vote")
        }
    };
    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.cancel(id, ic::time(), ic::caller(), proposer_votes)
    })?;
    insert(CancelEvent::new(caller, id as u64).to_indefinite_event()).await;
    Ok(())
}

#[update(name = "execute")]
#[candid_method(update, rename = "execute")]
async fn execute(id: usize) -> Response<Vec<u8>> {
    let caller = ic::caller();
    let timestamp = ic::time();
    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.pre_execute(id, timestamp)
    })?;

    let task = BRAVO.with(|bravo| {
        let bravo = bravo.borrow();
        bravo.get_task(id)
    })?;
    let result = ic::call_raw(
        task.target,
        task.method.to_owned(),
        task.arguments.to_owned(),
        task.cycles,
    ).await;

    let ret = BRAVO.with(move |bravo| {
        let mut bravo = bravo.borrow_mut();
        match result {
            Ok(ret) => {
                bravo.post_execute(id, true, timestamp)?;
                Ok(ret)
            }
            Err(_) => {
                bravo.post_execute(id, false, timestamp)?;
                Err("Execute error")
            }
        }
    })?;
    insert(ExecuteEvent::new(caller, id as u64, ret.clone()).to_indefinite_event()).await;
    Ok(ret)
}

#[update(name = "castVote")]
#[candid_method(update, rename = "castVote")]
async fn cast_vote(id: usize, vote_type: VoteType, reason: Option<String>) -> Response<Receipt> {
    let caller = ic::caller();
    let timestamp = ic::time();
    let gov_token = BRAVO.with(|bravo| {
        let bravo = bravo.borrow();
        bravo.gov_token
    });
    let result : CallResult<(Nat, )> = ic::call(gov_token, "getPriorVotes", (caller, Nat::from(timestamp), )).await;
    let votes : Nat = match result {
        Ok(res) => {
            res.0
        }
        Err(_) => {
            return Err("Error in getting proposer's prior vote");
        }
    };
    let receipt = BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.cast_vote(
            id,
            vote_type.clone(),
            votes.clone(),
            reason,
            caller,
            timestamp,
        )
    })?;
    insert(VoteEvent::new(caller, id as u64, votes, vote_type).to_indefinite_event()).await;
    Ok(receipt)
}

#[update(name = "setPendingAdmin", guard = "is_admin")]
#[candid_method(update, rename = "setPendingAdmin")]
async fn set_pending_admin(pending_admin: Principal) -> Response<()> {
    let caller = ic::caller();
    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.set_pending_admin(pending_admin);
    });
    insert(SetPendingAdminEvent::new(caller, pending_admin).to_indefinite_event()).await;
    Ok(())
}

#[update(name = "acceptAdmin")]
#[candid_method(update, rename = "setAdmin")]
async fn accept_admin() -> Response<()> {
    let caller = ic::caller();
    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        if bravo.pending_admin != Some(caller) {
            Err("Unauthorized")
        } else {
            bravo.accept_admin();
            Ok(())
        }
    })?;
    insert(AcceptAdminEvent::new(caller).to_indefinite_event()).await;
    Ok(())
}

#[update(name = "setQuorumVotes", guard = "is_admin")]
#[candid_method(update, rename = "setQuorumVotes")]
async fn set_quorum_votes(quorum: u64) -> Response<()> {
    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.set_quorum_votes(quorum);
    });
    insert(IndefiniteEventBuilder::new()
        .caller(ic::caller())
        .operation("setQuorumVotes")
        .details(vec![("quorumVotes".to_string(), U64(quorum))])
        .build()
        .unwrap()
    ).await;
    Ok(())
}

#[update(name = "setVotePeriod", guard = "is_admin")]
#[candid_method(update, rename = "setVotePeriod")]
async fn set_vote_period(period: u64) -> Response<()> {
    // if period < GovernorBravo::MIN_VOTING_PERIOD {
    //     return Err("Invalid vote period: too small");
    // }
    // if period > GovernorBravo::MAX_VOTING_PERIOD {
    //     return Err("Invalid vote period: too large");
    // }
    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.set_vote_period(period);
    });
    insert(IndefiniteEventBuilder::new()
        .caller(ic::caller())
        .operation("setVotePeriod")
        .details(vec![("votePeriod".to_string(), U64(period))])
        .build()
        .unwrap()
    ).await;
    Ok(())
}

#[update(name = "setVoteDelay", guard = "is_admin")]
#[candid_method(update, rename = "setVoteDelay")]
async fn set_vote_delay(delay: u64) -> Response<()> {
    // if delay < GovernorBravo::MIN_VOTING_DELAY {
    //     return Err("Invalid vote delay: too small");
    // }
    // if delay > GovernorBravo::MAX_VOTING_DELAY {
    //     return Err("Invalid vote delay: too large");
    // }
    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.set_vote_delay(delay);
    });
    insert(IndefiniteEventBuilder::new()
        .caller(ic::caller())
        .operation("setVoteDelay")
        .details(vec![("voteDelay".to_string(), U64(delay))])
        .build()
        .unwrap()
    ).await;
    Ok(())
}

#[update(name = "setProposalThreshold", guard = "is_admin")]
#[candid_method(update, rename = "setProposalThreshold")]
async fn set_proposal_threshold(threshold: u64) -> Response<()> {
    // if threshold < GovernorBravo::MIN_PROPOSAL_THRESHOLD {
    //     return Err("Invalid proposal threshold: too small");
    // }
    // if threshold > GovernorBravo::MAX_PROPOSAL_THRESHOLD {
    //     return Err("Invalid proposal threshold: too large");
    // }
    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.set_proposal_threshold(threshold);
    });
    insert(IndefiniteEventBuilder::new()
        .caller(ic::caller())
        .operation("setProposalThreshold")
        .details(vec![("proposalThreshold".to_string(), U64(threshold))])
        .build()
        .unwrap()
    ).await;
    Ok(())
}

#[update(name = "setTimelockDelay", guard = "is_admin")]
#[candid_method(update, rename = "setTimelockDelay")]
async fn set_timelock_delay(delay: u64) -> Response<()> {
    // if delay < Timelock::MIN_DELAY {
    //     return Err("Invalid timelock delay: too small");
    // }
    // if delay > Timelock::MAX_DELAY {
    //     return Err("Invalid timelock delay: too large");
    // }
    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.timelock.set_delay(delay);
    });
    insert(IndefiniteEventBuilder::new()
        .caller(ic::caller())
        .operation("setTimelockDelay")
        .details(vec![("timelockDelay".to_string(), U64(delay))])
        .build()
        .unwrap()
    ).await;
    Ok(())
}

#[pre_upgrade]
fn pre_upgrade() {
    BRAVO.with(|b| {
        let bravo = b.borrow();
        stable_store((bravo.to_owned(), CapEnv::to_archive(), )).unwrap();
    });
}

#[post_upgrade]
fn post_upgrade() {
    let (bravo, cap_env, ): (GovernorBravo, CapEnv, ) = stable_restore().unwrap();
    BRAVO.with(|b| {
        let mut b_mut = b.borrow_mut();
        *b_mut = bravo;
    });
    CapEnv::load_from_archive(cap_env);
}

// needed to export candid on save
#[query(name = "__get_candid_interface_tmp_hack")]
fn export_candid() -> String {
    export_service!();
    __export_service()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_candid() {
        use std::env;
        use std::fs::write;
        use std::path::PathBuf;

        let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
        write(dir.join("governance.did"), export_candid()).expect("Write failed.");
    }
}