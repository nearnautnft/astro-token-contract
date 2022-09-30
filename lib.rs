/*!
Fungible Token implementation with JSON serialization.
NOTES:
  - The maximum balance value is limited by U128 (2**128 - 1).
  - JSON calls should pass U128 as a base-10 string. E.g. "100".
  - The contract optimizes the inner trie structure by hashing account IDs. It will prevent some
    abuse of deep tries. Shouldn't be an issue, once NEAR clients implement full hashing of keys.
  - The contract tracks the change in storage before and after the call. If the storage increases,
    the contract requires the caller of the contract to attach enough deposit to the function call
    to cover the storage cost.
    This is done to prevent a denial of service attack on the contract by taking all available storage.
    If the storage decreases, the contract will issue a refund for the cost of the released storage.
    The unused tokens from the attached deposit are also refunded, so it's safe to
    attach more deposit than required.
  - To prevent the deployed contract from being modified or deleted, it should not have any access
    keys on its account.
*/
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider, FT_METADATA_SPEC,
};
use near_contract_standards::fungible_token::FungibleToken;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::LazyOption;
use near_sdk::json_types::{ValidAccountId, U128};
use near_sdk::{env, log, near_bindgen, AccountId, Balance, PanicOnDefault, PromiseOrValue};

near_sdk::setup_alloc!();

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct Contract {
    token: FungibleToken,
    metadata: LazyOption<FungibleTokenMetadata>,
}

const SVG_TOKEN_ICON: &str = "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAGIAAABiCAMAAACce/Y8AAAGf2lUWHRYTUw6Y29tLmFkb2JlLnhtcAAAAAAAPD94cGFja2V0IGJlZ2luPSLvu78iIGlkPSJXNU0wTXBDZWhpSHpyZVN6TlRjemtjOWQiPz4gPHg6eG1wbWV0YSB4bWxuczp4PSJhZG9iZTpuczptZXRhLyIgeDp4bXB0az0iQWRvYmUgWE1QIENvcmUgNS42LWMxNDIgNzkuMTYwOTI0LCAyMDE3LzA3LzEzLTAxOjA2OjM5ICAgICAgICAiPiA8cmRmOlJERiB4bWxuczpyZGY9Imh0dHA6Ly93d3cudzMub3JnLzE5OTkvMDIvMjItcmRmLXN5bnRheC1ucyMiPiA8cmRmOkRlc2NyaXB0aW9uIHJkZjphYm91dD0iIiB4bWxuczp4bXA9Imh0dHA6Ly9ucy5hZG9iZS5jb20veGFwLzEuMC8iIHhtbG5zOmRjPSJodHRwOi8vcHVybC5vcmcvZGMvZWxlbWVudHMvMS4xLyIgeG1sbnM6cGhvdG9zaG9wPSJodHRwOi8vbnMuYWRvYmUuY29tL3Bob3Rvc2hvcC8xLjAvIiB4bWxuczp4bXBNTT0iaHR0cDovL25zLmFkb2JlLmNvbS94YXAvMS4wL21tLyIgeG1sbnM6c3RFdnQ9Imh0dHA6Ly9ucy5hZG9iZS5jb20veGFwLzEuMC9zVHlwZS9SZXNvdXJjZUV2ZW50IyIgeG1wOkNyZWF0b3JUb29sPSJBZG9iZSBQaG90b3Nob3AgQ0MgKFdpbmRvd3MpIiB4bXA6Q3JlYXRlRGF0ZT0iMjAyMi0wNS0yNFQxODo1ODowOSswMzowMCIgeG1wOk1vZGlmeURhdGU9IjIwMjItMDUtMjRUMTk6MTQ6MjMrMDM6MDAiIHhtcDpNZXRhZGF0YURhdGU9IjIwMjItMDUtMjRUMTk6MTQ6MjMrMDM6MDAiIGRjOmZvcm1hdD0iaW1hZ2UvcG5nIiBwaG90b3Nob3A6Q29sb3JNb2RlPSIzIiB4bXBNTTpJbnN0YW5jZUlEPSJ4bXAuaWlkOjc0ZjQ4NmFiLWFiYzQtNWU0Yy05MDc3LTNmYjNjMjQzM2E5NCIgeG1wTU06RG9jdW1lbnRJRD0ieG1wLmRpZDo2YzQ3M2U0OS00MWYzLTg3NDItYmQyYS0yZGM5NWFmMjlkOTkiIHhtcE1NOk9yaWdpbmFsRG9jdW1lbnRJRD0ieG1wLmRpZDo2YzQ3M2U0OS00MWYzLTg3NDItYmQyYS0yZGM5NWFmMjlkOTkiPiA8eG1wTU06SGlzdG9yeT4gPHJkZjpTZXE+IDxyZGY6bGkgc3RFdnQ6YWN0aW9uPSJjcmVhdGVkIiBzdEV2dDppbnN0YW5jZUlEPSJ4bXAuaWlkOjZjNDczZTQ5LTQxZjMtODc0Mi1iZDJhLTJkYzk1YWYyOWQ5OSIgc3RFdnQ6d2hlbj0iMjAyMi0wNS0yNFQxODo1ODowOSswMzowMCIgc3RFdnQ6c29mdHdhcmVBZ2VudD0iQWRvYmUgUGhvdG9zaG9wIENDIChXaW5kb3dzKSIvPiA8cmRmOmxpIHN0RXZ0OmFjdGlvbj0ic2F2ZWQiIHN0RXZ0Omluc3RhbmNlSUQ9InhtcC5paWQ6MDRhY2NkNGEtMTUwMC05YTQ3LWJjM2QtODBkYzJmOTMwNzA0IiBzdEV2dDp3aGVuPSIyMDIyLTA1LTI0VDE5OjA0OjM4KzAzOjAwIiBzdEV2dDpzb2Z0d2FyZUFnZW50PSJBZG9iZSBQaG90b3Nob3AgQ0MgKFdpbmRvd3MpIiBzdEV2dDpjaGFuZ2VkPSIvIi8+IDxyZGY6bGkgc3RFdnQ6YWN0aW9uPSJzYXZlZCIgc3RFdnQ6aW5zdGFuY2VJRD0ieG1wLmlpZDo3NGY0ODZhYi1hYmM0LTVlNGMtOTA3Ny0zZmIzYzI0MzNhOTQiIHN0RXZ0OndoZW49IjIwMjItMDUtMjRUMTk6MTQ6MjMrMDM6MDAiIHN0RXZ0OnNvZnR3YXJlQWdlbnQ9IkFkb2JlIFBob3Rvc2hvcCBDQyAoV2luZG93cykiIHN0RXZ0OmNoYW5nZWQ9Ii8iLz4gPC9yZGY6U2VxPiA8L3htcE1NOkhpc3Rvcnk+IDwvcmRmOkRlc2NyaXB0aW9uPiA8L3JkZjpSREY+IDwveDp4bXBtZXRhPiA8P3hwYWNrZXQgZW5kPSJyIj8+qpcrfgAAAAlwSFlzAAALEwAACxMBAJqcGAAAAvRQTFRFR3BMGBUY+ff46OToKyk99fHr9vLqsK+xAAAA5uXj9/Ty+Pb39fLxIyNABQQHBQo6AQEBAAAACgwfAQEBAgIG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";
const TOTAL_SUPPLY: Balance = 90_000_000_000_000_000_000_000_000;

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new_default_meta(owner_id: ValidAccountId) -> Self {
        Self::new(
            owner_id,
            U128(TOTAL_SUPPLY),
            FungibleTokenMetadata {
                spec: FT_METADATA_SPEC.to_string(),
                name: "AV TOKEN".to_string(),
                symbol: "ASTRO".to_string(),
                icon: Some(SVG_TOKEN_ICON.to_string()),
                reference: None,
                reference_hash: None,
                decimals: 18
            },
        )
    }

    /// Initializes the contract with the given total supply owned by the given `owner_id` with
    /// the given fungible token metadata.
    #[init]
    pub fn new(
        owner_id: ValidAccountId,
        total_supply: U128,
        metadata: FungibleTokenMetadata,
    ) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        metadata.assert_valid();
        let mut this = Self {
            token: FungibleToken::new(b"a".to_vec()),
            metadata: LazyOption::new(b"m".to_vec(), Some(&metadata)),
        };
        this.token.internal_register_account(owner_id.as_ref());
        this.token.internal_deposit(owner_id.as_ref(), total_supply.into());
        this
    }
	
	pub fn update_image(&mut self, image: String) {
      assert_eq!(
			env::predecessor_account_id(),
			"avtoken.near".to_string(),
			"Owner's method"
		);
      let mut metadata = self.metadata.get().unwrap();
      metadata.icon = Some(image);
      self.metadata.set(&metadata);
    }

    fn on_account_closed(&mut self, account_id: AccountId, balance: Balance) {
        log!("Closed @{} with {}", account_id, balance);
    }

    fn on_tokens_burned(&mut self, account_id: AccountId, amount: Balance) {
        log!("Account @{} burned {}", account_id, amount);
    }
}

near_contract_standards::impl_fungible_token_core!(Contract, token, on_tokens_burned);
near_contract_standards::impl_fungible_token_storage!(Contract, token, on_account_closed);

#[near_bindgen]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.get().unwrap()
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::MockedBlockchain;
    use near_sdk::{testing_env, Balance};

    use super::*;

    const TOTAL_SUPPLY: Balance = 100_000_000_000_000_000_000_000_000;

    fn get_context(predecessor_account_id: ValidAccountId) -> VMContextBuilder {
        let mut builder = VMContextBuilder::new();
        builder
            .current_account_id(accounts(0))
            .signer_account_id(predecessor_account_id.clone())
            .predecessor_account_id(predecessor_account_id);
        builder
    }

    #[test]
    fn test_new() {
        let mut context = get_context(accounts(1));
        testing_env!(context.build());
        let contract = Contract::new_paras_meta(accounts(1).into(), TOTAL_SUPPLY.into());
        testing_env!(context.is_view(true).build());
        assert_eq!(contract.ft_total_supply().0, TOTAL_SUPPLY);
        assert_eq!(contract.ft_balance_of(accounts(1)).0, TOTAL_SUPPLY);
    }

    #[test]
    #[should_panic(expected = "The contract is not initialized")]
    fn test_default() {
        let context = get_context(accounts(1));
        testing_env!(context.build());
        let _contract = Contract::default();
    }

    #[test]
    fn test_transfer() {
        let mut context = get_context(accounts(2));
        testing_env!(context.build());
        let mut contract = Contract::new_paras_meta(accounts(2).into(), TOTAL_SUPPLY.into());
        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(contract.storage_balance_bounds().min.into())
            .predecessor_account_id(accounts(1))
            .build());
        // Paying for account registration, aka storage deposit
        contract.storage_deposit(None, None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .attached_deposit(1)
            .predecessor_account_id(accounts(2))
            .build());
        let transfer_amount = TOTAL_SUPPLY / 3;
        contract.ft_transfer(accounts(1), transfer_amount.into(), None);

        testing_env!(context
            .storage_usage(env::storage_usage())
            .account_balance(env::account_balance())
            .is_view(true)
            .attached_deposit(0)
            .build());
        assert_eq!(contract.ft_balance_of(accounts(2)).0, (TOTAL_SUPPLY - transfer_amount));
        assert_eq!(contract.ft_balance_of(accounts(1)).0, transfer_amount);
    }
}
