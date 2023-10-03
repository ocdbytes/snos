use blockifier::abi::constants::{INITIAL_GAS_COST, MAX_STEPS_PER_TX, N_STEPS_RESOURCE};
use blockifier::block_context::{BlockContext, FeeTokenAddresses, GasPrices};
use blockifier::transaction::objects::FeeType;
use serde::{Deserialize, Serialize};
use starknet_api::block::{BlockNumber, BlockTimestamp};
use starknet_api::core::{ChainId, ContractAddress, PatriciaKey};
use starknet_api::hash::StarkHash;
use starknet_api::{contract_address, patricia_key};
use std::collections::HashMap;
use std::fs::File;
use std::path::PathBuf;
use std::sync::Arc;

use crate::error::SnOsError;

const DEFAULT_CONFIG_PATH: &str =
    "cairo-lang/src/starkware/starknet/definitions/general_config.yml";

pub const DEFAULT_STORAGE_TREE_HEIGHT: u64 = 251;
pub const DEFAULT_INNER_TREE_HEIGHT: u64 = 64;
pub const DEFAULT_FEE_TOKEN_ADDR: &str =
    "482bc27fc5627bf974a72b65c43aa8a0464a70aab91ad8379b56a4f17a84c3";
pub const SEQUENCER_ADDR_0_12_2: &str =
    "6c95526293b61fa708c6cba66fd015afee89309666246952456ab970e9650aa";

// Given in units of wei
pub const DEFAULT_L1_GAS_PRICE: u64 = 10u64.pow(8);
pub const DEFAULT_STARK_L1_GAS_PRICE: u64 = 0;

#[derive(Debug, Serialize, Deserialize)]
pub struct StarknetOsConfig {
    pub chain_id: ChainId,
    pub fee_token_address: ContractAddress,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StarknetGeneralConfig {
    pub starknet_os_config: StarknetOsConfig,
    pub contract_storage_commitment_tree_height: u64,
    pub compiled_class_hash_commitment_tree_height: u64,
    pub global_state_commitment_tree_height: u64,
    pub invoke_tx_max_n_steps: u32,
    pub validate_max_n_steps: u32,
    pub min_gas_price: u128,
    pub constant_gas_price: bool,
    pub sequencer_address: ContractAddress,
    pub tx_commitment_tree_height: u64,
    pub event_commitment_tree_height: u64,
    pub cairo_resource_fee_weights: Arc<HashMap<String, f64>>,
    pub enforce_l1_handler_fee: bool,
}

impl Default for StarknetGeneralConfig {
    fn default() -> Self {
        match StarknetGeneralConfig::from_file(PathBuf::from(DEFAULT_CONFIG_PATH)) {
            Ok(conf) => conf,
            Err(_) => Self {
                starknet_os_config: StarknetOsConfig {
                    chain_id: ChainId("SN_GOERLI".to_string()),
                    fee_token_address: contract_address!(DEFAULT_FEE_TOKEN_ADDR),
                },
                contract_storage_commitment_tree_height: DEFAULT_STORAGE_TREE_HEIGHT,
                compiled_class_hash_commitment_tree_height: DEFAULT_STORAGE_TREE_HEIGHT,
                global_state_commitment_tree_height: DEFAULT_STORAGE_TREE_HEIGHT,
                invoke_tx_max_n_steps: MAX_STEPS_PER_TX as u32,
                validate_max_n_steps: MAX_STEPS_PER_TX as u32,
                min_gas_price: INITIAL_GAS_COST as u128,
                constant_gas_price: false,
                sequencer_address: contract_address!(SEQUENCER_ADDR_0_12_2),
                tx_commitment_tree_height: DEFAULT_INNER_TREE_HEIGHT,
                event_commitment_tree_height: DEFAULT_INNER_TREE_HEIGHT,
                cairo_resource_fee_weights: Arc::new(HashMap::from([(
                    N_STEPS_RESOURCE.to_string(),
                    1.0,
                )])),
                enforce_l1_handler_fee: true,
            },
        }
    }
}

impl StarknetGeneralConfig {
    pub fn from_file(f: PathBuf) -> Result<StarknetGeneralConfig, SnOsError> {
        let conf = File::open(f).map_err(|e| SnOsError::CatchAll(format!("config - {e}")))?;
        serde_yaml::from_reader(conf).map_err(|e| SnOsError::CatchAll(format!("config - {e}")))
    }
    pub fn empty_block_context(&self) -> BlockContext {
        BlockContext {
            chain_id: self.starknet_os_config.chain_id.clone(),
            block_number: BlockNumber(0),
            block_timestamp: BlockTimestamp(0),
            sequencer_address: self.sequencer_address,
            fee_token_addresses: FeeTokenAddresses {
                eth_fee_token_address: self.starknet_os_config.fee_token_address,
                strk_fee_token_address: contract_address!("0x0"),
            },
            vm_resource_fee_cost: self.cairo_resource_fee_weights.clone(),
            gas_prices: GasPrices {
                eth_l1_gas_price: self.min_gas_price,
                strk_l1_gas_price: self.min_gas_price,
            },
            invoke_tx_max_n_steps: self.invoke_tx_max_n_steps,
            validate_max_n_steps: self.validate_max_n_steps,
            max_recursion_depth: 50,
        }
    }
}

impl TryFrom<BlockContext> for StarknetGeneralConfig {
    type Error = SnOsError;

    fn try_from(block_context: BlockContext) -> Result<Self, SnOsError> {
        Ok(Self {
            starknet_os_config: StarknetOsConfig {
                chain_id: block_context.chain_id,
                fee_token_address: block_context
                    .fee_token_addresses
                    .get_by_fee_type(&FeeType::Eth),
            },
            sequencer_address: block_context.sequencer_address,
            cairo_resource_fee_weights: block_context.vm_resource_fee_cost,
            min_gas_price: block_context.gas_prices.get_by_fee_type(&FeeType::Eth),
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_starknet_config() {
        let expected_seq_addr = contract_address!(SEQUENCER_ADDR_0_12_2);

        let conf = StarknetGeneralConfig::default();

        assert_eq!(251, conf.compiled_class_hash_commitment_tree_height);
        assert_eq!(251, conf.contract_storage_commitment_tree_height);
        assert_eq!(251, conf.global_state_commitment_tree_height);

        assert_eq!(false, conf.constant_gas_price);
        assert_eq!(true, conf.enforce_l1_handler_fee);

        assert_eq!(64, conf.event_commitment_tree_height);
        assert_eq!(64, conf.tx_commitment_tree_height);

        assert_eq!(1000000, conf.invoke_tx_max_n_steps);
        assert_eq!(100000000000, conf.min_gas_price);
        assert_eq!(1000000, conf.validate_max_n_steps);

        assert_eq!(expected_seq_addr, conf.sequencer_address);
    }

    #[test]
    fn convert_block_context() {
        let conf = StarknetGeneralConfig::default();
        let ctx: BlockContext = conf.empty_block_context();

        assert_eq!(conf.starknet_os_config.chain_id, ctx.chain_id);
        assert_eq!(
            conf.starknet_os_config.fee_token_address,
            ctx.fee_token_addresses.get_by_fee_type(&FeeType::Eth)
        );
        assert_eq!(conf.sequencer_address, ctx.sequencer_address);
        assert_eq!(conf.cairo_resource_fee_weights, ctx.vm_resource_fee_cost);
    }
}
