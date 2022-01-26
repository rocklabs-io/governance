use std::collections::HashSet;
use ic_kit::candid::{CandidType, Deserialize};
use ic_kit::{ic, Principal};

#[derive(Deserialize, CandidType, Hash, PartialEq, Eq, Clone)]
pub struct Task {
    target: Principal,
    method: String,
    arguments: Vec<u8>,
    cycles: u64,
    eta: u64,
}

#[derive(Deserialize, CandidType)]
pub struct Timelock {
    delay: u64,
    queued_transactions: HashSet<Task>
}

const ONE_DAY : u64 = 24 * 3600 * 1_000_000_000;

impl Timelock {
    /// grace period for execution
    const GRACE_PERIOD : u64 = 14 * ONE_DAY;
    /// minimum delay for time lock execution
    const MIN_DELAY : u64 = 2 * ONE_DAY;
    /// maximum delay for time lock execution
    const MAX_DELAY : u64 = 30 * ONE_DAY;

    fn new(delay: u64) -> Timelock {
        Timelock {
            delay,
            queued_transactions: HashSet::new()
        }
    }

    fn set_delay(&mut self, delay: u64) {
        assert!(delay >= Timelock::MIN_DELAY, "Delay must exceed minimum delay");
        assert!(delay <= Timelock::MIN_DELAY, "Delay must not exceed maximum delay");
        self.delay = delay;
    }

    fn queue_transaction(&mut self, task: Task) {
        self.queued_transactions.insert(task);
    }

    fn cancel_transaction(&mut self, task: &Task) {
        self.queued_transactions.remove(&task);
    }

    async fn execute_transaction(&mut self, task: &Task) -> Result<Vec<u8>, &'static str> {
        assert!(self.queued_transactions.contains(task), "Transaction hasn't been queued");
        assert!(ic::time() >= task.eta, "Transaction hasn't surpassed time lock");
        assert!(ic::time() <= task.eta + Timelock::GRACE_PERIOD, "Transaction is stale");

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