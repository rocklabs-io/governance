/**
 * Module     : Token.mo
 * Copyright  : 2022 Rocklabs Team
 * License    : Apache 2.0 with LLVM Exception
 * Maintainer : Rocklabs Team <hello@rocklabs.io>
 * Stability  : Experimental
 */

import Array "mo:base/Array";
import Buffer "mo:base/Buffer";
import ExperimentalCycles "mo:base/ExperimentalCycles";
import HashMap "mo:base/HashMap";
import Iter "mo:base/Iter";
import Nat "mo:base/Nat";
import Option "mo:base/Option";
import Order "mo:base/Order";
import Principal "mo:base/Principal";
import Result "mo:base/Result";
import Time "mo:base/Time";
import Types "./Types";

shared(msg) actor class Token(
    _logo: Text,
    _name: Text,
    _symbol: Text,
    _decimals: Nat8,
    _totalSupply: Nat,
    _owner: Principal,
    _fee: Nat
    ) {
    type Operation = Types.Operation;
    type TransactionStatus = Types.TransactionStatus;
    type TxRecord = Types.TxRecord;
    type Metadata = {
        logo : Text;
        name : Text;
        symbol : Text;
        decimals : Nat8;
        totalSupply : Nat;
        owner : Principal;
        fee : Nat;
    };
    // returns tx index or error msg
    public type TxReceipt = {
        #Ok: Nat;
        #Err: {
            #InsufficientAllowance;
            #InsufficientBalance;
            #ErrorOperationStyle;
            #Unauthorized;
            #LedgerTrap;
            #ErrorTo;
            #Other;
            #BlockUsed;
            #AmountTooSmall;
        };
    };

    private stable var owner_ : Principal = _owner;
    private stable var logo_ : Text = _logo;
    private stable var name_ : Text = _name;
    private stable var decimals_ : Nat8 = _decimals;
    private stable var symbol_ : Text = _symbol;
    private stable var totalSupply_ : Nat = _totalSupply;
    private stable var blackhole : Principal = Principal.fromText("aaaaa-aa");
    private stable var feeTo : Principal = owner_;
    private stable var fee : Nat = _fee;
    private stable var balanceEntries : [(Principal, Nat)] = [];
    private stable var allowanceEntries : [(Principal, [(Principal, Nat)])] = [];
    private var balances = HashMap.HashMap<Principal, Nat>(1, Principal.equal, Principal.hash);
    private var allowances = HashMap.HashMap<Principal, HashMap.HashMap<Principal, Nat>>(1, Principal.equal, Principal.hash);
    balances.put(owner_, totalSupply_);
    private stable let genesis : TxRecord = {
        caller = ?owner_;
        op = #mint;
        index = 0;
        from = blackhole;
        to = owner_;
        amount = totalSupply_;
        fee = 0;
        timestamp = Time.now();
        status = #succeeded;
    };
    private stable var ops : [TxRecord] = [genesis];

    /// storage for delegation
    private var delegates = HashMap.HashMap<Principal, Principal>(1, Principal.equal, Principal.hash);
    private stable var delegateEntries : [(Principal, Principal)] = [];

    type CheckPoint = {
        timestamp: Time.Time ;
        votes: Nat;
    };
    private func _newCheckPoint(timestamp: Time.Time, votes: Nat) : CheckPoint {
        {
            timestamp = timestamp;
            votes = votes;
        }
    };
    private var checkPoints = HashMap.HashMap<Principal, Buffer.Buffer<CheckPoint>>(1, Principal.equal, Principal.hash);
    private stable var checkPointEntries : [(Principal, [CheckPoint])] = [];

    private func addRecord(
        caller: ?Principal, op: Operation, from: Principal, to: Principal, amount: Nat,
        fee: Nat, timestamp: Time.Time, status: TransactionStatus
    ): Nat {
        let index = ops.size();
        let o : TxRecord = {
            caller = caller;
            op = op;
            index = index;
            from = from;
            to = to;
            amount = amount;
            fee = fee;
            timestamp = timestamp;
            status = status;
        };
        ops := Array.append(ops, [o]);
        return index;
    };

    private func _chargeFee(from: Principal, fee: Nat) {
        if(fee > 0) {
            _transfer(from, feeTo, fee);
        };
    };

    private func _transfer(from: Principal, to: Principal, value: Nat) {
        let from_balance = _balanceOf(from);
        let from_balance_new : Nat = from_balance - value;
        if (from_balance_new != 0) { balances.put(from, from_balance_new); }
        else { balances.delete(from); };

        let to_balance = _balanceOf(to);
        let to_balance_new : Nat = to_balance + value;
        if (to_balance_new != 0) { balances.put(to, to_balance_new); };

        _moveDelegates(?from, ?to, value, fee);
    };

    private func _balanceOf(who: Principal) : Nat {
        switch (balances.get(who)) {
            case (?balance) { return balance; };
            case (_) { return 0; };
        }
    };

    private func _allowance(owner: Principal, spender: Principal) : Nat {
        switch(allowances.get(owner)) {
            case (?allowance_owner) {
                switch(allowance_owner.get(spender)) {
                    case (?allowance) { return allowance; };
                    case (_) { return 0; };
                }
            };
            case (_) { return 0; };
        }
    };

    /*
    *   Core interfaces:
    *       update calls:
    *           transfer/transferFrom/approve
    *       query calls:
    *           logo/name/symbol/decimal/totalSupply/balanceOf/allowance/getMetadata
    *           historySize/getTransaction/getTransactions
    */

    /// Transfers value amount of tokens to Principal to.
    public shared(msg) func transfer(to: Principal, value: Nat) : async TxReceipt {
        if (_balanceOf(msg.caller) < value + fee) { return #Err(#InsufficientBalance); };
        _chargeFee(msg.caller, fee);
        _transfer(msg.caller, to, value);
        let txid = addRecord(null, #transfer, msg.caller, to, value, fee, Time.now(), #succeeded);
        return #Ok(txid);
    };

    /// Transfers value amount of tokens from Principal from to Principal to.
    public shared(msg) func transferFrom(from: Principal, to: Principal, value: Nat) : async TxReceipt {
        if (_balanceOf(from) < value + fee) { return #Err(#InsufficientBalance); };
        let allowed : Nat = _allowance(from, msg.caller);
        if (allowed < value + fee) { return #Err(#InsufficientAllowance); };
        _chargeFee(from, fee);
        _transfer(from, to, value);
        let allowed_new : Nat = allowed - value - fee;
        if (allowed_new != 0) {
            let allowance_from = Types.unwrap(allowances.get(from));
            allowance_from.put(msg.caller, allowed_new);
            allowances.put(from, allowance_from);
        } else {
            if (allowed != 0) {
                let allowance_from = Types.unwrap(allowances.get(from));
                allowance_from.delete(msg.caller);
                if (allowance_from.size() == 0) { allowances.delete(from); }
                else { allowances.put(from, allowance_from); };
            };
        };
        let txid = addRecord(?msg.caller, #transferFrom, from, to, value, fee, Time.now(), #succeeded);
        return #Ok(txid);
    };

    /// Allows spender to withdraw from your account multiple times, up to the value amount.
    /// If this function is called again it overwrites the current allowance with value.
    public shared(msg) func approve(spender: Principal, value: Nat) : async TxReceipt {
        if(_balanceOf(msg.caller) < fee) { return #Err(#InsufficientBalance); };
        _chargeFee(msg.caller, fee);
        let v = value + fee;
        if (value == 0 and Option.isSome(allowances.get(msg.caller))) {
            let allowance_caller = Types.unwrap(allowances.get(msg.caller));
            allowance_caller.delete(spender);
            if (allowance_caller.size() == 0) { allowances.delete(msg.caller); }
            else { allowances.put(msg.caller, allowance_caller); };
        } else if (value != 0 and Option.isNull(allowances.get(msg.caller))) {
            var temp = HashMap.HashMap<Principal, Nat>(1, Principal.equal, Principal.hash);
            temp.put(spender, v);
            allowances.put(msg.caller, temp);
        } else if (value != 0 and Option.isSome(allowances.get(msg.caller))) {
            let allowance_caller = Types.unwrap(allowances.get(msg.caller));
            allowance_caller.put(spender, v);
            allowances.put(msg.caller, allowance_caller);
        };
        let txid = addRecord(null, #approve, msg.caller, spender, v, fee, Time.now(), #succeeded);
        return #Ok(txid);
    };

    public shared(msg) func mint(to: Principal, amount: Nat): async TxReceipt {
        if(msg.caller != owner_) {
            return #Err(#Unauthorized);
        };
        let to_balance = _balanceOf(to);
        totalSupply_ += amount;
        balances.put(to, to_balance + amount);
        let txid = addRecord(?msg.caller, #mint, blackhole, to, amount, 0, Time.now(), #succeeded);
        return #Ok(txid);
    };

    public shared(msg) func burn(amount: Nat): async TxReceipt {
        let from_balance = _balanceOf(msg.caller);
        if(from_balance < amount) {
            return #Err(#InsufficientBalance);
        };
        totalSupply_ -= amount;
        balances.put(msg.caller, from_balance - amount);
        let txid = addRecord(?msg.caller, #burn, msg.caller, blackhole, amount, 0, Time.now(), #succeeded);
        return #Ok(txid);
    };

    public query func logo() : async Text {
        return logo_;
    };

    public query func name() : async Text {
        return name_;
    };

    public query func symbol() : async Text {
        return symbol_;
    };

    public query func decimals() : async Nat8 {
        return decimals_;
    };

    public query func totalSupply() : async Nat {
        return totalSupply_;
    };

    public query func getTokenFee() : async Nat {
        return fee;
    };

    public query func balanceOf(who: Principal) : async Nat {
        return _balanceOf(who);
    };

    public query func allowance(owner: Principal, spender: Principal) : async Nat {
        return _allowance(owner, spender);
    };

    public query func getMetadata() : async Metadata {
        return {
            logo = logo_;
            name = name_;
            symbol = symbol_;
            decimals = decimals_;
            totalSupply = totalSupply_;
            owner = owner_;
            fee = fee;
        };
    };

    /// Get transaction history size
    public query func historySize() : async Nat {
        return ops.size();
    };

    /// Get transaction by index.
    public query func getTransaction(index: Nat) : async TxRecord {
        return ops[index];
    };

    /// Get history
    public query func getTransactions(start: Nat, limit: Nat) : async [TxRecord] {
        var ret: [TxRecord] = [];
        var i = start;
        while(i < start + limit and i < ops.size()) {
            ret := Array.append(ret, [ops[i]]);
            i += 1;
        };
        return ret;
    };

    /*
    *   Optional interfaces:
    *       setName/setLogo/setFee/setFeeTo/setOwner
    *       getUserTransactionsAmount/getUserTransactions
    *       getTokenInfo/getHolders/getUserApprovals
    */
    public shared(msg) func setName(name: Text) {
        assert(msg.caller == owner_);
        name_ := name;
    };

    public shared(msg) func setLogo(logo: Text) {
        assert(msg.caller == owner_);
        logo_ := logo;
    };

    public shared(msg) func setFeeTo(to: Principal) {
        assert(msg.caller == owner_);
        feeTo := to;
    };

    public shared(msg) func setFee(_fee: Nat) {
        assert(msg.caller == owner_);
        fee := _fee;
    };

    public shared(msg) func setOwner(_owner: Principal) {
        assert(msg.caller == owner_);
        owner_ := _owner;
    };

    public query func getUserTransactionAmount(a: Principal) : async Nat {
        var res: Nat = 0;
        for (i in ops.vals()) {
            if (i.caller == ?a or i.from == a or i.to == a) {
                res += 1;
            };
        };
        return res;
    };

    public query func getUserTransactions(a: Principal, start: Nat, limit: Nat) : async [TxRecord] {
        var res: [TxRecord] = [];
        var index: Nat = 0;
        for (i in ops.vals()) {
            if (i.caller == ?a or i.from == a or i.to == a) {
                if(index >= start and index < start + limit) {
                    res := Array.append<TxRecord>(res, [i]);
                };
                index += 1;
            };
        };
        return res;
    };

    public type TokenInfo = {
        metadata: Metadata;
        feeTo: Principal;
        // status info
        historySize: Nat;
        deployTime: Time.Time;
        holderNumber: Nat;
        cycles: Nat;
    };
    public query func getTokenInfo(): async TokenInfo {
        {
            metadata = {
                logo = logo_;
                name = name_;
                symbol = symbol_;
                decimals = decimals_;
                totalSupply = totalSupply_;
                owner = owner_;
                fee = fee;
            };
            feeTo = feeTo;
            historySize = ops.size();
            deployTime = genesis.timestamp;
            holderNumber = balances.size();
            cycles = ExperimentalCycles.balance();
        }
    };

    public query func getHolders(start: Nat, limit: Nat) : async [(Principal, Nat)] {
        let temp =  Iter.toArray(balances.entries());
        func order (a: (Principal, Nat), b: (Principal, Nat)) : Order.Order {
            return Nat.compare(b.1, a.1);
        };
        let sorted = Array.sort(temp, order);
        let limit_: Nat = if(start + limit > temp.size()) {
            temp.size() - start
        } else {
            limit
        };
        let res = Array.init<(Principal, Nat)>(limit_, (owner_, 0));
        for (i in Iter.range(0, limit_ - 1)) {
            res[i] := sorted[i+start];
        };
        return Array.freeze(res);
    };

    public query func getAllowanceSize() : async Nat {
        var size : Nat = 0;
        for ((k, v) in allowances.entries()) {
            size += v.size();
        };
        return size;
    };

    public query func getUserApprovals(who : Principal) : async [(Principal, Nat)] {
        switch (allowances.get(who)) {
            case (?allowance_who) {
                return Iter.toArray(allowance_who.entries());
            };
            case (_) {
                return [];
            };
        }
    };

    /**
     * Delegation interfaces:
     *  update calls:
     *      delegate
     *  query calls:
     *      getCurrentVotes/getPriorVotes
     */

    /// delegates votes from `msg.caller` to `delegatee`
    public shared(msg) func delegate(delegatee: Principal) : async TxReceipt {
        if (_balanceOf(msg.caller) == 0) { return #Err(#InsufficientBalance); };
        let value = _delegate(msg.caller, delegatee);
        let txid = addRecord(?msg.caller, #delegate, msg.caller, delegatee, value, fee, Time.now(), #succeeded);
        return #Ok(txid);
    };

    /// gets the current votes balance for `who`
    public query func getCurrentVotes(who: Principal) : async Nat {
        _getVotes(who)
    };

    /// gets the prior number of votes for an account before timestamp
    public query func getPriorVotes(who: Principal, timestamp: Time.Time) : async Nat {
        let accountCheckPoints = switch(checkPoints.get(who)) {
            case (?cp) { cp };
            case (_) { return 0; };
        };
        let currentCheckPoint = accountCheckPoints.get(accountCheckPoints.size() - 1);
        if (currentCheckPoint.timestamp <= timestamp) {
            return currentCheckPoint.votes;
        };
        let oldestCheckPoint = accountCheckPoints.get(0);
        if (oldestCheckPoint.timestamp > timestamp) {
            return oldestCheckPoint.votes;
        };

        // binary search
        var lower = 0;
        var upper = accountCheckPoints.size() - 1;
        while (lower > upper) {
            let center = upper - (upper - lower) / 2;
            let cp = accountCheckPoints.get(center);
            if (cp.timestamp == timestamp) {
                return cp.votes;
            } else if (cp.timestamp < timestamp) {
                lower := center;
            } else {
                upper := center - 1;
            };
        };
        return accountCheckPoints.get(lower).votes;
    };

    private func _delegate(delegator: Principal, delegatee: Principal) : Nat {
        let currentDelegate = delegates.get(delegator);
        let delegatorBalance = _balanceOf(delegator);

        delegates.put(delegator, delegatee);
        _moveDelegates(currentDelegate, ?delegatee, delegatorBalance, 0);

        delegatorBalance
    };    

    private func _moveDelegates(from: ?Principal, to: ?Principal, amount: Nat, fee: Nat) {
        if (amount > 0) {
            if (Option.isSome(from)) {
                let from_ = Option.get(from, blackhole);
                let fromDelegatesOld = _getVotes(from_);
                let fromDelegatesNew = fromDelegatesOld - amount - fee;
                _writeCheckPoint(from_, fromDelegatesNew);
            };
            if (Option.isSome(to)) {
                let to_ = Option.get(to, blackhole);
                let toDelegatesOld = _getVotes(to_);
                let toDelegatesNew = toDelegatesOld + amount;
                _writeCheckPoint(to_, toDelegatesNew);
            };
        };
    };

    private func _writeCheckPoint(who: Principal, newVotes: Nat) {
        let checkPoint = switch (checkPoints.get(who)) {
            case (?cp) {
                cp
            };
            case (_) {
                Buffer.Buffer<CheckPoint>(1);
            };
        };
        let size = checkPoint.size();
        let timestamp = Time.now();
        if (size > 0 and checkPoint.get(size - 1).timestamp == timestamp) {
            ignore checkPoint.removeLast();
        };
        checkPoint.add(_newCheckPoint(Time.now(), newVotes));
        checkPoints.put(who, checkPoint);
    };

    private func _getVotes(who: Principal) : Nat {
        switch(checkPoints.get(who)) {
            case (?checkPoint) {
                checkPoint.get(checkPoint.size() - 1).votes
            };
            case (_) { 0 }
        };
    };

    /*
    * upgrade functions
    */
    system func preupgrade() {
        balanceEntries := Iter.toArray(balances.entries());
        var size : Nat = allowances.size();
        var temp : [var (Principal, [(Principal, Nat)])] = Array.init<(Principal, [(Principal, Nat)])>(size, (owner_, []));
        size := 0;
        for ((k, v) in allowances.entries()) {
            temp[size] := (k, Iter.toArray(v.entries()));
            size += 1;
        };
        allowanceEntries := Array.freeze(temp);

        delegateEntries := Iter.toArray(delegates.entries());

        size := checkPoints.size();
        let buf = Buffer.Buffer<(Principal, [CheckPoint])>(size);
        for ((k, v) in checkPoints.entries()) {
            buf.add((k, v.toArray()));
        };
        checkPointEntries := buf.toArray();
    };

    system func postupgrade() {
        balances := HashMap.fromIter<Principal, Nat>(balanceEntries.vals(), 1, Principal.equal, Principal.hash);
        balanceEntries := [];
        for ((k, v) in allowanceEntries.vals()) {
            let allowed_temp = HashMap.fromIter<Principal, Nat>(v.vals(), 1, Principal.equal, Principal.hash);
            allowances.put(k, allowed_temp);
        };
        allowanceEntries := [];

        delegates := HashMap.fromIter<Principal, Principal>(delegateEntries.vals(), 1, Principal.equal, Principal.hash);
        delegateEntries := [];

        for ((k, v) in checkPointEntries.vals()) {
            let cps = Buffer.Buffer<CheckPoint>(v.size());
            for (cp in v.vals()) {
                cps.add(cp);
            };
            checkPoints.put(k, cps);
        };
        checkPointEntries := [];
    };
};
