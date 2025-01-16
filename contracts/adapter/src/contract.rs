use astroport::asset as aa;
use astroport::incentives as aincentives;
use astroport::pair as apair;
use astroport::pair_concentrated as apc;
use astroport::querier as aq;
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    entry_point, to_json_binary, Binary, Coin, Deps, DepsMut, Env, QueryRequest, Response,
    StdError, StdResult, Uint128, WasmQuery,
};
use cw_storage_plus::Item;

/// -----------------
/// STATE
/// -----------------
#[cw_serde]
pub struct Config {
    pub astroport_incentive_contract: String,
    pub concentrated_pool_address: String,
}

pub const CONFIG: Item<Config> = Item::new("config");

/// -----------------
/// INSTANTIATE
/// -----------------
#[cw_serde]
pub struct InstantiateMsg {
    pub astroport_incentive_contract: String,
    pub concentrated_pool_address: String,
}

#[entry_point]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: cosmwasm_std::MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let cfg = Config {
        astroport_incentive_contract: msg.astroport_incentive_contract,
        concentrated_pool_address: msg.concentrated_pool_address,
    };
    CONFIG.save(deps.storage, &cfg)?;
    Ok(Response::new().add_attribute("method", "instantiate"))
}

/// -----------------
/// EXECUTE (unused)
/// -----------------
#[cw_serde]
pub enum ExecuteMsg {
    // No execute messages for now
}

/// -----------------
/// QUERY
/// -----------------
#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// Returns the "simulated withdraw" amounts for the LP tokens owned by `address`.
    #[returns(CurrentHoldingsResponse)]
    CurrentHoldings { address: String },
    /// Unimplemented
    #[returns(CurrentTotalLiquidity)]
    CurrentTotalLiquidity {},
}

/// Response structure for CurrentHoldings
#[cw_serde]
pub struct CurrentHoldingsResponse {
    // The coins returned by simulate_withdraw
    pub coins: Vec<Coin>,
}

/// Response structure for CurrentHoldings
#[cw_serde]
pub struct CurrentTotalLiquidity {
    // The coins returned by simulate_withdraw
    pub coins: Vec<Coin>,
}

/// -----------------
/// QUERY ENTRY POINT
/// -----------------
#[entry_point]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::CurrentHoldings { address } => {
            to_json_binary(&query_current_holdings(deps, address)?)
        }
        QueryMsg::CurrentTotalLiquidity {} => {
            // Currently unimplemented
            to_json_binary(&query_total_liquidity(deps)?)
        }
    }
}

/// -----------------
/// QUERY HANDLER: CURRENT HOLDINGS
/// -----------------
fn query_current_holdings(deps: Deps, address: String) -> StdResult<CurrentHoldingsResponse> {
    let config = CONFIG.load(deps.storage)?;

    // --------------------------------------------------
    // 1) Query the pair to get the LP token denom
    //    e.g. {"pair":{}}
    //    The response has a "liquidity_token" field like:
    //    "factory/neutron1.../astroport/share"
    // --------------------------------------------------
    let pair_query = apc::QueryMsg::Pair {};
    let pair_res: aa::PairInfo = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: config.concentrated_pool_address.clone(),
        msg: to_json_binary(&pair_query)?,
    }))?;

    // The LP token denom is directly in the liquidity_token field
    let lp_token_denom = pair_res.liquidity_token;

    // --------------------------------------------------
    // 2) Query the bank balance for that LP token from the user's address
    // --------------------------------------------------
    let user_lp_balance_on_chain =
        aq::query_balance(&deps.querier, address.clone(), lp_token_denom.clone())?;

    // --------------------------------------------------
    // 3) Query the Astroport incentive contract for how many LP tokens are deposited
    //    e.g. {"deposit":{"lp_token":"LP_TOKEN_DENOM","user":"ADDRESS"}}
    // --------------------------------------------------
    let deposit_msg = aincentives::QueryMsg::Deposit {
        lp_token: lp_token_denom,
        user: address,
    };

    let user_lp_deposited_incentives: Uint128 =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: config.astroport_incentive_contract,
            msg: to_json_binary(&deposit_msg)?,
        }))?;

    // Sum up balance and staked incentives to a single number
    let total_user_lp_amount = user_lp_balance_on_chain + user_lp_deposited_incentives;

    // --------------------------------------------------
    // 4) Now call the pool with `simulate_withdraw` for the *total* user LP
    //    (i.e., total_user_lp_amount).
    // --------------------------------------------------
    let simulate_withdraw_msg = apc::QueryMsg::SimulateWithdraw {
        lp_amount: total_user_lp_amount,
    };

    let sim_res: Vec<aa::Asset> = deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
        contract_addr: config.concentrated_pool_address,
        msg: to_json_binary(&simulate_withdraw_msg)?,
    }))?;

    // Convert the output into an array of Coin
    let coins: Vec<Coin> = sim_res
        .into_iter()
        .map(|asset| Coin {
            denom: asset.info.to_string(),
            amount: asset.amount,
        })
        .collect();

    Ok(CurrentHoldingsResponse { coins })
}

fn query_total_liquidity(deps: Deps) -> StdResult<CurrentTotalLiquidity> {
    let config = CONFIG.load(deps.storage)?;
    let pool_info = apc::QueryMsg::Pool {};
    let pool_res: apair::PoolResponse =
        deps.querier.query(&QueryRequest::Wasm(WasmQuery::Smart {
            contract_addr: config.concentrated_pool_address.clone(),
            msg: to_json_binary(&pool_info)?,
        }))?;

    // Convert the output into an array of Coin
    let coins: Vec<Coin> = pool_res
        .assets
        .into_iter()
        .map(|asset| Coin {
            denom: asset.info.to_string(),
            amount: asset.amount,
        })
        .collect();

    Ok(CurrentTotalLiquidity { coins })
}

#[derive(thiserror::Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    // Add custom errors if you like
}
