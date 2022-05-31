use ethers::abi::Tokenize;
use ethers::contract::Contract;
use ethers::{
    prelude::*,
    utils::{Ganache, GanacheInstance},
};

use ethers::providers::{Http, Ws};

use eyre::{ContextCompat, Result};

// pub type DeployedContract<T> = Contract<Provider<T>>;

pub type SignerDeployedContract<T> = Contract<SignerMiddleware<Provider<T>, LocalWallet>>;

pub struct GanacheAccount<T: Clone + JsonRpcClient> {
    pub ganache: GanacheInstance,
    pub chain_id: u64,
    pub endpoint: String,
    pub provider: Provider<T>,
}

#[allow(dead_code)]
impl GanacheAccount<Http> {
    pub async fn new_from_seed_with_http(seed_words: String) -> Result<Self> {
        let ganache = Ganache::new().mnemonic(seed_words).spawn();
        let provider = Provider::try_from(ganache.endpoint())?;
        let endpoint = ganache.endpoint();
        let chain_id = provider.get_chainid().await?.as_u64();
        Ok(GanacheAccount {
            ganache,
            chain_id,
            endpoint,
            provider,
        })
    }

    pub async fn new_from_seed_with_http_args(
        seed_words: String,
        args: Vec<String>,
    ) -> Result<Self> {
        let ganache = Ganache::new().args(args).mnemonic(seed_words).spawn();
        let provider = Provider::try_from(ganache.endpoint())?;
        let endpoint = ganache.endpoint();
        let chain_id = provider.get_chainid().await?.as_u64();
        Ok(GanacheAccount {
            ganache,
            chain_id,
            endpoint,
            provider,
        })
    }
}

impl GanacheAccount<Ws> {
    pub async fn new_from_seed_with_ws(seed_words: String) -> Result<Self> {
        let ganache = Ganache::new().mnemonic(seed_words).spawn();
        let provider = Provider::connect(ganache.ws_endpoint()).await?;
        let endpoint = ganache.endpoint();
        let chain_id = provider.get_chainid().await?.as_u64();
        Ok(GanacheAccount {
            ganache,
            chain_id,
            endpoint,
            provider,
        })
    }
}

#[allow(dead_code)]
impl<T: 'static + Clone + JsonRpcClient> GanacheAccount<T> {
    pub fn get_default_wallet(&self) -> Result<LocalWallet> {
        self.get_wallet(0)
    }

    pub fn get_chain_id(&self) -> u64 {
        self.chain_id
    }

    pub fn get_wallet(&self, i: usize) -> Result<LocalWallet> {
        let wallet = self
            .ganache
            .keys()
            .get(i)
            .context("Wallet not found at this index")?
            .clone()
            .into();
        Ok(wallet)
    }

    pub async fn deploy_contract<A: Tokenize>(
        &self,
        contract: ConfigurableContractArtifact,
        constructor_args: A,
    ) -> Result<SignerDeployedContract<T>> {
        let (abi, bytecode, _) = contract.into_parts();
        let abi = abi.context("Missing abi from contract")?;
        let bytecode = bytecode.context("Missing bytecode from contract")?;

        // Create signer client
        let wallet = self.get_default_wallet()?.with_chain_id(self.chain_id);
        let provider = self.provider.clone();
        let client = SignerMiddleware::new(provider, wallet).into();

        let factory = ContractFactory::new(abi.clone(), bytecode, client);

        // Deploy contract
        let mut deployer = factory.deploy(constructor_args)?;
        deployer.tx.set_gas::<U256>(20_000_000.into()); // 10 times of default value
        deployer.tx.set_gas_price::<U256>(42_000_000_000u128.into()); // 2 times of default value
        let contract = deployer.clone().legacy().send().await?;
        Ok(contract)
    }
}
