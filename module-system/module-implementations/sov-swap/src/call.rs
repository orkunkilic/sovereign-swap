use anyhow::{Result, Error};
use sov_modules_api::{CallResponse, Address};
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

        let pool = Pool {
            token_a,
            token_b,
            token_a_liquidity: 0,
            token_b_liquidity: 0,
        };
        self.pools.set(&pool_id, &pool, working_set);
        working_set.add_event("create_pool", &format!("pool_id: {pool_id:?}"));

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

        // log token addresses
        println!("token_a: {:?}", pool.token_a);
        println!("token_b: {:?}", pool.token_b);

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
            // log errors
            if transfer_a.is_err() {
                println!("Transfer failed: {:?}", transfer_a.err().unwrap());
            }
            if transfer_b.is_err() {
                println!("Transfer failed: {:?}", transfer_b.err().unwrap());
            }
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
            working_set.add_event("add_liquidity", &format!("pool_id: {pool_id:?}"));
            return Ok(CallResponse::default());
        } else {
            // calculate liquidity to add
            let token_a_liquidity = token_a_amount * pool.token_a_liquidity / pool.token_a_liquidity;
            let token_b_liquidity = token_b_amount * pool.token_b_liquidity / pool.token_b_liquidity;

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
        // TODO
        Ok(CallResponse::default())
    }
}
