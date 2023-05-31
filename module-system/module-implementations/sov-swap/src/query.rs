use super::SwapModule;
use sov_state::WorkingSet;

#[derive(serde::Serialize, serde::Deserialize, Debug, Eq, PartialEq)]
pub struct Response {
    pub value: Option<u32>,
}

impl<C: sov_modules_api::Context> SwapModule<C> {
    /* pub fn query_value(&self, working_set: &mut WorkingSet<C::Storage>) -> Response {
        Response {
            value: self.address.ok().map(|_| 0),
        }
    } */
}
