use casper_engine_test_support::{
    ExecuteRequestBuilder, InMemoryWasmTestBuilder, DEFAULT_ACCOUNT_ADDR,
    MINIMUM_ACCOUNT_CREATION_BALANCE, PRODUCTION_RUN_GENESIS_REQUEST,
};
use casper_types::{
    account::AccountHash,
    bytesrepr::{Bytes, FromBytes, ToBytes},
    contracts::NamedKeys,
    runtime_args,
    system::mint,
    testing::TestRng,
    CLTyped, ContractHash, ContractPackageHash, HashAddr, Key, PublicKey, RuntimeArgs, SecretKey,
    URef, SECP256K1_TAG, U256, U512,
};

use crate::gas;

/// The key under which the events are stored.
pub const EVENTS_DICT: &str = "__events";
/// The key under which the events length is stored.
pub const EVENTS_LENGTH: &str = "__events_length";
/// The key under which the event schemas are stored.
pub const EVENTS_SCHEMA: &str = "__events_schema";

pub struct TestEnv {
    pub builder: InMemoryWasmTestBuilder,
    pub accounts: Vec<AccountHash>,
    pub block_time: u64,
}

pub fn generate_random_account(curve_tag: u8) -> AccountHash {
    if curve_tag == SECP256K1_TAG {
        let mut key_bytes = HashAddr::default();
        for item in &mut key_bytes {
            *item = rand::random::<u8>();
        }
        let sk: SecretKey = SecretKey::secp256k1_from_bytes(key_bytes).unwrap();
        let pk: PublicKey = PublicKey::from(&sk);
        let a: AccountHash = pk.to_account_hash();
        a
    } else {
        let sk = SecretKey::random_ed25519(&mut TestRng::new());
        let pk: PublicKey = PublicKey::from(&sk);
        let a: AccountHash = pk.to_account_hash();
        a
    }
}

impl TestEnv {
    pub fn new(accounts: &[AccountHash], genesis_block_time: u64) -> Self {
        let mut builder = InMemoryWasmTestBuilder::default();
        builder.run_genesis(&PRODUCTION_RUN_GENESIS_REQUEST);
        let mut test_env = TestEnv {
            builder,
            accounts: accounts.to_vec(),
            block_time: genesis_block_time,
        };
        for acc in accounts {
            test_env.fund_account(*acc);
        }

        test_env
    }

    pub fn get_account(self, n: usize) -> AccountHash {
        *self.accounts.get(n).unwrap()
    }

    pub fn fund_account(&mut self, account: AccountHash) {
        let id: Option<u64> = None;

        let transfer_1_args = runtime_args! {
            mint::ARG_TARGET => account,
            mint::ARG_AMOUNT => MINIMUM_ACCOUNT_CREATION_BALANCE,
            mint::ARG_ID => id,
        };
        let transfer_request_1 =
            ExecuteRequestBuilder::transfer(*DEFAULT_ACCOUNT_ADDR, transfer_1_args).build();
        self.builder
            .exec(transfer_request_1)
            .expect_success()
            .commit();
    }

    pub fn deploy_contract(
        &mut self,
        deployer: Option<AccountHash>,
        wasm_path: &str,
        args: RuntimeArgs,
    ) -> U512 {
        let deployer = deployer.unwrap_or(*DEFAULT_ACCOUNT_ADDR);
        let install_request_1 = ExecuteRequestBuilder::standard(deployer, wasm_path, args.clone())
            .with_block_time(self.block_time * 1000)
            .build();
        self.builder
            .exec(install_request_1)
            .expect_success()
            .commit();
        let gas_cost = self.last_call_contract_gas_cost();
        if args.get("entry_point").is_some() {
            let entry_name: String =
                String::from_bytes(&args.get("entry_point").unwrap().clone().to_bytes().unwrap())
                    .unwrap()
                    .0;
            gas::write_to(false, &entry_name, self.builder.last_exec_gas_cost());
        } else {
            gas::write_to(true, wasm_path, self.builder.last_exec_gas_cost());
        }
        gas_cost
    }

    pub fn call_contract(
        &mut self,
        caller: Option<AccountHash>,
        contract_package_hash: ContractPackageHash,
        fun_name: &str,
        args: RuntimeArgs,
        expect_success: bool,
    ) -> U512 {
        let request = ExecuteRequestBuilder::versioned_contract_call_by_hash(
            caller.unwrap_or(*DEFAULT_ACCOUNT_ADDR),
            contract_package_hash,
            None,
            fun_name,
            args,
        )
        .with_block_time(self.block_time * 1000)
        .build();
        if expect_success {
            self.builder.exec(request).expect_success().commit();
        } else {
            self.builder.exec(request).expect_failure();
        }
        let gas_cost = self.last_call_contract_gas_cost();
        gas::write_to(false, fun_name, self.builder.last_exec_gas_cost());
        gas_cost
    }

    pub fn get_contract_package_hash(&self, owner: AccountHash, key_name: &str) -> Key {
        let named_keys = self.get_named_keys(owner);

        let ret = named_keys
            .get(key_name)
            .expect("should have contract package hash");
        *ret
    }

    pub fn get_contract_hash(&self, owner: AccountHash, key_name: &str) -> Key {
        let named_keys = self.get_named_keys(owner);
        let ret = named_keys
            .get(key_name)
            .expect("should have contract package hash");
        *ret
    }

    pub fn get_named_keys(&self, owner: AccountHash) -> NamedKeys {
        let account = self
            .builder
            .get_account(owner)
            .expect("should have account");
        account.named_keys().clone()
    }

    pub fn last_call_contract_gas_cost(&self) -> U512 {
        self.builder.last_exec_gas_cost().value()
    }

    pub fn advance_block_time_by(&mut self, seconds: u64) {
        self.block_time += seconds;
    }

    pub fn set_block_time(&mut self, seconds: u64) {
        self.block_time = seconds;
    }

    pub fn get_account_cspr_balance(&self, account: AccountHash) -> U512 {
        let account = self.builder.get_account(account).unwrap();
        let purse = account.main_purse();
        self.get_balance_by_uref(purse)
    }

    pub fn get_balance_by_uref(&self, purse: URef) -> U512 {
        self.builder.get_purse_balance(purse)
    }

    pub fn get_default_account(&self) -> AccountHash {
        *DEFAULT_ACCOUNT_ADDR
    }

    pub fn call_view_function<T: FromBytes + CLTyped>(
        &mut self,
        package_hash: Key,
        fun_name: &str,
        args: RuntimeArgs,
    ) -> T {
        self.deploy_contract(
            Some(*DEFAULT_ACCOUNT_ADDR),
            "get-session.wasm",
            runtime_args! {
                "contract_package_hash" => package_hash,
                "entry_point" => fun_name,
                "args" => Bytes::from(args.to_bytes().unwrap())
            },
        );
        self.get_test_result_with_name()
    }

    pub fn get_test_result_with_name<T: FromBytes + CLTyped>(&mut self) -> T {
        let named_keys = self.get_named_keys(*DEFAULT_ACCOUNT_ADDR);
        let ret = named_keys.get("result").expect("should have result");
        let b = self
            .builder
            .query(None, *ret, &[])
            .unwrap()
            .as_cl_value()
            .unwrap()
            .clone()
            .into_t::<Bytes>()
            .unwrap();
        T::from_bytes(b.as_slice()).unwrap().0
    }

    pub fn get_named_key_value<T: FromBytes + CLTyped>(
        &mut self,
        contract_package_hash: Key,
        key_name: &str,
    ) -> T {
        let contract_hash = self.get_active_contract_hash(contract_package_hash);

        self.builder.get_value(contract_hash, key_name)
    }

    pub fn get_active_contract_hash(&mut self, package_hash: Key) -> ContractHash {
        let contract_package_hash: ContractPackageHash = package_hash.into_hash().unwrap().into();
        let contract_package = self
            .builder
            .get_contract_package(contract_package_hash)
            .unwrap();
        let enabled_versions = contract_package.enabled_versions();
        let (_version, contract_hash) = enabled_versions
            .iter()
            .rev()
            .next()
            .expect("should have latest version");
        contract_hash.clone()
    }

    pub fn approve(&mut self, token: Key, owner: AccountHash, spender: Key, amount: U256) {
        self.call_contract(
            Some(owner),
            token.into_hash().unwrap().into(),
            "approve",
            runtime_args! {
                "spender" => spender,
                "amount" => amount
            },
            true,
        );
    }

    pub fn transfer(&mut self, token: Key, owner: AccountHash, recipient: Key, amount: U256) {
        self.call_contract(
            Some(owner),
            token.into_hash().unwrap().into(),
            "transfer",
            runtime_args! {
                "recipient" => recipient,
                "amount" => amount
            },
            true,
        );
    }

    pub fn balance_of(&mut self, token: Key, user: Key) -> U256 {
        self.call_view_function(
            token,
            "balance_of",
            runtime_args! {
                "address" => user
            },
        )
    }

    pub fn get_event_length(&mut self, contract_package: Key) -> u32 {
        self.get_named_key_value(contract_package, EVENTS_LENGTH)
    }

    pub fn get_last_event<T: FromBytes>(&mut self, contract_package: Key) -> Option<T> {
        let events_length: u32 = self.get_event_length(contract_package);
        self.get_event(contract_package, events_length as usize - 1)
    }

    pub fn get_event<T: FromBytes>(&mut self, contract_package: Key, event_position: usize) -> Option<T> {
        let contract_hash: ContractHash = self.get_active_contract_hash(contract_package);

        let dictionary_seed_uref: URef = *self
            .builder
            .get_contract(contract_hash)
            .unwrap()
            .named_keys()
            .get(EVENTS_DICT)
            .unwrap()
            .as_uref()
            .unwrap();

        match self.builder.query_dictionary_item(
            None,
            dictionary_seed_uref,
            &event_position.to_string(),
        ) {
            Ok(val) => {
                let bytes = val
                    .as_cl_value()
                    .unwrap()
                    .clone()
                    .into_t::<Bytes>()
                    .unwrap();
                let value: T = T::from_bytes(bytes.as_slice()).unwrap().0;
                Some(value)
            }
            Err(_) => None,
        }
    }
}

impl Default for TestEnv {
    fn default() -> Self {
        TestEnv::new(&[], 0)
    }
}
