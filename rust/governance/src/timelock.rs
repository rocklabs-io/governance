/**
 * Module     : timelock.rs
 * Copyright  : 2021 Rocklabs
 * License    : Apache 2.0 with LLVM Exception
 * Maintainer : Rocklabs <hello@rocklabs.io>
 * Stability  : Experimental
 */

use std::collections::HashSet;
use ic_kit::candid::{CandidType, Deserialize};
use ic_kit::{Principal};

#[derive(Deserialize, CandidType, Hash, PartialEq, Eq, Clone, Debug)]
pub struct Task {
    /// principal of target canister
    pub(crate) target: Principal,
    /// method name to call
    pub(crate) method: String,
    /// encoded arguments
    pub(crate) arguments: Vec<u8>,
    /// with cycles
    pub(crate) cycles: u64,
    /// timestamp that the proposal will be available for execution, set once the vote succeed
    pub(crate) eta: u64,
}

impl Task {
    pub(crate) fn new(
        target: Principal,
        method: String,
        arguments: Vec<u8>,
        cycles: u64,
    ) -> Self {
        Self {
            target,
            method,
            arguments,
            cycles,
            eta: 0,
        }
    }
}

#[derive(Deserialize, CandidType, Clone, Debug)]
pub struct Timelock {
    pub(crate) delay: u64,
    pub(crate) queued_transactions: HashSet<Task>,
}

pub const ONE_DAY: u64 = 24 * 3600 * 1_000_000_000;

impl Timelock {
    /// grace period for execution
    pub(crate) const GRACE_PERIOD: u64 = 14 * ONE_DAY;
    /// minimum delay for time lock execution
    pub(crate) const MIN_DELAY: u64 = 2 * ONE_DAY;
    /// maximum delay for time lock execution
    pub(crate) const MAX_DELAY: u64 = 30 * ONE_DAY;

    fn new(delay: u64) -> Self {
        Timelock {
            delay,
            queued_transactions: HashSet::new(),
        }
    }

    pub(crate) fn set_delay(&mut self, delay: u64) {
        self.delay = delay;
    }

    pub(crate) fn queue_transaction(&mut self, task: Task) {
        self.queued_transactions.insert(task);
    }

    pub(crate) fn cancel_transaction(&mut self, task: &Task) {
        self.queued_transactions.remove(&task);
    }

    pub(crate) fn pre_execute_transaction(&mut self, task: &Task, timestamp: u64) -> Result<(), &'static str> {
        if !self.queued_transactions.contains(task) {
            return Err("Transaction hasn't been queued");
        }
        if timestamp < task.eta {
            return Err("Transaction hasn't surpassed time lock");
        };
        if timestamp > task.eta + Timelock::GRACE_PERIOD {
            return Err("Transaction is stale");
        }

        self.queued_transactions.remove(task);
        Ok(())
    }

    pub(crate) fn post_execute_transaction(&mut self, task: Task, result: bool) {
        if !result {
            self.queued_transactions.insert(task);
        }
    }
}

impl Default for Timelock {
    fn default() -> Self {
        Self {
            delay: 0,
            queued_transactions: HashSet::new(),
        }
    }
}