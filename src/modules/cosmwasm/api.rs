use cosmrs::tx::Msg;
use serde::Serialize;

use cosmos_sdk_proto::cosmwasm::wasm::v1::{
    AccessConfig, QuerySmartContractStateRequest, QuerySmartContractStateResponse,
};
use cosmrs::cosmwasm::{
    MsgExecuteContract, MsgInstantiateContract, MsgMigrateContract, MsgStoreCode,
};

use crate::chain::coin::Coin;
use crate::chain::error::ChainError;
use crate::chain::request::TxOptions;
use crate::chain::tx::sign_tx;
use crate::clients::client::CosmTome;

use crate::modules::auth::model::Address;
use crate::{clients::client::CosmosClient, key::key::SigningKey};

use super::model::{ExecResponse, MigrateResponse, QueryResponse};
use super::{
    error::CosmwasmError,
    model::{InstantiateResponse, StoreCodeResponse},
};

#[derive(Clone, Debug)]
pub struct Cosmwasm {}

impl Cosmwasm {
    pub(crate) async fn wasm_store<T: CosmosClient>(
        &self,
        client: &CosmTome<T>,
        payload: Vec<u8>,
        key: &SigningKey,
        instantiate_perms: Option<AccessConfig>, // TODO: make my own type in chain::model
        tx_options: &TxOptions,
    ) -> Result<StoreCodeResponse, CosmwasmError> {
        let sender_addr = key.to_addr(&client.cfg.prefix)?;

        let msg = MsgStoreCode {
            sender: sender_addr.clone().into(),
            wasm_byte_code: payload,
            instantiate_permission: instantiate_perms
                .map(TryInto::try_into)
                .transpose()
                .map_err(|e| CosmwasmError::InstantiatePerms { source: e })?,
        }
        .to_any()
        .map_err(ChainError::proto_encoding)?;

        let tx_raw = sign_tx(client, msg, key, &sender_addr, tx_options).await?;

        let res = client.client.broadcast_tx(&tx_raw).await?;

        let code_id = res
            .find_event_tag("store_code".to_string(), "code_id".to_string())
            .ok_or(CosmwasmError::MissingEvent)?
            .value
            .parse::<u64>()
            .unwrap();

        Ok(StoreCodeResponse { code_id, res: res })
    }

    pub(crate) async fn wasm_instantiate<S, T, I>(
        &self,
        client: &CosmTome<T>,
        code_id: u64,
        msg: &S,
        label: String,
        key: &SigningKey,
        admin: Option<Address>,
        funds: I,
        tx_options: &TxOptions,
    ) -> Result<InstantiateResponse, CosmwasmError>
    where
        S: Serialize,
        T: CosmosClient,
        I: IntoIterator<Item = Coin>,
    {
        let payload = serde_json::to_vec(msg).map_err(CosmwasmError::json)?;
        let sender_addr = key.to_addr(&client.cfg.prefix)?;

        let mut cosm_funds = vec![];
        for fund in funds {
            cosm_funds.push(fund.try_into()?);
        }

        let msg = MsgInstantiateContract {
            sender: sender_addr.clone().into(),
            admin: admin.map(|s| s.into()),
            code_id,
            label: Some(label),
            msg: payload,
            funds: cosm_funds,
        }
        .to_any()
        .map_err(ChainError::proto_encoding)?;

        let tx_raw = sign_tx(client, msg, key, &sender_addr, tx_options).await?;

        let res = client.client.broadcast_tx(&tx_raw).await?;

        let addr = res
            .find_event_tag("instantiate".to_string(), "_contract_address".to_string())
            .ok_or(CosmwasmError::MissingEvent)?
            .value;

        Ok(InstantiateResponse {
            address: addr.parse()?,
            res: res,
        })
    }

    pub(crate) async fn wasm_execute<S, T, I>(
        &self,
        client: &CosmTome<T>,
        address: &Address,
        msg: &S,
        key: &SigningKey,
        funds: I,
        tx_options: &TxOptions,
    ) -> Result<ExecResponse, CosmwasmError>
    where
        S: Serialize,
        T: CosmosClient,
        I: IntoIterator<Item = Coin>,
    {
        let payload = serde_json::to_vec(msg).map_err(CosmwasmError::json)?;

        let sender_addr = key.to_addr(&client.cfg.prefix)?;

        let mut cosm_funds = vec![];
        for fund in funds {
            cosm_funds.push(fund.try_into()?);
        }

        let msg = MsgExecuteContract {
            sender: sender_addr.clone().into(),
            contract: address.into(),
            msg: payload,
            funds: cosm_funds,
        }
        .to_any()
        .map_err(ChainError::proto_encoding)?;

        let tx_raw = sign_tx(client, msg, key, &sender_addr, tx_options).await?;

        let res = client.client.broadcast_tx(&tx_raw).await?;

        Ok(ExecResponse { res })
    }

    pub(crate) async fn wasm_query<S: Serialize, T: CosmosClient>(
        &self,
        client: &CosmTome<T>,
        address: &Address,
        msg: &S,
    ) -> Result<QueryResponse, CosmwasmError> {
        let payload = serde_json::to_vec(msg).map_err(CosmwasmError::json)?;

        let req = QuerySmartContractStateRequest {
            address: address.to_string(),
            query_data: payload,
        };

        let res = client
            .client
            .query::<_, QuerySmartContractStateRequest, QuerySmartContractStateResponse>(
                req,
                "/cosmwasm.wasm.v1.Query/SmartContractState",
            )
            .await?;

        Ok(QueryResponse { res: res.into() })
    }

    pub async fn migrate<T: CosmosClient>(
        &self,
        client: &CosmTome<T>,
        address: &Address,
        new_code_id: u64,
        payload: Vec<u8>,
        key: &SigningKey,
        tx_options: &TxOptions,
    ) -> Result<MigrateResponse, CosmwasmError> {
        let sender_addr = key.to_addr(&client.cfg.prefix)?;

        let msg = MsgMigrateContract {
            sender: sender_addr.clone().into(),
            contract: address.into(),
            code_id: new_code_id,
            msg: payload,
        }
        .to_any()
        .map_err(ChainError::proto_encoding)?;

        let tx_raw = sign_tx(client, msg, key, &sender_addr, tx_options).await?;

        let res = client.client.broadcast_tx(&tx_raw).await?;

        Ok(MigrateResponse { res })
    }

    // TODO: Finish
}
