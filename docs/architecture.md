# Architecture overview

Diagrams exist for this project `exposed architecture` excalidraw board. 

The `data latest` CLI command calls `ExposedDataPipeline::latest_ingestion`
directly. The filesystem struct selects the most recently modified run
directory and returns its absolute path and ingestion key. The CLI formats the
result. This operation does not change stored data.
