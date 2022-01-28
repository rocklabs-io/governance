/**
 * Module     : lib.rs
 * Copyright  : 2021 Rocklabs
 * License    : Apache 2.0 with LLVM Exception
 * Maintainer : Rocklabs <hello@rocklabs.io>
 * Stability  : Experimental
 */

use std::cell::RefCell;
use std::collections::HashMap;
use ic_kit::candid::{export_service, candid_method};
use ic_kit::{ic, Principal};
use ic_kit::ic::{stable_restore, stable_store};
use ic_kit::macros::*;
use crate::governance::{GovernorBravo, GovernorBravoInfo, Proposal, ProposalDigest, ProposalState, Receipt, VoteType};
use crate::timelock::Task;

mod timelock;
mod governance;

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

#[init(guard = "is_admin")]
#[candid_method(init)]
fn initialize(
    name: String,
    quorum_votes: u64,
    voting_delay: u64,
    voting_period: u64,
    proposal_threshold: u64,
    gov_token: Principal,
) {
    // assert!(voting_delay >= GovernorBravo::MIN_VOTING_DELAY && voting_delay <= GovernorBravo::MAX_VOTING_DELAY);
    // assert!(voting_period >= GovernorBravo::MIN_VOTING_PERIOD && voting_period <= GovernorBravo::MAX_VOTING_PERIOD);
    // assert!(proposal_threshold >= GovernorBravo::MIN_PROPOSAL_THRESHOLD && proposal_threshold <= GovernorBravo::MAX_PROPOSAL_THRESHOLD);
    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.initialize(
            name,
            quorum_votes,
            voting_delay,
            voting_period,
            proposal_threshold,
            gov_token,
        );
    })
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
fn get_proposal(id: usize) -> Response<(Proposal, ProposalState)> {
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
        let task = bravo.get_proposal(id)?.task.to_owned();
        Ok(task)
    })
}

#[query(name = "getReceipt")]
#[candid_method(query, rename = "getReceipt")]
fn get_receipt(id: usize, voter: Principal) -> Response<Receipt> {
    BRAVO.with(|bravo| {
        let bravo = bravo.borrow();
        let receipt = bravo.get_receipt(id, voter)?.to_owned();
        Ok(receipt)
    })
}

#[query(name = "getReceipts")]
#[candid_method(query, rename = "getReceipts")]
fn get_receipts(id: usize) -> Response<HashMap<Principal, Receipt>> {
    BRAVO.with(|bravo| {
        let bravo = bravo.borrow();
        let receipts = bravo.get_proposal(id)?.receipts.to_owned();
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
    BRAVO.with(|bravo| async {
        let mut bravo = bravo.borrow_mut();
        let id = bravo.propose(
            ic::caller(),
            title,
            description,
            target,
            method,
            arguments,
            cycles,
            ic::time(),
        ).await?;
        Ok(id)
    }).await
}

#[update(name = "queue")]
#[candid_method(update, rename = "queue")]
fn queue(id: usize) -> Response<u64> {
    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        let eta = bravo.queue(id, ic::time())?;
        Ok(eta)
    })
}

#[update(name = "cancel")]
#[candid_method(update, rename = "cancel")]
async fn cancel(id: usize) -> Response<()> {
    BRAVO.with(|bravo| async {
        let mut bravo = bravo.borrow_mut();
        bravo.cancel(id, ic::time(), ic::caller()).await?;
        Ok(())
    }).await
}

#[update(name = "execute")]
#[candid_method(update, rename = "execute")]
async fn execute(id: usize) -> Response<Vec<u8>> {
    BRAVO.with(|bravo| async {
        let mut bravo = bravo.borrow_mut();
        let res = bravo.execute(id, ic::time()).await?;
        Ok(res)
    }).await
}

#[update(name = "castVote")]
#[candid_method(update, rename = "castVote")]
async fn cast_vote(id: usize, vote_type: VoteType, reason: Option<String>) -> Response<Receipt> {
    let x1 = BRAVO.with(|x| x);
    BRAVO.with(|bravo| async {
        let mut bravo = bravo.borrow_mut();
        let receipt = bravo.cast_vote(
            id,
            vote_type,
            reason,
            ic::caller(),
            ic::time(),
        ).await?;
        Ok(receipt)
    }).await
}

#[update(name = "setPendingAdmin", guard = "is_admin")]
#[candid_method(update, rename = "setPendingAdmin")]
async fn set_pending_admin(pending_admin: Principal) -> Response<()> {
    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.set_pending_admin(pending_admin);
        Ok(())
    })
}

#[update(name = "acceptAdmin")]
#[candid_method(update, rename = "setAdmin")]
fn accept_admin() -> Response<()> {
    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        if bravo.pending_admin != Some(ic::caller()) {
            Err("Unauthorized")
        } else {
            bravo.accept_admin();
            Ok(())
        }
    })
}

#[update(name = "setQuorumVotes", guard = "is_admin")]
#[candid_method(update, rename = "setQuorumVotes")]
async fn set_quorum_votes(quorum: u64) -> Response<()> {
    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.set_quorum_votes(quorum);
        Ok(())
    })
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
        Ok(())
    })
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
        Ok(())
    })
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
        Ok(())
    })
}

#[pre_upgrade]
fn pre_upgrade() {
    BRAVO.with(|b| {
        let bravo = b.borrow();
        stable_store((bravo.to_owned(), )).unwrap();
    });
}

#[post_upgrade]
fn post_upgrade() {
    let (bravo, ): (GovernorBravo, ) = stable_restore().unwrap();
    BRAVO.with(|b| {
        let mut b_mut = b.borrow_mut();
        *b_mut = bravo;
    });
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