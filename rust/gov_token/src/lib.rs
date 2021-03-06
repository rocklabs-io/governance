/**
* Module     : lib.rs
* Copyright  : 2021 DFinance Team
* License    : Apache 2.0 with LLVM Exception
* Maintainer : DFinance Team <hello@dfinance.ai>
* Stability  : Experimental
*/
use candid::{candid_method, CandidType, Deserialize, Int, Nat, export_service};
use cap_sdk::{handshake, insert, Event, IndefiniteEvent, IndefiniteEventBuilder, DetailsBuilder, TypedEvent, CapEnv};
use cap_std::dip20::cap::DIP20Details;
use cap_std::dip20::{Operation, TransactionStatus, TxRecord};
use ic_cdk_macros::*;
use ic_kit::{ic, Principal};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::convert::Into;
use std::string::String;

#[derive(CandidType, Default, Deserialize)]
pub struct TxLog {
    pub ie_records: VecDeque<IndefiniteEvent>,
}

pub fn tx_log<'a>() -> &'a mut TxLog {
    ic_kit::ic::get_mut::<TxLog>()
}

#[allow(non_snake_case)]
#[derive(Deserialize, CandidType, Clone, Debug)]
struct Metadata {
    logo: String,
    name: String,
    symbol: String,
    decimals: u8,
    totalSupply: Nat,
    owner: Principal,
    fee: Nat,
}

#[derive(Deserialize, CandidType, Clone, Debug)]
struct StatsData {
    logo: String,
    name: String,
    symbol: String,
    decimals: u8,
    total_supply: Nat,
    owner: Principal,
    fee: Nat,
    fee_to: Principal,
    history_size: usize,
    deploy_time: u64,
}

#[allow(non_snake_case)]
#[derive(Deserialize, CandidType, Clone, Debug)]
struct TokenInfo {
    metadata: Metadata,
    feeTo: Principal,
    // status info
    historySize: usize,
    deployTime: u64,
    holderNumber: usize,
    cycles: u64,
}

impl Default for StatsData {
    fn default() -> Self {
        StatsData {
            logo: "".to_string(),
            name: "".to_string(),
            symbol: "".to_string(),
            decimals: 0u8,
            total_supply: Nat::from(0),
            owner: Principal::anonymous(),
            fee: Nat::from(0),
            fee_to: Principal::anonymous(),
            history_size: 0,
            deploy_time: 0,
        }
    }
}

type Balances = HashMap<Principal, Nat>;
type Allowances = HashMap<Principal, HashMap<Principal, Nat>>;

#[derive(CandidType, Debug, PartialEq)]
pub enum TxError {
    InsufficientBalance,
    InsufficientAllowance,
    Unauthorized,
    LedgerTrap,
    AmountTooSmall,
    BlockUsed,
    ErrorOperationStyle,
    ErrorTo,
    Other,
}
pub type TxReceipt = Result<Nat, TxError>;

#[derive(Deserialize, CandidType, Debug, PartialEq)]
struct CheckPoint {
    timestamp: Nat,
    votes: Nat,
}
type Delegates = HashMap<Principal, Principal>;
type CheckPoints = HashMap<Principal, Vec<CheckPoint>>;

#[init]
#[candid_method(init)]
fn init(
    logo: String,
    name: String,
    symbol: String,
    decimals: u8,
    total_supply: Nat,
    owner: Principal,
    fee: Nat,
    fee_to: Principal,
    cap: Principal,
) {
    let stats = ic::get_mut::<StatsData>();
    stats.logo = logo;
    stats.name = name;
    stats.symbol = symbol;
    stats.decimals = decimals;
    stats.total_supply = total_supply.clone();
    stats.owner = owner;
    stats.fee = fee;
    stats.fee_to = fee_to;
    stats.history_size = 1;
    stats.deploy_time = ic::time();
    handshake(1_000_000_000_000, Some(cap));
    let balances = ic::get_mut::<Balances>();
    balances.insert(owner, total_supply.clone());
    let _ = add_record(
        owner,
        Operation::Mint,
        owner,
        owner,
        total_supply,
        Nat::from(0),
        ic::time(),
        TransactionStatus::Succeeded,
    );
}

fn _transfer(from: Principal, to: Principal, value: Nat) {
    let balances = ic::get_mut::<Balances>();
    let from_balance = balance_of(from);
    let from_balance_new = from_balance - value.clone();
    if from_balance_new != 0 {
        balances.insert(from, from_balance_new);
    } else {
        balances.remove(&from);
    }
    let to_balance = balance_of(to);
    let to_balance_new = to_balance + value;
    if to_balance_new != 0 {
        balances.insert(to, to_balance_new);
    }
}

fn _charge_fee(user: Principal, fee_to: Principal, fee: Nat) {
    let stats = ic::get::<StatsData>();
    if stats.fee > Nat::from(0) {
        _transfer(user, fee_to, fee);
    }
}

fn _delegate(delegator: Principal, delegatee: Principal) -> Nat {
    let delegates = ic::get_mut::<Delegates>();
    let current_delegate = ic::get::<Delegates>().get(&delegator);
    let delegator_balance = balance_of(delegator);

    delegates.insert(delegator, delegatee);
    _move_delegates(current_delegate, Some(&delegatee), delegator_balance.clone(), Nat::from(0));

    delegator_balance
}

fn _move_delegates(from: Option<&Principal>, to: Option<&Principal>, amount: Nat, fee: Nat) {
    if amount > 0u64 {
        if let Some(from_) = from {
            let from_delegates_old = _get_votes(from_);
            let from_delegates_new = from_delegates_old - amount.clone() - fee;
            _write_check_point(from_, from_delegates_new);
        }

        if let Some(to_) = to {
            let to_delegates_old = _get_votes(to_);
            let to_delegates_new = to_delegates_old + amount.clone();
            _write_check_point(to_, to_delegates_new);
        }
    }
}

fn _get_votes(who: &Principal) -> Nat {
    let check_points = ic::get::<CheckPoints>();
    match check_points.get(who) {
        Some(check_point) => {
            check_point.last().unwrap().votes.clone()
        },
        None => Nat::from(0)
    }
}

fn _write_check_point(who: &Principal, new_votes: Nat) {
    let check_points = ic::get_mut::<CheckPoints>();
    
    let check_point = check_points.entry(who.to_owned()).or_insert(vec![]);
    let timestamp = Nat::from(ic::time());
    if !check_point.is_empty() && check_point.last().unwrap().timestamp == timestamp {
        check_point.last_mut().unwrap().votes = new_votes;
    } else {
        check_point.push(CheckPoint {timestamp, votes: new_votes});
    }
}

/// gets the current votes balance for `who`
#[query(name = "getCurrentVotes")]
#[candid_method(query, rename = "getCurrentVotes")]
fn get_current_votes(who: Principal) -> Nat {
    _get_votes(&who)
}

#[query(name = "getPriorVotes")]
#[candid_method(query, rename = "getPriorVotes")]
fn get_prior_votes(who: Principal, timestamp: Nat) -> Nat {
    let check_points = ic::get::<CheckPoints>();
    let account_check_points = match check_points.get(&who) {
        Some(cp) => cp,
        None => { return Nat::from(0); }
    };
    let current_check_point = account_check_points.last().unwrap();
    if current_check_point.timestamp <= timestamp {
        return current_check_point.votes.clone();
    }
    let oldest_check_point = account_check_points.first().unwrap();
    if oldest_check_point.timestamp > timestamp {
        return oldest_check_point.votes.clone();
    }
    
    let idx = account_check_points
        .binary_search_by(|item| item.timestamp.cmp(&timestamp))
        .unwrap_or_else(|x| x - 1);

    account_check_points[idx].votes.clone()
}

#[update(name = "delegate")]
#[candid_method(update)]
async fn delegate(delegatee: Principal) -> TxReceipt {
    let caller = ic::caller();
    if balance_of(caller) == 0 {
        return Err(TxError::InsufficientBalance);
    }
    let value = _delegate(caller, delegatee);

    let event = IndefiniteEventBuilder::new()
       .caller(caller)
       .operation(String::from("delegate"))
       .details(
           DetailsBuilder::new()
            .insert("from", caller)
            .insert("to", delegatee)
            .insert("amount", value)
            .insert("index", Nat::from(0))
            .insert("fee", Nat::from(0))
            .insert("timestamp", ic::time())
            .insert("status", String::from("succeeded"))
            .build()
        )
       .build()
       .unwrap();

    insert_into_cap(event).await
}

#[update(name = "transfer")]
#[candid_method(update)]
async fn transfer(to: Principal, value: Nat) -> TxReceipt {
    let from = ic::caller();
    let stats = ic::get_mut::<StatsData>();
    if balance_of(from) < value.clone() + stats.fee.clone() {
        return Err(TxError::InsufficientBalance);
    }
    _charge_fee(from, stats.fee_to, stats.fee.clone());
    _transfer(from, to, value.clone());
    _move_delegates(Some(&from), Some(&to), value.clone(), stats.fee.clone());
    stats.history_size += 1;

    add_record(
        from,
        Operation::Transfer,
        from,
        to,
        value,
        stats.fee.clone(),
        ic::time(),
        TransactionStatus::Succeeded,
    )
    .await
}

#[update(name = "transferFrom")]
#[candid_method(update, rename = "transferFrom")]
async fn transfer_from(from: Principal, to: Principal, value: Nat) -> TxReceipt {
    let owner = ic::caller();
    let from_allowance = allowance(from, owner);
    let stats = ic::get_mut::<StatsData>();
    if from_allowance < value.clone() + stats.fee.clone() {
        return Err(TxError::InsufficientAllowance);
    }
    let from_balance = balance_of(from);
    if from_balance < value.clone() + stats.fee.clone() {
        return Err(TxError::InsufficientBalance);
    }
    _charge_fee(from, stats.fee_to, stats.fee.clone());
    _transfer(from, to, value.clone());
    _move_delegates(Some(&from), Some(&to), value.clone(), stats.fee.clone());
    let allowances = ic::get_mut::<Allowances>();
    match allowances.get(&from) {
        Some(inner) => {
            let result = inner.get(&owner).unwrap().clone();
            let mut temp = inner.clone();
            if result.clone() - value.clone() - stats.fee.clone() != 0 {
                temp.insert(owner, result.clone() - value.clone() - stats.fee.clone());
                allowances.insert(from, temp);
            } else {
                temp.remove(&owner);
                if temp.len() == 0 {
                    allowances.remove(&from);
                } else {
                    allowances.insert(from, temp);
                }
            }
        }
        None => {
            assert!(false);
        }
    }
    stats.history_size += 1;

    add_record(
        owner,
        Operation::TransferFrom,
        from,
        to,
        value,
        stats.fee.clone(),
        ic::time(),
        TransactionStatus::Succeeded,
    )
    .await
}

#[update(name = "approve")]
#[candid_method(update)]
async fn approve(spender: Principal, value: Nat) -> TxReceipt {
    let owner = ic::caller();
    let stats = ic::get_mut::<StatsData>();
    if balance_of(owner) < stats.fee.clone() {
        return Err(TxError::InsufficientBalance);
    }
    _charge_fee(owner, stats.fee_to, stats.fee.clone());
    let v = value.clone() + stats.fee.clone();
    let allowances = ic::get_mut::<Allowances>();
    match allowances.get(&owner) {
        Some(inner) => {
            let mut temp = inner.clone();
            if v.clone() != 0 {
                temp.insert(spender, v.clone());
                allowances.insert(owner, temp);
            } else {
                temp.remove(&spender);
                if temp.len() == 0 {
                    allowances.remove(&owner);
                } else {
                    allowances.insert(owner, temp);
                }
            }
        }
        None => {
            if v.clone() != 0 {
                let mut inner = HashMap::new();
                inner.insert(spender, v.clone());
                let allowances = ic::get_mut::<Allowances>();
                allowances.insert(owner, inner);
            }
        }
    }
    stats.history_size += 1;

    add_record(
        owner,
        Operation::Approve,
        owner,
        spender,
        v,
        stats.fee.clone(),
        ic::time(),
        TransactionStatus::Succeeded,
    )
    .await
}

#[update(name = "mint")]
#[candid_method(update, rename = "mint")]
async fn mint(to: Principal, amount: Nat) -> TxReceipt {
    let caller = ic::caller();
    let stats = ic::get_mut::<StatsData>();
    if caller != stats.owner {
        return Err(TxError::Unauthorized);
    }
    let to_balance = balance_of(to);
    let balances = ic::get_mut::<Balances>();
    balances.insert(to, to_balance + amount.clone());
    stats.total_supply += amount.clone();
    stats.history_size += 1;

    add_record(
        caller,
        Operation::Mint,
        caller,
        to,
        amount,
        Nat::from(0),
        ic::time(),
        TransactionStatus::Succeeded,
    )
    .await
}

#[update(name = "burn")]
#[candid_method(update, rename = "burn")]
async fn burn(amount: Nat) -> TxReceipt {
    let caller = ic::caller();
    let stats = ic::get_mut::<StatsData>();
    let caller_balance = balance_of(caller);
    if caller_balance.clone() < amount.clone() {
        return Err(TxError::InsufficientBalance);
    }
    let balances = ic::get_mut::<Balances>();
    balances.insert(caller, caller_balance - amount.clone());
    stats.total_supply -= amount.clone();
    stats.history_size += 1;

    add_record(
        caller,
        Operation::Burn,
        caller,
        caller,
        amount,
        Nat::from(0i32),
        ic::time(),
        TransactionStatus::Succeeded,
    )
    .await
}

#[update(name = "setName")]
#[candid_method(update, rename = "setName")]
fn set_name(name: String) {
    let stats = ic::get_mut::<StatsData>();
    assert_eq!(ic::caller(), stats.owner);
    stats.name = name;
}

#[update(name = "setLogo")]
#[candid_method(update, rename = "setLogo")]
fn set_logo(logo: String) {
    let stats = ic::get_mut::<StatsData>();
    assert_eq!(ic::caller(), stats.owner);
    stats.logo = logo;
}

#[update(name = "setFee")]
#[candid_method(update, rename = "setFee")]
fn set_fee(fee: Nat) {
    let stats = ic::get_mut::<StatsData>();
    assert_eq!(ic::caller(), stats.owner);
    stats.fee = fee;
}

#[update(name = "setFeeTo")]
#[candid_method(update, rename = "setFeeTo")]
fn set_fee_to(fee_to: Principal) {
    let stats = ic::get_mut::<StatsData>();
    assert_eq!(ic::caller(), stats.owner);
    stats.fee_to = fee_to;
}

#[update(name = "setOwner")]
#[candid_method(update, rename = "setOwner")]
fn set_owner(owner: Principal) {
    let stats = ic::get_mut::<StatsData>();
    assert_eq!(ic::caller(), stats.owner);
    stats.owner = owner;
}

#[query(name = "balanceOf")]
#[candid_method(query, rename = "balanceOf")]
fn balance_of(id: Principal) -> Nat {
    let balances = ic::get::<Balances>();
    match balances.get(&id) {
        Some(balance) => balance.clone(),
        None => Nat::from(0),
    }
}

#[query(name = "allowance")]
#[candid_method(query)]
fn allowance(owner: Principal, spender: Principal) -> Nat {
    let allowances = ic::get::<Allowances>();
    match allowances.get(&owner) {
        Some(inner) => match inner.get(&spender) {
            Some(value) => value.clone(),
            None => Nat::from(0),
        },
        None => Nat::from(0),
    }
}

#[query(name = "logo")]
#[candid_method(query, rename = "logo")]
fn get_logo() -> String {
    let stats = ic::get::<StatsData>();
    stats.logo.clone()
}

#[query(name = "name")]
#[candid_method(query)]
fn name() -> String {
    let stats = ic::get::<StatsData>();
    stats.name.clone()
}

#[query(name = "symbol")]
#[candid_method(query)]
fn symbol() -> String {
    let stats = ic::get::<StatsData>();
    stats.symbol.clone()
}

#[query(name = "decimals")]
#[candid_method(query)]
fn decimals() -> u8 {
    let stats = ic::get::<StatsData>();
    stats.decimals
}

#[query(name = "totalSupply")]
#[candid_method(query, rename = "totalSupply")]
fn total_supply() -> Nat {
    let stats = ic::get::<StatsData>();
    stats.total_supply.clone()
}

#[query(name = "owner")]
#[candid_method(query)]
fn owner() -> Principal {
    let stats = ic::get::<StatsData>();
    stats.owner
}

#[query(name = "getMetadata")]
#[candid_method(query, rename = "getMetadata")]
fn get_metadata() -> Metadata {
    let s = ic::get::<StatsData>().clone();
    Metadata {
        logo: s.logo,
        name: s.name,
        symbol: s.symbol,
        decimals: s.decimals,
        totalSupply: s.total_supply,
        owner: s.owner,
        fee: s.fee,
    }
}

#[query(name = "historySize")]
#[candid_method(query, rename = "historySize")]
fn history_size() -> usize {
    let stats = ic::get::<StatsData>();
    stats.history_size
}

#[query(name = "getTokenInfo")]
#[candid_method(query, rename = "getTokenInfo")]
fn get_token_info() -> TokenInfo {
    let stats = ic::get::<StatsData>().clone();
    let balance = ic::get::<Balances>();

    return TokenInfo {
        metadata: get_metadata(),
        feeTo: stats.fee_to,
        historySize: stats.history_size,
        deployTime: stats.deploy_time,
        holderNumber: balance.len(),
        cycles: ic::balance(),
    };
}

#[query(name = "getHolders")]
#[candid_method(query, rename = "getHolders")]
fn get_holders(start: usize, limit: usize) -> Vec<(Principal, Nat)> {
    let mut balance = Vec::new();
    for (k, v) in ic::get::<Balances>().clone() {
        balance.push((k, v));
    }
    balance.sort_by(|a, b| b.1.cmp(&a.1));
    let limit: usize = if start + limit > balance.len() {
        balance.len() - start
    } else {
        limit
    };
    balance[start..start + limit].to_vec()
}

#[query(name = "getAllowanceSize")]
#[candid_method(query, rename = "getAllowanceSize")]
fn get_allowance_size() -> usize {
    let mut size = 0;
    let allowances = ic::get::<Allowances>();
    for (_, v) in allowances.iter() {
        size += v.len();
    }
    size
}

#[query(name = "getUserApprovals")]
#[candid_method(query, rename = "getUserApprovals")]
fn get_user_approvals(who: Principal) -> Vec<(Principal, Nat)> {
    let allowances = ic::get::<Allowances>();
    match allowances.get(&who) {
        Some(allow) => return Vec::from_iter(allow.clone().into_iter()),
        None => return Vec::new(),
    }
}

#[query(name = "__get_candid_interface_tmp_hack")]
fn export_candid() -> String {
    export_service!();
    __export_service()
}

#[pre_upgrade]
fn pre_upgrade() {
    ic::stable_store((
        ic::get::<StatsData>().clone(),
        ic::get::<Balances>(),
        ic::get::<Allowances>(),
        ic::get::<Delegates>(),
        ic::get::<CheckPoints>(),
        tx_log(),
        CapEnv::to_archive()
    ))
    .unwrap();
}

#[post_upgrade]
fn post_upgrade() {
    let (metadata_stored, balances_stored, allowances_stored, delegates_stored, checkpoints_stored, tx_log_stored, cap_env): (
        StatsData,
        Balances,
        Allowances,
        Delegates,
        CheckPoints,
        TxLog,
        CapEnv
    ) = ic::stable_restore().unwrap();
    let stats = ic::get_mut::<StatsData>();
    *stats = metadata_stored;

    let balances = ic::get_mut::<Balances>();
    *balances = balances_stored;

    let allowances = ic::get_mut::<Allowances>();
    *allowances = allowances_stored;

    let deletages = ic::get_mut::<Delegates>();
    *deletages = delegates_stored;

    let checkpoints = ic::get_mut::<CheckPoints>();
    *checkpoints = checkpoints_stored;

    let tx_log = tx_log();
    *tx_log = tx_log_stored;

    CapEnv::load_from_archive(cap_env);
}

async fn add_record(
    caller: Principal,
    op: Operation,
    from: Principal,
    to: Principal,
    amount: Nat,
    fee: Nat,
    timestamp: u64,
    status: TransactionStatus,
) -> TxReceipt {
    insert_into_cap(Into::<IndefiniteEvent>::into(Into::<Event>::into(Into::<
        TypedEvent<DIP20Details>,
    >::into(
        TxRecord {
            caller: Some(caller),
            index: Nat::from(0i32),
            from,
            to,
            amount: Nat::from(amount),
            fee: Nat::from(fee),
            timestamp: Int::from(timestamp),
            status,
            operation: op,
        },
    ))))
    .await
}

pub async fn insert_into_cap(ie: IndefiniteEvent) -> TxReceipt {
    let tx_log = tx_log();
    if let Some(failed_ie) = tx_log.ie_records.pop_front() {
        let _ = insert_into_cap_priv(failed_ie).await;
    }
    insert_into_cap_priv(ie).await
}

async fn insert_into_cap_priv(ie: IndefiniteEvent) -> TxReceipt {
    let insert_res = insert(ie.clone())
        .await
        .map(|tx_id| Nat::from(tx_id))
        .map_err(|_| TxError::Other);

    if insert_res.is_err() {
        tx_log().ie_records.push_back(ie.clone());
    }

    insert_res
}
