# Architecture overview

Diagrams exist for this project `exposed architecture` excalidraw board. 

The `data latest` CLI command constructs `IngestionStatusService` with the
filesystem adapter `ExposedIngestionCatalog`. The service selects the latest run
and each dataset's furthest stage through the domain-owned `IngestionCatalog`
port. The adapter reads file names and modification times. The CLI formats the
result, and the command does not change stored data.
