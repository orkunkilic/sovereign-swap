pub mod call;
pub mod genesis;

#[cfg(feature = "native")]
pub mod query;

use sov_modules_api::{Error, Context, CallResponse};
use sov_modules_macros::ModuleInfo;
use sov_state::WorkingSet;
use borsh::{BorshDeserialize, BorshSerialize};

// Struct that stores tokens and liquidity amounts
#[derive(BorshDeserialize, BorshSerialize, Debug, PartialEq, Clone)]
pub struct Pool<C: Context> {
    pub token_a: C::Address,
    pub token_a_liquidity: u64,
    
    pub token_b: C::Address,
    pub token_b_liquidity: u64,

    pub liquidity_token: C::Address,
}

pub struct SwapModuleConfig<C: Context> {
    pub pools: Vec<Pool<C>>,
}

/// A new module:
/// - Must derive `ModuleInfo`
/// - Must contain `[address]` field
/// - Can contain any number of ` #[state]` or `[module]` fields
#[derive(ModuleInfo, Clone)]
pub struct SwapModule<C: Context> {
    /// Address of the module.
    #[address]
    pub address: C::Address,

    /// Mapping to store Pool structs
    #[state]
    pub pools: sov_state::StateMap<C::Address, Pool<C>>,

    /// Reference to the Bank module.
    #[module]
    pub(crate) _bank: sov_bank::Bank<C>,
}

impl<C: sov_modules_api::Context> sov_modules_api::Module for SwapModule<C> {
    type Context = C;

    type Config = SwapModuleConfig<C>;

    type CallMessage = call::CallMessage<C>;

    fn genesis(
        &self,
        config: &Self::Config,
        working_set: &mut WorkingSet<C::Storage>,
    ) -> Result<(), Error> {
        // The initialization logic
        Ok(self.init_module(config, working_set)?)
    }

    fn call(
        &self,
        msg: Self::CallMessage,
        context: &Self::Context,
        working_set: &mut WorkingSet<C::Storage>,
    ) -> Result<CallResponse, Error> {
        let call_result = match msg {
            call::CallMessage::CreatePool {
                token_a,
                token_b,
            } => self.create_pool(token_a, token_b, context, working_set),
            call::CallMessage::AddLiquidity {
                pool_id,
                token_a_amount,
                token_b_amount,
            } => self.add_liquidity(pool_id, token_a_amount, token_b_amount, context, working_set),
            call::CallMessage::RemoveLiquidity {
                pool_id,
                liquidity_amount,
            } => self.remove_liquidity(pool_id, liquidity_amount, context, working_set),
            call::CallMessage::Swap {
                pool_id,
                token_a_amount,
                token_b_amount,
            } => self.swap(pool_id, token_a_amount, token_b_amount, context, working_set),
        };
        Ok(call_result?)
    }
}
