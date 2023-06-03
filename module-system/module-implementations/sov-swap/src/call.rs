use anyhow::{Result, Error};
use sov_bank::create_token_address;
use sov_modules_api::{CallResponse, Address, Module};
use sov_state::WorkingSet;
use std::fmt::Debug;
use thiserror::Error;
use sov_modules_api::{Spec, Hasher};
use anyhow::{bail};
use sov_modules_api::{ Context};

use crate::{
    Pool,
    SwapModule,
};

pub fn generate_address<C: Context>(key: &str) -> C::Address {
    let hash = C::Hasher::hash(key.as_bytes());
    C::Address::from(hash)
}

/// This enumeration represents the available call messages for interacting with the `SwapModule` module.
#[cfg_attr(
    feature = "native",
    derive(serde::Serialize),
    derive(serde::Deserialize)
)]
#[derive(borsh::BorshDeserialize, borsh::BorshSerialize, Debug, PartialEq, Clone)]
pub enum CallMessage<C: Context> {
    CreatePool {
        token_a: C::Address,
        token_b: C::Address,
    },
    AddLiquidity {
        pool_id: C::Address,
        token_a_amount: u64,
        token_b_amount: u64,
    },
    RemoveLiquidity {
        pool_id: C::Address,
        liquidity_amount: u64,
    },
    Swap {
        pool_id: C::Address,
        token_a_amount: u64,
        token_b_amount: u64,
    },
}

/// Example of a custom error.
#[derive(Debug, Error)]
enum SetValueError {}

impl<C: Context> SwapModule<C> {
    // Create a new pool
    pub(crate) fn create_pool(
        &self,
        token_a: C::Address,
        token_b: C::Address,
        _context: &C,
        working_set: &mut WorkingSet<C::Storage>,
    ) -> Result<sov_modules_api::CallResponse> {
        // pool id is sum of token addresses
        let pool_id = generate_address::<C>(&format!("{:?}{:?}", token_a, token_b));

        // check if pool already exists (pool.get is None if pool does not exist)
        if self.pools.get(&pool_id, working_set).is_some() {
            bail!("Pool already exists");
        }

        // create token
        let create_token_message = sov_bank::call::CallMessage::<C>::CreateToken {
            salt: 0,
            token_name: format!("LP:{}_{}", token_a, token_b),
            initial_balance: 0,
            minter_address: self.address.clone(),
            authorized_minters: vec![self.address.clone()],
        };
        let create_token_result = sov_modules_api::Module::call(&self._bank, create_token_message, &C::new(self.address.clone()), working_set);
        if create_token_result.is_err() {
            bail!("Create token failed");
        }

        
        // generate token address
        let token_address = create_token_address::<C>(
            &format!("LP:{}_{}", &token_a, &token_b),
            self.address.clone().as_ref(),
            0,
        );
        
        // get total supply to check if token was created
        let total_supply = self._bank.supply_of(token_address.clone(), working_set);
        if total_supply.amount.is_none() {
            bail!("Token was not created");
        } else {
            println!("total_supply: {:?}", total_supply.amount.unwrap());
        }

        let pool = Pool {
            token_a: token_a.clone(),
            token_b: token_b.clone(),
            token_a_liquidity: 0,
            token_b_liquidity: 0,
            liquidity_token: token_address.clone(),
        };
        self.pools.set(&pool_id, &pool, working_set);
        working_set.add_event("create_pool", &format!(
            "pool_id: {pool_id:?}, token_a: {token_a:?}, token_b: {token_b:?}, liquidity_token: {liquidity_token:?}",
            pool_id = pool_id,
            token_a = token_a,
            token_b = token_b,
            liquidity_token = token_address,
        ));

        Ok(CallResponse::default())
    }

    // Add liquidity to a pool
    pub(crate) fn add_liquidity(
        &self,
        pool_id: C::Address,
        token_a_amount: u64,
        token_b_amount: u64,
        _context: &C,
        working_set: &mut WorkingSet<C::Storage>,
    ) -> Result<sov_modules_api::CallResponse> {
        // check if pool exists
        let mut pool = self.pools.get_or_err(&pool_id, working_set)?;

        let transfer_a = self._bank.transfer_from(
            // from
            _context.sender(),
            // to (this)
            &self.address,
            // coin
            sov_bank::Coins {
                token_address: pool.token_a.clone(),
                amount: token_a_amount,
            },
            working_set,
        );
        let transfer_b = self._bank.transfer_from(
            // from
            _context.sender(),
            // to (this)
            &self.address,
            // coin
            sov_bank::Coins {
                token_address: pool.token_b.clone(),
                amount: token_b_amount,
            },
            working_set,
        );

        // check if transfers were successful
        if transfer_a.is_err() || transfer_b.is_err() {
            bail!("Transfer failed");
        }

        // if pool is empty, add liquidity
        if pool.token_a_liquidity == 0 && pool.token_b_liquidity == 0 {
            pool.token_a_liquidity = token_a_amount;
            pool.token_b_liquidity = token_b_amount;
            self.pools.set(
                &pool_id,
                &pool.clone(),
                working_set,
            );

            // Initial amount is geometric mean of token amounts
            let liquidity_token_amount = ((token_a_amount * token_b_amount) as f64).sqrt() as u64;

            // log amounts
            println!("liquidity_token_amount: {:?}", liquidity_token_amount);
            println!("liquidity_token-address: {:?}", pool.liquidity_token.clone());

            // mint liquidity token
            let mint_message = sov_bank::call::CallMessage::<C>::Mint {
                coins: sov_bank::Coins {
                    token_address: pool.liquidity_token.clone(),
                    amount: liquidity_token_amount,
                },
                minter_address: _context.sender().clone(),
            };
            let mint_result = sov_modules_api::Module::call(
                &self._bank,
                mint_message,
                // context of this
                &C::new(self.address.clone()),
                working_set);
            if mint_result.is_err() {
                let error = mint_result.err().unwrap();
                bail!("Mint failed: {:?}", error);
            }
        } else {
            // calculate liquidity to add
            let token_a_liquidity = token_a_amount * pool.token_a_liquidity / pool.token_a_liquidity;
            let token_b_liquidity = token_b_amount * pool.token_b_liquidity / pool.token_b_liquidity;

            // get liquidity token total supply
            let liquidity_token_total_supply = self._bank.supply_of(pool.liquidity_token.clone(), working_set);
            let liquidity_token_total_supply = liquidity_token_total_supply.amount.unwrap_or(0);

            // Amount to mint is (Xdeposited / XStarting) * TotalSupply
            let liquidity_token_amount = (token_a_amount / pool.token_a_liquidity) * liquidity_token_total_supply;

            // mint liquidity token
            let mint_message = sov_bank::call::CallMessage::<C>::Mint {
                coins: sov_bank::Coins {
                    token_address: pool.liquidity_token.clone(),
                    amount: liquidity_token_amount,
                },
                minter_address: _context.sender().clone(),
            };
            let mint_result = sov_modules_api::Module::call(&self._bank, mint_message, _context, working_set);
            if mint_result.is_err() {
                bail!("Mint failed");
            }

            // update pool
            pool.token_a_liquidity += token_a_liquidity;
            pool.token_b_liquidity += token_b_liquidity;
        }

        // update pool
        self.pools.set(&pool_id, &pool, working_set);
        working_set.add_event("add_liquidity", &format!("pool_id: {pool_id:?}"));

        Ok(CallResponse::default())
    }

    // Remove liquidity from a pool
    pub(crate) fn remove_liquidity(
        &self,
        pool_id: C::Address,
        liquidity_amount: u64,
        _context: &C,
        working_set: &mut WorkingSet<C::Storage>,
    ) -> Result<sov_modules_api::CallResponse> {
        // TODO
        Ok(CallResponse::default())
    }

    // Swap tokens
    pub(crate) fn swap(
        &self,
        pool_id: C::Address,
        token_a_amount: u64,
        token_b_amount: u64,
        _context: &C,
        working_set: &mut WorkingSet<C::Storage>,
    ) -> Result<sov_modules_api::CallResponse> {
        
        // check if pool exists
        let mut pool = self.pools.get_or_err(&pool_id, working_set)?;


        // only send one token
        if token_a_amount > 0 && token_b_amount > 0 {
            bail!("Can only send one token");
        }

        // check if pool has liquidity
        if pool.token_a_liquidity == 0 || pool.token_b_liquidity == 0 {
            bail!("Pool has no liquidity");
        }

        // check if pool has enough liquidity
        if pool.token_a_liquidity < token_a_amount || pool.token_b_liquidity < token_b_amount {
            bail!("Pool does not have enough liquidity");
        }

        let mut a_to_send = 0;
        let mut b_to_send = 0;

        // if sending a, calculate b to send. Increase liquidity of a, decrease liquidity of b
        if token_a_amount > 0 {
            b_to_send = token_a_amount * pool.token_b_liquidity / pool.token_a_liquidity;
            pool.token_a_liquidity += token_a_amount;
            pool.token_b_liquidity -= b_to_send;

            // transfer tokens
            let transfer = self._bank.transfer_from(
                // from
                &self.address,
                // to (this)
                _context.sender(),
                // coin
                sov_bank::Coins {
                    token_address: pool.token_b.clone(),
                    amount: b_to_send,
                },
                working_set,
            );

            // check if transfer was successful
            if transfer.is_err() {
                bail!("Transfer failed");
            }

            // receive tokens
            let transfer = self._bank.transfer_from(
                // from
                _context.sender(),
                // to (this)
                &self.address,
                // coin
                sov_bank::Coins {
                    token_address: pool.token_a.clone(),
                    amount: token_a_amount,
                },
                working_set,
            );

            // check if transfer was successful
            if transfer.is_err() {
                bail!("Transfer failed");
            }
        } else {
            // if sending b, calculate a to send. Increase liquidity of b, decrease liquidity of a
            a_to_send = token_b_amount * pool.token_a_liquidity / pool.token_b_liquidity;
            pool.token_b_liquidity += token_b_amount;
            pool.token_a_liquidity -= a_to_send;

            // transfer tokens
            let transfer = self._bank.transfer_from(
                // from
                &self.address,
                // to (this)
                _context.sender(),
                // coin
                sov_bank::Coins {
                    token_address: pool.token_a.clone(),
                    amount: a_to_send,
                },
                working_set,
            );

            // check if transfer was successful
            if transfer.is_err() {
                bail!("Transfer failed");
            }

            // receive tokens
            let transfer = self._bank.transfer_from(
                // from
                _context.sender(),
                // to (this)
                &self.address,
                // coin
                sov_bank::Coins {
                    token_address: pool.token_b.clone(),
                    amount: token_b_amount,
                },
                working_set,
            );

            // check if transfer was successful
            if transfer.is_err() {
                bail!("Transfer failed");
            }
        }

        // update pool
        self.pools.set(&pool_id, &pool, working_set);

        // log amounts
        println!("token_a_amount: {:?}", token_a_amount);
        println!("token_b_amount: {:?}", token_b_amount);
        println!("a_to_send: {:?}", a_to_send);
        println!("b_to_send: {:?}", b_to_send);


        // emit event with pool_id, sender, send_token, send_amount, receive_token, receive_amount
        let sender = _context.sender();
        working_set.add_event(
            "swap",
            &format!(
                "pool_id: {pool_id:?}, sender: {sender}, send_token: {send_token:?}, send_amount: {send_amount:?}, receive_token: {receive_token:?}, receive_amount: {receive_amount:?}",
                pool_id = pool_id,
                sender = sender,
                send_token = if token_a_amount > 0 { pool.token_a.clone() } else { pool.token_b.clone() },
                send_amount = if token_a_amount > 0 { token_a_amount } else { token_b_amount },
                receive_token = if token_a_amount > 0 { pool.token_b.clone() } else { pool.token_a.clone() },
                receive_amount = if token_a_amount > 0 { b_to_send } else { a_to_send },
            ),
        );

        Ok(CallResponse::default())
    }
}
