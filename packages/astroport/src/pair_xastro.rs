use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct XastroPairInitParams {
    pub staking: String,
}
