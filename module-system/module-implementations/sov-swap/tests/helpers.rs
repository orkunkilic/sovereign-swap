use anyhow::{Error, bail};
use sov_bank::create_token_address;
use sov_modules_api::{Address, Module, Context, CallResponse};
use sov_modules_api::default_context::DefaultContext;
use sov_state::{ProverStorage, DefaultStorageSpec, WorkingSet};
use sov_swap::call::generate_address;

type C = DefaultContext;
type Storage = ProverStorage<DefaultStorageSpec>;

pub fn create_token_helper(
    bank: sov_bank::Bank<C>,
    working_set: &mut sov_state::WorkingSet<Storage>,
    context: &C,
    token_name: String,
    initial_balance: u64,
    minter_address: Address,
    authorized_minters: Vec<Address>,
) -> Result<Address, Error> {
    let create_token_message = sov_bank::call::CallMessage::CreateToken {
        salt: 0,
        token_name: token_name.clone(),
        initial_balance,
        minter_address,
        authorized_minters,
    };
    let message_response = bank.call(
        create_token_message,
        context,
        working_set,
    );
    if message_response.is_err() {
        bail!("Error creating token");
    };

    Ok(create_token_address::<C>(
        &token_name,
        context.sender().as_ref(),
        0,
    ))
}

pub fn create_pool_helper(
    swap: sov_swap::SwapModule<C>,
    working_set: &mut sov_state::WorkingSet<Storage>,
    context: &C,
    token_a_address: Address,
    token_b_address: Address,
) -> Result<(Address, Address), Error> {
    let create_pool_message = sov_swap::call::CallMessage::CreatePool {
        token_a: token_a_address.clone(),
        token_b: token_b_address.clone(),
    };
    let message_response = swap.call(
        create_pool_message,
        context,
        working_set,
    );
    if message_response.is_err() {
        bail!("Error creating pool");
    };

    let pool_id = generate_address::<C>(&format!("{:?}{:?}", token_a_address.clone(), token_b_address.clone()));
    let liquidity_token_address = create_token_address::<C>(
        &format!("LP:{}_{}", &token_a_address.clone(), &token_b_address.clone()),
        swap.address.as_ref(),
        0,
    );

    Ok((pool_id, liquidity_token_address))
}

pub fn add_liquidity_helper(
    swap: sov_swap::SwapModule<C>,
    working_set: &mut sov_state::WorkingSet<Storage>,
    context: &C,
    pool_id: Address,
    token_a_amount: u64,
    token_b_amount: u64,
) -> Result<(), Error> {
    let add_liquidity_message = sov_swap::call::CallMessage::AddLiquidity {
        pool_id: pool_id.clone(),
        token_a_amount,
        token_b_amount,
    };
    let message_response = swap.call(
        add_liquidity_message,
        context,
        working_set,
    );
    if message_response.is_err() {
        bail!("Error adding liquidity");
    };

    Ok(())
}