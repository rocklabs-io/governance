/**
 * Module     : timelock.rs
 * Copyright  : 2021 DFinance Team
 * License    : Apache 2.0 with LLVM Exception
 * Maintainer : DFinance Team <hello@dfinance.ai>
 * Stability  : Experimental
 */

use std::collections::HashSet;
use ic_kit::candid::{CandidType, Deserialize};
use ic_kit::{ic, Principal};

#[derive(Deserialize, CandidType, Hash, PartialEq, Eq, Clone)]
pub struct Task {
    /// principal of target canister
    target: Principal,
    /// method name to call
    method: String,
    /// encoded arguments
    arguments: Vec<u8>,
    /// with cycles
    cycles: u64,
    /// timestamp that the proposal will be available for execution, set once the vote succeed
    eta: u64,
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
            eta: 0
        }
    }
}

#[derive(Deserialize, CandidType)]
pub struct Timelock {
    delay: u64,
    queued_transactions: HashSet<Task>
}

pub const ONE_DAY : u64 = 24 * 3600 * 1_000_000_000;

impl Timelock {
    /// grace period for execution
    const GRACE_PERIOD : u64 = 14 * ONE_DAY;
    /// minimum delay for time lock execution
    const MIN_DELAY : u64 = 2 * ONE_DAY;
    /// maximum delay for time lock execution
    const MAX_DELAY : u64 = 30 * ONE_DAY;

    fn new(delay: u64) -> Self {
        Timelock {
            delay,
            queued_transactions: HashSet::new()
        }
    }

    fn set_delay(&mut self, delay: u64) {
        self.delay = delay;
    }

    fn queue_transaction(&mut self, task: Task) {
        self.queued_transactions.insert(task);
    }

    fn cancel_transaction(&mut self, task: &Task) {
        self.queued_transactions.remove(&task);
    }

    async fn execute_transaction(&mut self, task: &Task, timestamp: u64) -> Result<Vec<u8>, &'static str> {
        if self.queued_transactions.contains(task) {
            return Err("Transaction hasn't been queued");
        }
        if timestamp >= task.eta {
            return Err("Transaction hasn't surpassed time lock")
        };
        if timestamp <= task.eta + Timelock::GRACE_PERIOD {
            return Err("Transaction is stale");
        }

        self.queued_transactions.remove(task);

        let result = ic::call_raw(
            task.target,
            task.method.to_owned(),
            task.arguments.to_owned(),
            task.cycles)
            .await;

        match result {
            Ok(ret) => {
                Ok(ret)
            }
            Err(_) => {
                self.queued_transactions.insert(task.clone());
                Err("Execute error")
            }
        }
    }
}

impl Default for Timelock {
    fn default() -> Self {
        Self {
            delay: 0,
            queued_transactions: HashSet::new()
        }
    }
}