load-declarations:
	ingestion_key=$$(python3 -c "import uuid; print(uuid.uuid7())") && \
	cargo run -- data util copy-latest-raw --config exposed/config/cli-dev.yaml --ingestion-key "$$ingestion_key" && \
	cargo run -- data declarations clean --config exposed/config/cli-dev.yaml --ingestion-key "$$ingestion_key" && \
	cargo run -- data declarations resolve --config exposed/config/declaration-resolution-dev.yaml --ingestion-key "$$ingestion_key" && \
	cargo run -- data declarations load --config exposed/config/cli-dev.yaml --ingestion-key "$$ingestion_key" 
