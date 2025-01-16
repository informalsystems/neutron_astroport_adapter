compile: WORK_DIR=$(CURDIR)
compile: compile-inner

compile-inner:
	docker run --rm -v "$(WORK_DIR)":/code \
		--mount type=volume,source="$(notdir $(WORK_DIR))_cache",target=/target \
		--mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
		cosmwasm/optimizer:0.16.0

NEUTROND := neutrond
KEY := submitter
CHAIN_ID := neutron-1
NODE := https://neutron-rpc.polkachu.com:443
FEES := 20000untrn

store:
	$(NEUTROND) tx wasm store artifacts/neutron_astroport_adapter.wasm --from $(KEY) --gas 2000000 --chain-id $(CHAIN_ID) --node $(NODE) --keyring-backend test -y --fees $(FEES)

instantiate:
	@echo ">>> Storing contract code..."
	@$(NEUTROND) tx wasm store artifacts/neutron_astroport_adapter.wasm \
	  --from $(KEY) --gas 2000000 --chain-id $(CHAIN_ID) --node $(NODE) \
	  --keyring-backend test -y --fees $(FEES) -o json \
	  | tee store_tx.json

	@echo ">>> Extracting store TX hash..."
	@store_tx_hash=$$(jq -r '.txhash' store_tx.json); \
	 echo "$$store_tx_hash" > store_tx_hash.txt

	@echo ">>> Waiting for the store transaction to be indexed..."
	@sleep 10

	@echo ">>> Querying the chain for the store transaction..."
	@store_tx_hash=$$(cat store_tx_hash.txt); \
	 $(NEUTROND) query tx "$$store_tx_hash" --chain-id $(CHAIN_ID) --node $(NODE) -o json \
	  | tee store_query.json

	@echo ">>> Extracting code_id from store transaction events..."
	@code_id=$$(cat store_query.json \
	  | jq -r '.events[] | select(.type == "store_code").attributes[] | select(.key == "code_id").value'); \
	 echo "Code ID: $$code_id" && echo "$$code_id" > code_id.txt;

	@echo ">>> Instantiating the contract..."
	@code_id=$$(cat code_id.txt); \
	 $(NEUTROND) tx wasm instantiate $$code_id \
	   '{"astroport_incentive_contract": "neutron173fd8wpfzyqnfnpwq2zhtgdstujrjz2wkprkjfr6gqg4gknctjyq6m3tch","concentrated_pool_address":"neutron1yem82r0wf837lfkwvcu2zxlyds5qrzwkz8alvmg0apyrjthk64gqeq2e98"}' \
	   --label "astroportAdapter" --no-admin \
	   --from $(KEY) --gas 2000000 --chain-id $(CHAIN_ID) --node $(NODE) \
	   --keyring-backend test -y --fees $(FEES) -o json \
	  | tee instantiate_tx.json

	@echo ">>> Extracting instantiate TX hash..."
	@instantiate_tx_hash=$$(jq -r '.txhash' instantiate_tx.json); \
	 echo "$$instantiate_tx_hash" > instantiate_tx_hash.txt

	@echo ">>> Waiting for the instantiate transaction to be indexed..."
	@sleep 5

	@echo ">>> Querying the chain for the instantiate transaction..."
	@instantiate_tx_hash=$$(cat instantiate_tx_hash.txt); \
	 $(NEUTROND) query tx "$$instantiate_tx_hash" --chain-id $(CHAIN_ID) --node $(NODE) -o json \
	  | tee instantiate_query.json

	@echo ">>> Extracting contract address from instantiate transaction events..."
	@contract_addr=$$(cat instantiate_query.json \
	  | jq -r '.events[] | select(.type == "instantiate").attributes[] | select(.key == "_contract_address").value'); \
	 echo "Contract Address: $$contract_addr" && echo "$$contract_addr" > contract_address.txt;

	@echo ">>> Done!"
	@echo " - Code ID stored in code_id.txt"
	@echo " - Contract address stored in contract_address.txt"