use sov_bank::{create_token_address, BankConfig};
use sov_modules_api::default_signature::private_key;
use sov_swap::call::CallMessage;
use sov_swap::{SwapModule, SwapModuleConfig};
use sov_modules_api::default_context::DefaultContext;
use sov_modules_api::{Address, Context, Hasher, Module, ModuleInfo, Spec};
use sov_rollup_interface::stf::Event;
use sov_state::{DefaultStorageSpec, ProverStorage, WorkingSet};

pub type C = DefaultContext;
pub type Storage = ProverStorage<DefaultStorageSpec>;

pub fn generate_address(key: &str) -> <C as Spec>::Address {
    let hash = <C as Spec>::Hasher::hash(key.as_bytes());
    Address::from(hash)
}

#[test]
fn create_pool() {
    // Preparation
    let token_a = generate_address("token_a");
    let token_b = generate_address("token_b");
    let config: SwapModuleConfig<C> = SwapModuleConfig {
        pools: vec![],
    };
    let mut working_set = WorkingSet::new(ProverStorage::temporary());
    let swap = SwapModule::new();

    // Genesis
    let genesis_result = swap.genesis(&config, &mut working_set);
    assert!(genesis_result.is_ok());

    // Create Pool
    let create_pool_message = CallMessage::<C>::CreatePool {
        token_a: token_a.clone(),
        token_b: token_b.clone(),
    };

    let create_pool_result = swap.call(
        create_pool_message.clone(),
        &C::new(token_a.clone()),
        &mut working_set,
    );

    assert!(create_pool_result.is_ok());

    let pool_id = generate_address(&format!("{:?}{:?}", token_a, token_b));

    let liquidity_token = create_token_address::<C>(
        &format!("LP:{}_{}", &token_a, &token_b),
        swap.address().as_ref(),
        0,
    );

    assert_eq!(
        working_set.events()[0],
        Event::new("create_pool", &format!(
            "pool_id: {pool_id:?}, token_a: {token_a:?}, token_b: {token_b:?}, liquidity_token: {liquidity_token:?}",
            pool_id = pool_id,
            token_a = token_a,
            token_b = token_b,
            liquidity_token = liquidity_token,
        ))
    );


    // Attempt to create pool with the same tokens
    let create_pool_result = swap.call(
        create_pool_message.clone(),
        &C::new(token_a.clone()),
        &mut working_set,
    );

    assert!(create_pool_result.is_err());
    let error_message = create_pool_result.err().unwrap().to_string();
    assert_eq!("Pool already exists", error_message);
}

#[test]
fn add_liquidity() {
    // Preparation
    let bank = sov_bank::Bank::<C>::new();
    let empty_bank_config = BankConfig::<C> { tokens: vec![] };
    let mut working_set = WorkingSet::new(ProverStorage::temporary());
    bank.genesis(&empty_bank_config, &mut working_set).unwrap();

    let swap = SwapModule::new();
    let config: SwapModuleConfig<C> = SwapModuleConfig {
        pools: vec![],
    };
    swap.genesis(&config, &mut working_set).unwrap();

    // create private key
    let private_key = private_key::DefaultPrivateKey::generate();

    // create tokens
    let create_token_a_message = sov_bank::call::CallMessage::CreateToken {
        salt: 0,
        token_name: "TokenA".to_owned(),
        initial_balance: 100,
        minter_address: private_key.default_address(),
        authorized_minters: vec![private_key.default_address()],
    };
    let token_a_response = bank.call(
        create_token_a_message.clone(),
        &C::new(private_key.default_address()),
        &mut working_set,
    );
    assert!(token_a_response.is_ok());
    // generate token address
    let token_a_address = create_token_address::<C>(
        &"TokenA".to_owned(),
        private_key.default_address().as_ref(),
        0,
    );
    
    let create_token_b_message = sov_bank::call::CallMessage::CreateToken {
        salt: 0,
        token_name: "TokenB".to_owned(),
        initial_balance: 100,
        minter_address: private_key.default_address(),
        authorized_minters: vec![private_key.default_address()],
    };
    let token_b_response = bank.call(
        create_token_b_message.clone(),
        &C::new(private_key.default_address()),
        &mut working_set,
    );
    assert!(token_b_response.is_ok());
    // generate token address
    let token_b_address = create_token_address::<C>(
        &"TokenB".to_owned(),
        private_key.default_address().as_ref(),
        0,
    );

    // Create Pool
    let create_pool_message = CallMessage::<C>::CreatePool {
        token_a: token_a_address.clone(),
        token_b: token_b_address.clone(),
    };

    let create_pool_result = swap.call(
        create_pool_message.clone(),
        &C::new(token_a_address.clone()),
        &mut working_set,
    );

    assert!(create_pool_result.is_ok());

    let pool_id = generate_address(&format!("{:?}{:?}", token_a_address, token_b_address));

    let liquidity_token = create_token_address::<C>(
        &format!("LP:{}_{}", &token_a_address, &token_b_address),
        swap.address().as_ref(),
        0,
    );

    assert_eq!(
        working_set.events()[0],
        Event::new("create_pool", &format!(
            "pool_id: {pool_id:?}, token_a: {token_a:?}, token_b: {token_b:?}, liquidity_token: {liquidity_token:?}",
            pool_id = pool_id,
            token_a = token_a_address,
            token_b = token_b_address,
            liquidity_token = liquidity_token,
        ))
    );

    // Add liquidity
    let add_liquidity_message = CallMessage::<C>::AddLiquidity {
        pool_id: pool_id.clone(),
        token_a_amount: 100,
        token_b_amount: 50,
    };

    let add_liquidity_result = swap.call(
        add_liquidity_message.clone(),
        &C::new(private_key.default_address().clone()),
        &mut working_set,
    );

    // print error
    if add_liquidity_result.is_err() {
        println!("add_liquidity_result: {:?}", add_liquidity_result.err().unwrap().to_string());
    }

    assert_eq!(
        working_set.events()[1],
        Event::new("add_liquidity", &format!("pool_id: {pool_id:?}"))
    );

    // get liquidity_token balance
    let liquidity_token_balance = bank.balance_of(
        private_key.default_address().clone(),
        liquidity_token.clone(),
        &mut working_set,
    );
    let liquidity_token_balance = liquidity_token_balance.amount.unwrap_or(0);

    assert_eq!(
        liquidity_token_balance,
        ((100 * 50) as f64).sqrt() as u64
    );
}

#[test]
fn swap() {
    // Preparation
    let bank = sov_bank::Bank::new();
    let empty_bank_config = BankConfig { tokens: vec![] };
    let mut working_set = WorkingSet::new(ProverStorage::temporary());
    bank.genesis(&empty_bank_config, &mut working_set).unwrap();

    let swap = SwapModule::new();
    let config = SwapModuleConfig {
        pools: vec![],
    };
    swap.genesis(&config, &mut working_set).unwrap();
    
    
    // create private key
    let private_key = private_key::DefaultPrivateKey::generate();
    
    // create tokens
    let create_token_a_message = sov_bank::call::CallMessage::CreateToken {
        salt: 0,
        token_name: "TokenA".to_owned(),
        initial_balance: 1000,
        minter_address: private_key.default_address(),
        authorized_minters: vec![private_key.default_address()],
    };
    let token_a_response = bank.call(
        create_token_a_message.clone(),
        &C::new(private_key.default_address()),
        &mut working_set,
    );
    assert!(token_a_response.is_ok());
    // generate token address
    let token_a_address = create_token_address::<C>(
        &"TokenA".to_owned(),
        private_key.default_address().as_ref(),
        0,
    );
    
    let create_token_b_message = sov_bank::call::CallMessage::CreateToken {
        salt: 0,
        token_name: "TokenB".to_owned(),
        initial_balance: 1000,
        minter_address: private_key.default_address(),
        authorized_minters: vec![private_key.default_address()],
    };
    let token_b_response = bank.call(
        create_token_b_message.clone(),
        &C::new(private_key.default_address()),
        &mut working_set,
    );
    assert!(token_b_response.is_ok());
    // generate token address
    let token_b_address = create_token_address::<C>(
        &"TokenB".to_owned(),
        private_key.default_address().as_ref(),
        0,
    );

    // Create Pool
    let create_pool_message = CallMessage::<C>::CreatePool {
        token_a: token_a_address.clone(),
        token_b: token_b_address.clone(),
    };

    let create_pool_result = swap.call(
        create_pool_message.clone(),
        &C::new(private_key.default_address().clone()),
        &mut working_set,
    );

    assert!(create_pool_result.is_ok());

    let pool_id = generate_address(&format!("{:?}{:?}", token_a_address, token_b_address));

    assert_eq!(
        working_set.events()[0],
        Event::new("create_pool", &format!("pool_id: {pool_id:?}"))
    );

    // Add liquidity
    let add_liquidity_message = CallMessage::<C>::AddLiquidity {
        pool_id: pool_id.clone(),
        token_a_amount: 100,
        token_b_amount: 200,
    };

    let add_liquidity_result = swap.call(
        add_liquidity_message.clone(),
        &C::new(private_key.default_address().clone()),
        &mut working_set,
    );

    assert_eq!(
        working_set.events()[1],
        Event::new("add_liquidity", &format!("pool_id: {pool_id:?}"))
    );

    // Swap
    let swap_message = CallMessage::<C>::Swap {
        pool_id: pool_id.clone(),
        token_a_amount: 100,
        token_b_amount: 0,
    };

    let swap_result = swap.call(
        swap_message.clone(),
        &C::new(private_key.default_address().clone()),
        &mut working_set,
    );

    assert_eq!(
        working_set.events()[2],
        Event::new(
            "swap",
            &format!(
                "pool_id: {pool_id:?}, sender: {sender}, send_token: {send_token:?}, send_amount: {send_amount:?}, receive_token: {receive_token:?}, receive_amount: {receive_amount:?}",
                pool_id = pool_id.clone(),
                sender = private_key.default_address().clone(),
                send_token = token_a_address.clone(),
                send_amount = 100,
                receive_token = token_b_address.clone(),
                receive_amount = 200,
            ),
        )
    );

    // TODO: assert balances
    {
        /* let balance = bank.get_balance_of(
            private_key.default_address().clone(),
            token_a_address.clone(),
            &mut working_set,
        ).unwrap();
        println!("balance: {:?}", balance); */
    }

    // print error
    if swap_result.is_err() {
        println!("swap_result: {:?}", swap_result.err().unwrap().to_string());
    }

}