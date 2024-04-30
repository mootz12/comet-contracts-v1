use contracts::c_pool::comet::{CometPoolContract, CometPoolContractClient};
use factory::FactoryClient;
use sep_41_token::testutils::MockTokenClient;
use soroban_sdk::{testutils::{Address as _, Ledger as _, LedgerInfo}, vec, Address, Env};

pub const ONE_DAY_LEDGERS: u32 = 17280;

pub struct TestFixture<'a> {
    pub env: Env,
    pub bombadil: Address,
    pub samwise: Address,
    pub frodo: Address,
    pub token_1: MockTokenClient<'a>,
    pub token_2: MockTokenClient<'a>,
    pub pool: CometPoolContractClient<'a>,
}

impl TestFixture<'_> {
    pub fn create<'a>(weight_1: i128, weight_2: i128, balance_1: i128, balance_2: i128, swap_fee: i128) -> TestFixture<'a> {
        let env = Env::default();
        env.mock_all_auths();
        env.budget().reset_unlimited();
        env.ledger().set(LedgerInfo {
            timestamp: 1441065600, // Sept 1st, 2015 12:00:00 AM UTC
            protocol_version: 20,
            sequence_number: 100,
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 30 * ONE_DAY_LEDGERS,
            min_persistent_entry_ttl: 120 * ONE_DAY_LEDGERS,
            max_entry_ttl: 365 * ONE_DAY_LEDGERS,
        });
        let bombadil = Address::generate(&env);
        let samwise = Address::generate(&env);
        let frodo = Address::generate(&env);

        // Create two tokens
        let token_1_id = env.register_stellar_asset_contract(bombadil.clone());
        let token_1 = MockTokenClient::new(&env, &token_1_id); 
        let token_2_id = env.register_stellar_asset_contract(bombadil.clone());
        let token_2 = MockTokenClient::new(&env, &token_2_id);

        // create 2 token comet pool
        let comet_id = env.register_contract(None, CometPoolContract {});
        let pool = CometPoolContractClient::new(&env, &comet_id);
        token_1.mint(&bombadil, &balance_1);
        token_2.mint(&bombadil, &balance_2);
        let tokens = vec![&env, token_1_id, token_2_id];
        let weights = vec![&env, weight_1, weight_2];
        let balances = vec![&env, balance_1, balance_2];
        pool.init(&bombadil, &tokens, &weights, &balances, &swap_fee);

        TestFixture {
            env,
            bombadil,
            samwise,
            frodo,
            token_1,
            token_2,
            pool,
        }
    }

    pub fn jump(&self, ledgers: u32) {
        self.env.ledger().set(LedgerInfo {
            timestamp: self.env.ledger().timestamp().saturating_add(ledgers as u64 * 5),
            protocol_version: 20,
            sequence_number: self.env.ledger().sequence().saturating_add(ledgers),
            network_id: Default::default(),
            base_reserve: 10,
            min_temp_entry_ttl: 50 * ONE_DAY_LEDGERS,
            min_persistent_entry_ttl: 50 * ONE_DAY_LEDGERS,
            max_entry_ttl: 365 * ONE_DAY_LEDGERS,
        });
    }

    pub fn calc_invariant(&self) -> f64 {
        let token_1_balance = self.token_1.balance(&self.pool.address);
        let token_2_balance = self.token_2.balance(&self.pool.address);
        let token_1_weight = self.pool.get_normalized_weight(&self.token_1.address);
        let token_2_weight = self.pool.get_normalized_weight(&self.token_2.address);

        let t1_b = token_1_balance as f64 / 1e7;
        let t2_b = token_2_balance as f64 / 1e7;
        let t1_w = token_1_weight as f64 / 1e7;
        let t2_w = token_2_weight as f64 / 1e7;

        t1_b.powf(t1_w) * t2_b.powf(t2_w) 
    }
}