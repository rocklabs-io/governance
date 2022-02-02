use std::thread::sleep;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use ic_kit::{Method, MockContext, async_test};
use ic_kit::mock_principals::{alice, bob};
use crate::VoteType::Support;
use super::*;

#[test]
fn save_candid() {
    use std::env;
    use std::fs::write;
    use std::path::PathBuf;

    let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    write(dir.join("governance.did"), export_candid()).expect("Write failed.");
}

fn set_up() -> &'static mut MockContext {
    MockContext::new()
        .with_caller(alice())
        .with_handler(Method::new().name("getCurrentVotes").response(Nat::from(5000)))
        .with_handler(Method::new().name("getPriorVotes").response(Nat::from(5000)))
        .with_handler(Method::new().name("test"))
        .inject()
}

#[async_test]
async fn test_propose() -> Result<(), String> {
    let ctx = set_up();

    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.initialize(
            alice(),
            "Test".to_string(),
            100,
            10e9 as u64,
            10e9 as u64,
            500,
            10e9 as u64,
            Principal::anonymous(),
        );
    });

    propose(
        "test".to_string(),
        "test".to_string(),
        Principal::management_canister(),
        "test".to_string(),
        vec![],
        0,
    ).await?;

    let (_, state) = get_proposal(0)?;
    if state != ProposalState::Pending {
        return Err("New proposal must be pending".to_string());
    }

    Ok(())
}

#[async_test]
async fn test_propose_fail_below_threshold() -> Result<(), String> {
    set_up();

    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.initialize(
            alice(),
            "Test".to_string(),
            1000,
            10e9 as u64,
            10e9 as u64,
            5001,
            10e9 as u64,
            Principal::anonymous(),
        );
    });

    println!("{}",
             propose(
                 "test".to_string(),
                 "test".to_string(),
                 Principal::management_canister(),
                 "test".to_string(),
                 vec![],
                 0,
             ).await.unwrap_err()
    );

    Ok(())
}

#[async_test]
async fn test_cast_vote() -> Result<(), String> {
    let ctx = set_up();

    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.initialize(
            alice(),
            "Test".to_string(),
            1000,
            1e9 as u64,
            10e9 as u64,
            5000,
            10e9 as u64,
            Principal::anonymous(),
        );

        bravo.propose(
            alice(),
            Nat::from(10000),
            "Test".to_string(),
            "".to_string(),
            Principal::management_canister(),
            "test".to_string(),
            vec![],
            0,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_nanos() as u64,
        );
    });

    sleep(Duration::from_secs(1));
    cast_vote(0, Support, None).await?;

    let (proposal, state) = get_proposal(0)?;
    if state != ProposalState::Active {
        return Err("Proposal must be Active".to_string());
    }
    if proposal.support_votes != 5000 {
        return Err("Support votes invalid".to_string());
    }

    Ok(())
}

#[async_test]
async fn test_queue() -> Result<(), String> {
    let ctx = set_up();

    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.initialize(
            alice(),
            "Test".to_string(),
            1000,
            0 as u64,
            3e9 as u64,
            5000,
            10e9 as u64,
            Principal::anonymous(),
        );

        bravo.propose(
            alice(),
            Nat::from(10000),
            "Test".to_string(),
            "".to_string(),
            Principal::management_canister(),
            "test".to_string(),
            vec![],
            0,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_nanos() as u64,
        );

        bravo.cast_vote(
            0,
            VoteType::Support,
            Nat::from(5000),
            None,
            alice(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_nanos() as u64,
        )
    });

    sleep(Duration::from_secs(3));
    queue(0).await?;
    let state = get_proposal_state(0)?;
    if state != ProposalState::Queued {
        return Err("Proposal must be queued".to_string());
    }

    Ok(())
}

#[async_test]
async fn test_queue_fail_quorum_limit() -> Result<(), String> {
    let ctx = set_up();

    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.initialize(
            alice(),
            "Test".to_string(),
            5001,
            0 as u64,
            3e9 as u64,
            5000,
            10e9 as u64,
            Principal::anonymous(),
        );

        bravo.propose(
            alice(),
            Nat::from(10000),
            "Test".to_string(),
            "".to_string(),
            Principal::management_canister(),
            "test".to_string(),
            vec![],
            0,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_nanos() as u64,
        );

        bravo.cast_vote(
            0,
            VoteType::Support,
            Nat::from(5000),
            None,
            alice(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_nanos() as u64,
        )
    });

    sleep(Duration::from_secs(3));
    println!("{}", queue(0).await.unwrap_err());

    Ok(())
}

#[async_test]
async fn test_queue_fail_not_end() -> Result<(), String> {
    let ctx = set_up();

    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.initialize(
            alice(),
            "Test".to_string(),
            5000,
            0 as u64,
            3e9 as u64,
            5000,
            10e9 as u64,
            Principal::anonymous(),
        );

        bravo.propose(
            alice(),
            Nat::from(10000),
            "Test".to_string(),
            "".to_string(),
            Principal::management_canister(),
            "test".to_string(),
            vec![],
            0,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_nanos() as u64,
        );

        bravo.cast_vote(
            0,
            VoteType::Support,
            Nat::from(5001),
            None,
            alice(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_nanos() as u64,
        )
    });

    sleep(Duration::from_secs(2));
    println!("{}", queue(0).await.unwrap_err());

    Ok(())
}

#[async_test]
async fn test_execute() -> Result<(), String> {
    let ctx = set_up();

    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.initialize(
            alice(),
            "Test".to_string(),
            5000,
            0 as u64,
            1e9 as u64,
            5000,
            1e9 as u64,
            Principal::anonymous(),
        );

        bravo.propose(
            alice(),
            Nat::from(10000),
            "Test".to_string(),
            "".to_string(),
            Principal::management_canister(),
            "test".to_string(),
            vec![],
            0,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_nanos() as u64,
        );

        bravo.cast_vote(
            0,
            VoteType::Support,
            Nat::from(5001),
            None,
            alice(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_nanos() as u64,
        );

        sleep(Duration::from_secs(1));

        bravo.queue(0,
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_nanos() as u64,
        );
    });

    sleep(Duration::from_secs(1));
    execute(0).await?;

    let (_, state) = get_proposal(0)?;
    if state != ProposalState::Executed {
        return Err("Proposal must be executed".to_string());
    }

    Ok(())
}

#[async_test]
async fn test_execute_fail_before_timelock() -> Result<(), String> {
    let ctx = set_up();

    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.initialize(
            alice(),
            "Test".to_string(),
            5000,
            0 as u64,
            1e9 as u64,
            5000,
            1e9 as u64,
            Principal::anonymous(),
        );

        bravo.propose(
            alice(),
            Nat::from(10000),
            "Test".to_string(),
            "".to_string(),
            Principal::management_canister(),
            "test".to_string(),
            vec![],
            0,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_nanos() as u64,
        );

        bravo.cast_vote(
            0,
            VoteType::Support,
            Nat::from(5001),
            None,
            alice(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_nanos() as u64,
        );

        sleep(Duration::from_secs(1));

        bravo.queue(0,
                    SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .expect("Time went backwards")
                        .as_nanos() as u64,
        );
    });

    execute(0).await.unwrap_err();

    Ok(())
}

#[async_test]
async fn test_cancel() -> Result<(), String> {
    let ctx = set_up();

    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.initialize(
            alice(),
            "Test".to_string(),
            5000,
            0 as u64,
            1e9 as u64,
            5000,
            1e9 as u64,
            Principal::anonymous(),
        );

        bravo.propose(
            alice(),
            Nat::from(10000),
            "Test".to_string(),
            "".to_string(),
            Principal::management_canister(),
            "test".to_string(),
            vec![],
            0,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_nanos() as u64,
        );
    });

    cancel(0).await?;

    let state = get_proposal_state(0)?;
    if state != ProposalState::Canceled {
        return Err("Proposal must be canceled".to_string());
    }

    Ok(())
}

#[async_test]
async fn test_cancel_below_threshold() -> Result<(), String> {
    let ctx = set_up();

    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.initialize(
            alice(),
            "Test".to_string(),
            5000,
            0 as u64,
            1e9 as u64,
            6000,
            1e9 as u64,
            Principal::anonymous(),
        );

        bravo.propose(
            bob(),
            Nat::from(10000),
            "Test".to_string(),
            "".to_string(),
            Principal::management_canister(),
            "test".to_string(),
            vec![],
            0,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_nanos() as u64,
        );
    });

    cancel(0).await?;

    let state = get_proposal_state(0)?;
    if state != ProposalState::Canceled {
        return Err("Proposal must be canceled".to_string());
    }

    Ok(())
}

#[async_test]
async fn test_cancel_fail() -> Result<(), String> {
    let ctx = set_up();

    BRAVO.with(|bravo| {
        let mut bravo = bravo.borrow_mut();
        bravo.initialize(
            alice(),
            "Test".to_string(),
            5000,
            0 as u64,
            1e9 as u64,
            4000,
            1e9 as u64,
            Principal::anonymous(),
        );

        bravo.propose(
            bob(),
            Nat::from(10000),
            "Test".to_string(),
            "".to_string(),
            Principal::management_canister(),
            "test".to_string(),
            vec![],
            0,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_nanos() as u64,
        );
    });

    cancel(0).await.unwrap_err();

    Ok(())
}