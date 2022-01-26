/**
 * Module     : governance.rs
 * Copyright  : 2021 DFinance Team
 * License    : Apache 2.0 with LLVM Exception
 * Maintainer : DFinance Team <hello@dfinance.ai>
 * Stability  : Experimental
 */

use std::cell::RefCell;
use std::collections::HashMap;
use ic_kit::candid::{CandidType, Deserialize};
use ic_kit::{ic, Principal};
use ic_kit::macros::*;
use crate::timelock::{ONE_DAY, Task, Timelock};

thread_local! {
    pub static BRAVO : RefCell<GovernorBravo> = RefCell::new(GovernorBravo::default());
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
    latest_proposal_ids: HashMap<Principal, u64>,

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
    for_votes: u64,
    /// Current number of votes in opposition to this proposal
    against_votes: u64,
    /// Current number of votes for abstaining for this proposal
    abstain_votes: u64,
    /// Flag marking whether the proposal has been canceled
    canceled: bool,
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
            for_votes: 0,
            against_votes: 0,
            abstain_votes: 0,
            canceled: false,
            executed: false,
            receipts: HashMap::new()
        }
    }
}

#[derive(Deserialize, CandidType)]
struct Receipt {
    /// Whether or not the voter supports the proposal or abstains
    /// 0: agree, 1: against, 2: abstain
    support: u8,
    votes: u64
}

impl Receipt {
    fn new(support: u8, votes: u64) -> Self {
        Self {
            support,
            votes
        }
    }
}

impl GovernorBravo {
    /// minimum proposal threshold, 50000 TOKEN
    const MIN_PROPOSAL_THRESHOLD: u64 = 50000e8 as u64;
    /// maximum proposal threshold, 100000 TOKEN
    const MAX_PROPOSAL_THRESHOLD: u64 = 100000e8 as u64;
    /// minimum voting period, 1 day
    const MIN_VOTING_PERIOD: u64 = ONE_DAY;
    /// maximum voting period, 2 weeks
    const MAX_VOTING_PERIOD: u64 = 14 * ONE_DAY;
    /// minimum voting delay, 1 ns
    const MIN_VOTING_DELAY: u64 = 1;
    /// maximum voting delay: 7 day
    const MAX_VOTING_DELAY: u64 = 7 * ONE_DAY;

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

    fn propose() { todo!() }

    fn queue() { todo!() }

    fn execute() { todo!() }

    fn cancel() { todo!() }

    fn get_task() { todo!() }

    fn get_receipt() { todo!() }

    fn state() { todo!() }

    fn cast_vote() { todo!() }

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
