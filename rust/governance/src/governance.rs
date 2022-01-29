/**
 * Module     : governance.rs
 * Copyright  : 2021 Rocklabs
 * License    : Apache 2.0 with LLVM Exception
 * Maintainer : Rocklabs <hello@rocklabs.io>
 * Stability  : Experimental
 */

use std::collections::HashMap;
use ic_kit::candid::{CandidType, Deserialize};
use ic_kit::{Principal};
use crate::stable::{Memory, Position, StableMemory};
use crate::timelock::{ONE_DAY, Task, Timelock};

type GovernResult<R> = Result<R, &'static str>;

#[derive(CandidType, PartialEq)]
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

#[derive(PartialEq, Deserialize, CandidType, Clone, Debug)]
pub enum VoteType {
    Support,
    Against,
    Abstain,
}

#[derive(Deserialize, CandidType, Clone)]
pub struct GovernorBravo {
    pub(crate) admin: Principal,
    pub(crate) pending_admin: Option<Principal>,

    /// name for the governance
    name: String,

    /// number of votes in support of a proposal required
    /// in order for a quorum to be reached and for a vote to succeed
    quorum_votes: u64,
    /// delay before voting on a proposal may take place, once proposed
    voting_delay: u64,
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

    pub(crate) gov_token: Principal,
    pub(crate) timelock: Timelock,
    pub(crate) stable_memory: StableMemory,
}

#[derive(CandidType)]
pub struct GovernorBravoInfo {
    admin: Principal,
    pending_admin: Option<Principal>,
    /// name for the governance
    name: String,
    /// number of votes in support of a proposal required
    /// in order for a quorum to be reached and for a vote to succeed
    quorum_votes: u64,
    /// delay before voting on a proposal may take place, once proposed
    voting_delay: u64,
    /// duration of voting on a proposal
    voting_period: u64,
    /// number of votes required in order for a voter to become a proposer
    proposal_threshold: u64,
    /// number of proposal record ever proposed
    proposals_num: usize,

    gov_token: Principal,
    stable_memory: StableMemory,
}

#[derive(Deserialize, CandidType, Clone)]
pub struct Proposal {
    /// id of the proposal
    id: usize,
    /// Creator of the proposal
    pub(crate) proposer: Principal,
    /// Title of this proposal
    title: String,
    // may limit its length
    /// Description of this proposal
    description: Position,
    /// proposal task to action
    pub(crate) task: Task,
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
    pub(crate) receipts: HashMap<Principal, Receipt>,
}

#[derive(Deserialize, CandidType, Clone)]
pub struct ProposalInfo {
    /// id of the proposal
    id: usize,
    /// Creator of the proposal
    proposer: Principal,
    /// Title of this proposal
    title: String,
    // may limit its length
    /// Description of this proposal
    description: String,
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
}

#[derive(CandidType)]
pub struct ProposalDigest {
    /// id of the proposal
    id: usize,
    /// Creator of the proposal
    proposer: Principal,
    /// Title of this proposal
    title: String,
    // may limit its length
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
    /// Number of voter
    receipt_num: usize,
}

impl Proposal {
    fn new(
        id: usize,
        proposer: Principal,
        title: String,
        description: Position,
        target: Principal,
        method: String,
        arguments: Vec<u8>,
        cycles: u64,
        start_time: u64,
        end_time: u64,
    ) -> Self {
        Self {
            id,
            proposer,
            title,
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
            receipts: HashMap::new(),
        }
    }

    fn to_info(&self, description: String) -> ProposalInfo {
        ProposalInfo {
            id: self.id,
            proposer: self.proposer,
            title: self.title.clone(),
            description,
            task: self.task.clone(),
            start_time: self.start_time,
            end_time: self.end_time,
            support_votes: self.support_votes,
            against_votes: self.against_votes,
            abstain_votes: self.abstain_votes,
            canceled: self.canceled,
            executing: self.executing,
            executed: self.executed,
        }
    }

    fn digest(&self) -> ProposalDigest {
        ProposalDigest {
            id: self.id,
            proposer: self.proposer,
            title: self.title.clone(),
            start_time: self.start_time,
            end_time: self.end_time,
            support_votes: self.support_votes,
            against_votes: self.against_votes,
            abstain_votes: self.abstain_votes,
            receipt_num: self.receipts.len(),
        }
    }
}

#[derive(Deserialize, CandidType, Clone)]
pub struct Receipt {
    /// Whether or not the voter supports the proposal or abstains
    vote_type: VoteType,
    /// votes number
    votes: u64,
    /// optional: voting reason
    reason: Option<Position>,
}

#[derive(Deserialize, CandidType, Clone)]
pub struct ReceiptInfo {
    vote_type: VoteType,
    votes: u64,
    reason: Option<String>,
}

#[derive(Deserialize, CandidType, Clone)]
pub struct ReceiptDigest {
    vote_type: VoteType,
    votes: u64,
}

impl Receipt {
    fn new(vote_type: VoteType, votes: u64, reason: Option<Position>) -> Self {
        Self {
            vote_type,
            votes,
            reason,
        }
    }

    fn digest(&self) -> ReceiptDigest {
        ReceiptDigest {
            votes: self.votes,
            vote_type: self.vote_type.clone(),
        }
    }

    fn to_info(&self, reason: Option<String>) -> ReceiptInfo {
        ReceiptInfo {
            vote_type: self.vote_type.clone(),
            votes: self.votes,
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
    pub fn initialize(
        &mut self,
        name: String,
        quorum_votes: u64,
        voting_delay: u64,
        voting_period: u64,
        proposal_threshold: u64,
        timelock_delay: u64,
        gov_token: Principal,
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
        self.timelock.set_delay(timelock_delay);
    }

    /// propose a proposal, return id of proposal created
    pub fn propose(
        &mut self,
        proposer: Principal,
        proposer_votes: u64,
        title: String,
        description: String,
        target: Principal,
        method: String,
        arguments: Vec<u8>,
        cycles: u64,
        timestamp: u64,
    ) -> GovernResult<usize> {
        // allow addresses above proposal threshold to propose
        if proposer_votes <= self.proposal_threshold {
            return Err("proposer votes below proposal threshold");
        }

        if let Some(lpi) = self.latest_proposal_ids.get(&proposer) {
            // one proposer can only propose an one living proposal
            let proposal_state = self.get_state(*lpi, timestamp)?;
            match proposal_state {
                ProposalState::Pending => {
                    return Err("one live proposal per proposer, found an already pending proposal");
                }
                ProposalState::Active => {
                    return Err("one live proposal per proposer, found an already active proposal");
                }
                ProposalState::Executing => {
                    return Err("one live proposal per proposer, found an executing proposal");
                }
                _ => {}
            }
        }

        let id = self.proposals.len();
        let buf = description.into_bytes();
        let offset = self.stable_memory.offset;
        let len = self.stable_memory.write(buf.as_slice()).map_err(|_| "Stable memory error")?;
        let pos = Position {
            offset,
            len
        };
        let proposal = Proposal::new(
            id, proposer, title, pos, target, method, arguments, cycles,
            timestamp + self.voting_delay,
            timestamp + self.voting_delay + self.voting_period,
        );
        self.proposals.push(proposal);
        self.latest_proposal_ids.insert(proposer, id);

        return Ok(id);
    }

    /// queue an proposal into time lock, return expected time
    pub(crate) fn queue(&mut self, id: usize, timestamp: u64) -> GovernResult<u64> {
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
    pub fn pre_execute(&mut self, id: usize, timestamp: u64) -> GovernResult<()> {
        let proposal_state = self.get_state(id, timestamp)?;
        if proposal_state != ProposalState::Queued {
            return Err("proposal can only be executed if it is queued");
        }

        let proposal = &mut self.proposals[id];
        proposal.executing = true;
        self.timelock.pre_execute_transaction(&proposal.task, timestamp)
    }

    pub fn post_execute(&mut self, id: usize, result: bool, timestamp: u64) -> GovernResult<()> {
        let proposal_state = self.get_state(id, timestamp)?;
        if proposal_state != ProposalState::Executing {
            return Err("proposal is not executing");
        }

        let proposal = &mut self.proposals[id];
        proposal.executed = result;
        self.timelock.post_execute_transaction(proposal.task.to_owned(), result);
        Ok(())
    }

    /// cancels a proposal only if sender is the proposer, or proposer delegates dropped below proposal threshold
    pub fn cancel(&mut self, id: usize, timestamp: u64, caller: Principal, proposer_votes: u64) -> GovernResult<()> {
        let proposal_state = self.get_state(id, timestamp)?;
        if proposal_state != ProposalState::Executing {
            return Err("cannot cancel executing proposal");
        } else if proposal_state != ProposalState::Executed {
            return Err("cannot cancel executed proposal");
        }

        let proposal = &mut self.proposals[id];
        if caller != proposal.proposer {
            if proposer_votes > self.proposal_threshold {
                return Err("proposer above threshold");
            }
        }
        proposal.canceled = true;
        self.timelock.cancel_transaction(&proposal.task);
        Ok(())
    }

    pub fn cast_vote(
        &mut self,
        id: usize,
        vote_type: VoteType,
        votes: u64,
        reason: Option<String>,
        caller: Principal,
        timestamp: u64,
    ) -> GovernResult<Receipt> {
        let proposal_state = self.get_state(id, timestamp)?;
        if proposal_state != ProposalState::Active {
            return Err("voting is closed");
        }

        let proposal = &mut self.proposals[id];
        match vote_type {
            VoteType::Support => {
                proposal.support_votes += votes;
            }
            VoteType::Against => {
                proposal.against_votes += votes;
            }
            VoteType::Abstain => {
                proposal.abstain_votes += votes;
            }
        }

        let reason = match reason {
            Some(r) => {
                let buf = r.into_bytes();
                let offset = self.stable_memory.offset;
                let len = self.stable_memory.write(buf.as_slice()).map_err(|_| "Stable memory error")?;
                Some(Position {
                    offset,
                    len
                })
            }
            None => { None }
        };
        let receipt = Receipt::new(vote_type, votes, reason);
        proposal.receipts.insert(caller, receipt.clone());

        Ok(receipt)
    }

    pub fn get_proposal(&self, id: usize) -> GovernResult<ProposalInfo> {
        match self.proposals.get(id) {
            Some(p) => {
                let pos = &p.description;
                let mut buf = vec![0u8; pos.len];
                self.stable_memory.read(pos.offset, buf.as_mut_slice()).map_err(|_| "Stable memory error")?;
                let str = String::from_utf8(buf).map_err(|_| "Err utf-8 format")?;
                Ok(p.to_info(str))
            }
            None => { Err("invalid proposal id") }
        }
    }

    /// get specific number of proposal, in reverse sequence
    /// page: from which page, start from 0
    /// num: number of item in a page
    pub fn get_proposal_pages(&self, page: usize, num: usize, timestamp: u64) -> GovernResult<Vec<(ProposalDigest, ProposalState)>> {
        let proposal_count = self.proposals.len();
        if proposal_count == 0 || page * num >= proposal_count{
            return Ok(vec![]);
        }
        let mut proposals = self.proposals.clone();
        proposals.reverse();
        let start = page * num;
        let end = if start + num > proposal_count {
            proposal_count
        } else {
            start + num
        };
        Ok(proposals[start..end].iter().map(|x| {
            (x.digest(), self.get_state(x.id, timestamp).unwrap())
        }).collect())
    }

    pub fn get_receipt(&self, id: usize, voter: Principal) -> GovernResult<ReceiptInfo> {
        match self.proposals.get(id) {
            Some(p) => {
                match p.receipts.get(&voter) {
                    Some(r) => {

                        let reason = match &r.reason {
                            Some(pos) =>  {
                                let mut buf = vec![0u8; pos.len];
                                self.stable_memory.read(pos.offset, buf.as_mut_slice()).map_err(|_| "Stable memory error")?;
                                let str = String::from_utf8(buf).unwrap_or("".to_string());
                                Some(str)
                            }
                            None => { None }
                        };
                        Ok(r.to_info(reason))
                    }
                    None => { Err("receipt not found") }
                }
            }
            None => { Err("invalid proposal id") }
        }
    }

    /// get specific number of voting receipt
    /// page: from which page, start from 0
    /// num: number of item in a page
    pub fn get_receipt_pages(&self, id: usize, page: usize, num: usize) -> GovernResult<Vec<(Principal, ReceiptDigest)>> {
        match self.proposals.get(id) {
            Some(p) => {
                let receipts_count = p.receipts.len();
                if p.receipts.is_empty() || page * num >= receipts_count {
                    return Ok(vec![]);
                }
                let receipts = Vec::from_iter(p.receipts.clone().into_iter());
                let start = page * num;
                let end = if start + num > receipts_count {
                    receipts_count
                } else {
                    start + num
                };
                Ok(receipts[start..end].iter().map(|(x, y)| {
                    (x.to_owned(), y.digest())
                }).collect::<Vec<(Principal, ReceiptDigest)>>())
            }
            None => {
                Err("invalid proposal id")
            }
        }
    }

    pub fn get_task(&self, id: usize) -> GovernResult<Task> {
        match self.proposals.get(id) {
            Some(p) => {
                Ok(p.task.clone())
            }
            None => {
                Err("Invalid proposal id")
            }
        }
    }

    pub fn get_state(&self, id: usize, timestamp: u64) -> GovernResult<ProposalState> {
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

    pub fn set_quorum_votes(&mut self, quorum: u64) {
        self.quorum_votes = quorum;
    }

    pub fn set_vote_delay(&mut self, delay: u64) {
        self.voting_delay = delay;
    }

    pub fn set_vote_period(&mut self, period: u64) {
        self.voting_period = period;
    }

    pub fn set_proposal_threshold(&mut self, threshold: u64) {
        self.proposal_threshold = threshold;
    }

    pub fn set_pending_admin(&mut self, pending_admin: Principal) {
        self.pending_admin = Some(pending_admin);
    }

    pub fn accept_admin(&mut self) {
        assert!(self.pending_admin.is_some());
        self.admin = self.pending_admin.unwrap();
        self.pending_admin = None;
    }

    pub(crate) fn digest(&self) -> GovernorBravoInfo {
        GovernorBravoInfo {
            admin: self.admin,
            pending_admin: self.pending_admin,
            name: self.name.clone(),
            quorum_votes: self.quorum_votes,
            voting_delay: self.voting_delay,
            voting_period: self.voting_period,
            proposal_threshold: self.proposal_threshold,
            proposals_num: self.proposals.len(),
            gov_token: self.gov_token,
            stable_memory: self.stable_memory.clone(),
        }
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
            timelock: Timelock::default(),
            stable_memory: Default::default(),
        }
    }
}