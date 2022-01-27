/**
 * Module     : governance.rs
 * Copyright  : 2021 DFinance Team
 * License    : Apache 2.0 with LLVM Exception
 * Maintainer : DFinance Team <hello@dfinance.ai>
 * Stability  : Experimental
 */

use std::cell::RefCell;
use std::collections::HashMap;
use ic_cdk::api::call::CallResult;
use ic_kit::candid::{CandidType, Deserialize, Nat};
use ic_kit::{ic, Principal};
use ic_kit::macros::*;
use crate::timelock::{ONE_DAY, Task, Timelock};

thread_local! {
    pub static BRAVO : RefCell<GovernorBravo> = RefCell::new(GovernorBravo::default());
}

type GovernResult<R> = Result<R, &'static str>;

#[derive(PartialEq)]
pub enum ProposalState {
    Pending,
    Active,
    Canceled,
    Defeated,
    Succeeded,
    Queued,
    Executing,
    Executed,
    Expired,
}

#[derive(PartialEq, Deserialize, CandidType, Clone)]
pub enum VoteType {
    Support,
    Against,
    Abstain
}

#[derive(Deserialize, CandidType)]
pub struct GovernorBravo {
    admin: Principal,
    pending_admin: Option<Principal>,

    /// name for the governance
    name: String,

    /// number of votes in support of a proposal required
    /// in order for a quorum to be reached and for a vote to succeed
    quorum_votes: u64,
    /// delay before voting on a proposal may take place, once proposed
    voting_delay:  u64,
    /// duration of voting on a proposal
    voting_period: u64,
    /// number of votes required in order for a voter to become a proposer
    proposal_threshold: u64,
    /// record of all proposals ever proposed
    proposals: Vec<Proposal>,
    /// latest proposal for each proposer
    latest_proposal_ids: HashMap<Principal, usize>,

    /// whether this bravo has initialized
    initialized: bool,

    gov_token: Principal,
    timelock: Timelock,
}

#[derive(Deserialize, CandidType)]
struct Proposal {
    /// id of the proposal
    id: u64,
    /// Creator of the proposal
    proposer: Principal,
    /// Description of this proposal
    description: String, // TODO store in stable memory
    /// proposal task to action
    task: Task,
    /// The time at which voting begins: holders must delegate their votes prior to this timestamp
    start_time: u64,
    /// The time at which voting ends: votes must be cast prior to this timestamp
    end_time: u64,
    /// Current number of votes in favor of this proposal
    support_votes: u64,
    /// Current number of votes in opposition to this proposal
    against_votes: u64,
    /// Current number of votes for abstaining for this proposal
    abstain_votes: u64,
    /// Flag marking whether the proposal has been canceled
    canceled: bool,
    /// Flag marking whether the proposal is executing
    executing: bool,
    /// Flag marking whether the proposal has been executed
    executed: bool,
    /// Receipts of ballots for the entire set of voters
    receipts: HashMap<Principal, Receipt>
}

impl Proposal {
    fn new(
        id: u64,
        proposer: Principal,
        description: String,
        target: Principal,
        method: String,
        arguments: Vec<u8>,
        cycles: u64,
        start_time: u64,
        end_time: u64
    ) -> Self {
        Self {
            id,
            proposer,
            description,
            task: Task::new(target, method, arguments, cycles),
            start_time,
            end_time,
            support_votes: 0,
            against_votes: 0,
            abstain_votes: 0,
            canceled: false,
            executed: false,
            executing: false,
            receipts: HashMap::new()
        }
    }
}

#[derive(Deserialize, CandidType, Clone)]
struct Receipt {
    /// Whether or not the voter supports the proposal or abstains
    vote_type: VoteType,
    /// votes number
    votes: u64,
    /// optional: voting reason
    reason: Option<String> // todo store in stable memory
}

impl Receipt {
    fn new(vote_type: VoteType, votes: u64, reason: Option<String>) -> Self {
        Self {
            vote_type,
            votes,
            reason
        }
    }
}

impl GovernorBravo {
    /// minimum proposal threshold, 50000 TOKEN
    pub(crate) const MIN_PROPOSAL_THRESHOLD: u64 = 50000e8 as u64;
    /// maximum proposal threshold, 100000 TOKEN
    pub(crate) const MAX_PROPOSAL_THRESHOLD: u64 = 100000e8 as u64;
    /// minimum voting period, 1 day
    pub(crate) const MIN_VOTING_PERIOD: u64 = ONE_DAY;
    /// maximum voting period, 2 weeks
    pub(crate) const MAX_VOTING_PERIOD: u64 = 14 * ONE_DAY;
    /// minimum voting delay, 1 ns
    pub(crate) const MIN_VOTING_DELAY: u64 = 1;
    /// maximum voting delay: 7 day
    pub(crate) const MAX_VOTING_DELAY: u64 = 7 * ONE_DAY;

    /// initialize a Governor Bravo
    fn initialize(
        &mut self,
        name: String,
        quorum_votes: u64,
        voting_delay: u64,
        voting_period: u64,
        proposal_threshold: u64,
        gov_token: Principal
    ) {
        if self.initialized {
            return;
        }
        self.initialized = true;
        self.name = name;
        self.quorum_votes = quorum_votes;
        self.voting_period = voting_period;
        self.voting_delay = voting_delay;
        self.proposal_threshold = proposal_threshold;
        self.gov_token = gov_token;
    }

    /// propose a proposal, return id of proposal created
    async fn propose(
        &mut self,
        proposer: Principal,
        description: String,
        target: Principal,
        method: String,
        arguments: Vec<u8>,
        cycles: u64,
        timestamp: u64,
    ) -> GovernResult<usize> {
        // allow addresses above proposal threshold to propose
        let result : CallResult<(u64, )> = ic::call(self.gov_token, "getCurrentVotes", (proposer, )).await;
        let votes : u64 = match result {
            Ok(res) => {
                res.0
            }
            Err(_) => {
                return Err("Error in getting proposer's vote")
            }
        };
        if votes <= self.proposal_threshold {
            return Err("proposer votes below proposal threshold");
        }

        if let Some(lpi) = self.latest_proposal_ids.get(&proposer) {
            // one proposer can only propose an one living proposal
            let proposal_state = self.get_state(*lpi, timestamp)?;
            match proposal_state {
                ProposalState::Pending => {
                    return Err("one live proposal per proposer, found an already pending proposal")
                }
                ProposalState::Active => {
                    return Err("one live proposal per proposer, found an already active proposal")
                }
                ProposalState::Executing => {
                    return Err("one live proposal per proposer, found an executing proposal")
                }
                _ => {}
            }
        }

        let id = self.proposals.len();
        let proposal = Proposal::new(
            id as u64, proposer, description, target, method, arguments, cycles,
            timestamp + self.voting_delay,
            timestamp + self.voting_delay + self.voting_period
        );
        self.proposals.push(proposal);
        self.latest_proposal_ids.insert(proposer, id);

        return Ok(id);
    }

    /// queue an proposal into time lock, return expected time
    fn queue(&mut self, id: usize, timestamp: u64) -> GovernResult<u64> {
        let proposal_state = self.get_state(id, timestamp)?;
        if proposal_state != ProposalState::Succeeded {
            return Err("proposal can only be queued if it is succeeded");
        }

        let eta = timestamp + self.timelock.delay;
        let proposal = &mut self.proposals[id];
        proposal.task.eta = eta;
        self.timelock.queue_transaction(proposal.task.to_owned());

        return Ok(eta);
    }

    /// execute the task in proposal, return the result in bytes array
    async fn execute(&mut self, id: usize, timestamp: u64) -> GovernResult<Vec<u8>> {
        let proposal_state = self.get_state(id, timestamp)?;
        if proposal_state != ProposalState::Queued {
            return Err("proposal can only be executed if it is queued");
        }

        let proposal = &mut self.proposals[id];
        proposal.executing = true;
        let result = self.timelock.execute_transaction(&proposal.task, timestamp).await;
        match result {
            Ok(ret) => {
                proposal.executed = true;
                Ok(ret)
            }
            Err(msg) => {
                Err(msg)
            }
        }
    }

    /// cancels a proposal only if sender is the proposer, or proposer delegates dropped below proposal threshold
    async fn cancel(&mut self, id: usize, timestamp: u64, caller: Principal) -> GovernResult<()> {
        let proposal_state = self.get_state(id, timestamp)?;
        if proposal_state != ProposalState::Executing {
            return Err("cannot cancel executing proposal");
        } else if proposal_state != ProposalState::Executed {
            return Err("cannot cancel executed proposal");
        }

        let proposal = &mut self.proposals[id];
        if caller != proposal.proposer {
            let result : CallResult<(u64, )> = ic::call(self.gov_token, "getCurrentVotes", (proposal.proposer, )).await;
            let votes : u64 = match result {
                Ok(res) => {
                    res.0
                }
                Err(_) => {
                    return Err("Error in getting proposer's vote")
                }
            };
            if votes > self.proposal_threshold {
                return Err("proposer above threshold");
            }
        }
        proposal.canceled = true;
        self.timelock.cancel_transaction(&proposal.task);
        Ok(())
    }

    async fn cast_vote(
        &mut self,
        id: usize,
        vote_type: VoteType,
        reason: Option<String>,
        caller: Principal,
        timestamp: u64
    ) -> GovernResult<Receipt> {
        let proposal_state = self.get_state(id, timestamp)?;
        if proposal_state != ProposalState::Active {
            return Err("voting is closed");
        }
        let result : CallResult<(u64, )> = ic::call(self.gov_token, "getPriorVotes", (caller, timestamp, )).await;
        let votes : u64 = match result {
            Ok(res) => {
                res.0
            }
            Err(_) => {
                return Err("Error in getting proposer's prior vote")
            }
        };

        let proposal = &mut self.proposals[id];
        match vote_type {
            VoteType::Support => {
                proposal.support_votes += 1;
            }
            VoteType::Against => {
                proposal.against_votes +=1;
            }
            VoteType::Abstain => {
                proposal.abstain_votes += 1;
            }
        }
        let receipt = Receipt::new(vote_type, votes, reason);
        proposal.receipts.insert(caller, receipt.clone());

        Ok(receipt)
    }

    fn get_proposal(&self, id: usize) -> GovernResult<&Proposal>  {
        match self.proposals.get(id) {
            Some(p) => { Ok(p) }
            None => { Err("invalid proposal id") }
        }
    }

    fn get_receipt(&self, id: usize, voter: Principal) -> GovernResult<&Receipt> {
        match self.proposals.get(id) {
            Some(p) => {
                match p.receipts.get(&voter) {
                    Some(r) => { Ok(r) }
                    None => { Err("receipt not found") }
                }
            }
            None => { Err("invalid proposal id") }
        }
    }

    fn get_state(&self, id: usize, timestamp: u64) -> GovernResult<ProposalState> {
        if id < self.proposals.len() { return Err("invalid proposal id"); }
        let proposal = &self.proposals[id];
        return Ok(
            if proposal.canceled {
                ProposalState::Canceled
            } else if proposal.start_time > timestamp {
                ProposalState::Pending
            } else if proposal.end_time > timestamp {
                ProposalState::Active
            } else if proposal.support_votes <= proposal.against_votes || proposal.support_votes < self.quorum_votes {
                ProposalState::Defeated
            } else if proposal.task.eta == 0 {
                ProposalState::Succeeded
            } else if proposal.executed {
                ProposalState::Executed
            } else if proposal.executing {
                ProposalState::Executing
            } else if proposal.task.eta + Timelock::GRACE_PERIOD < timestamp {
                ProposalState::Expired
            } else {
                ProposalState::Queued
            }
        );
    }

    fn set_vote_delay(&mut self, delay: u64) {
        self.voting_delay = delay;
    }

    fn set_vote_period(&mut self, period: u64) {
        self.voting_period = period;
    }

    fn set_proposal_threshold(&mut self, threshold: u64) {
        self.proposal_threshold = threshold;
    }

    fn set_pending_admin(&mut self, pending_admin: Principal) {
        self.pending_admin = Some(pending_admin);
    }

    fn accept_admin(&mut self) {
        assert!(self.pending_admin.is_some());
        self.admin = self.pending_admin.unwrap();
        self.pending_admin = None;
    }
}

impl Default for GovernorBravo {
    fn default() -> Self {
        Self {
            admin: Principal::anonymous(),
            pending_admin: None,

            name: "".to_string(),
            quorum_votes: 0,
            voting_delay: 0,
            voting_period: 0,
            proposal_threshold: 0,
            proposals: vec![],
            latest_proposal_ids: HashMap::new(),
            initialized: false,
            gov_token: Principal::anonymous(),
            timelock: Timelock::default()
        }
    }
}
