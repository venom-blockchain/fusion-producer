<p align="center">
  <a href="https://github.com/venom-blockchain/developer-program">
    <img src="https://raw.githubusercontent.com/venom-blockchain/developer-program/main/vf-dev-program.png" alt="Logo" width="366.8" height="146.4">
  </a>
</p>

# venom-data-producer

The indexing infrastructure for TVM-compatible blockchains includes a node
available via [jRPC](https://github.com/broxus/everscale-jrpc) and indexer
services with some high-level APIs for each dApp we want to index. The latter
doesn’t fetch needed messages from the former.

The data producer is a software component that connects to the blockchain node
and deliver data to handlers. Resulting stream includes information
about transactions, blocks, and other relevant data from the blockchain network.

It provides three different methods of scanning blockchain data:

- `NetworkScanner` scans data from a running node. It uses Indexer to retrieve
  the blockchain data and scans the data using various network protocols, such
  as ADNL, RLD, and DHT. This
  method requires a running TON node and access to its data.

- `ArchivesScanner` scans data from local disk archives. It reads the blockchain
  data from the archive files and sends the data to handlers. This method
  requires a local copy of the blockchain archives.

- `S3Scanner` scans data from S3 storage. It reads the blockchain data from the
  specified S3 bucket and sends the data to a hand. This method requires
  access to an S3 bucket containing blockchain data.

### Runtime requirements

- CPU: 4 cores, 2 GHz
- RAM: 8 GB
- Storage: 100 GB fast SSD
- Network: 100 MBit/s

### How to run

1. Build all binaries and prepare services
   ```bash
   ./scripts/setup.sh
   ```
2. Edit `/etc/venom-data-producer/config.yaml`
3. Enable and start the service:
   ```bash
   systemctl enable venom-data-producer
   systemctl start venom-data-producer
   ```

### Config example

The example configuration includes settings that specify how the data producer should filter blockchain data. It also includes settings for the scan type, which
determines how the producer retrieves data from the TON node.

```yaml
---
# Optional states endpoint (see docs below)
rpc_config:
  # States RPC endpoint
  listen_address: "0.0.0.0:8081"
  generate_stub_keyblock: true
  # Minimal JRPC API:
  type: simple
  # # Or full JRPC API:
  # type: full
  # persistent_db_path: "/var/db/jrpc-storage"
  # # Virtual shards depth to use during shard state accounts processing
  # shard_split_depth: 4
  # # Specify options to enable the transactions GC (disabled by default)
  # transactions_gc_options:
  #   # For at least how long to store transactions (in seconds)
  #   ttl_sec: 1209600
  #   # GC invocation interval (in seconds)
  #   interval_sec: 3600

metrics_settings:
  # Listen address of metrics. Used by the client to gather prometheus metrics.
  # Default: "127.0.0.1:10000"
  listen_address: "0.0.0.0:10000"
  # Metrics update interval in seconds. Default: 10
  collection_interval_sec: 10

# # Scan from local archives
# scan_type:
#   kind: FromArchives
#   # Example how to prepare: `find path/to/archives > path/to/archives_list`
#   list_path: path/to/archives_list

scan_type:
  kind: FromNetwork
  node_config:
    # Root directory for node DB. Default: "./db"
    db_path: "/var/db/venom-data-producer"

    # UDP port, used for ADNL node. Default: 30303
    adnl_port: 30000

    # Path to temporary ADNL keys.
    # NOTE: Will be generated if it was not there.
    # Default: "./adnl-keys.json"
    temp_keys_path: "/etc/venom-data-producer/adnl-keys.json"

    # Archives map queue. Default: 16
    parallel_archive_downloads: 32

    # archive_options:
    #   # Archives S3 uploader options
    #   uploader_options:
    #     name: ""
    #     endpoint: "http://127.0.0.1:9000"
    #     bucket: "archives"
    #     credentials:
    #       access_key: "example_key"
    #       secret_key: "example_password"

    # # Specific block from which to run the indexer
    # start_from: 12365000

    # Manual rocksdb memory options (will be computed from the
    # available memory otherwise).
    # db_options:
    #   rocksdb_lru_capacity: "512 MB"
    #   cells_cache_size: "4 GB"

    # Everscale specific network settings
    adnl_options:
      use_loopback_for_neighbours: true
      force_use_priority_channels: true
    rldp_options:
      force_compression: true
    overlay_shard_options:
      force_compression: true

# Format for data serialization
serializer:
  kind: Protobuf

# Data transfer protocol
transport:
  kind: Http2 # HTTP/2 stream
  capacity: 1024 # Channel queue capacity
  listen_address: 127.0.0.1:10002 # Listen address for HTTP/2 server

# Data filtering configuration
filter_config:
  # Blockchain message filters
  message_filters:
  # There are 3 message filter types: contract, native_transfer and any_message
  # native_transfer allows only outbound venom native transfer with empty message body
  - type: native_transfer
    # entries denotes a set of filters of the same type
    entries:
      # A custom name for a message, which will be sent to the consumer
      - name: Venom transfer
        # Allows to filter by specific sender (by address or code hash)
        sender:
          address: 0:...
        # Filters by a specific message receiver
        receiver:
          address: 0:...
        # messages field is only relevant for contract filters 
        messages: []
  # contract filter allows to filter messages using a custom contract ABI
  - type:
      # When using a contract filter, we need to specify additional parameters
      contract:
        # A custom name to send to the consumer with a message payload
        name: TokenWallet
        # A path to contract's ABI file
        abi_path: ./test/abi/TokenWallet.abi.json
    entries:
    # Custom user name
    - name: TIP-3 transfer
      messages:
      # ABI name of the message operation
      - name: acceptTransfer
        # One of the: internal, external_inbound, external_outbound
        # internal - message between contracts
        # external_inbound - user transaction message
        # external_outbound - contract event
        type: internal
  # any_messages filter passes any message, additional receiver or sender filtering is advised
  - type: any_message
    entries:
      - name: fallthrough
        receiver:
          # Filter by account's code hash
          code_hash: 3ba6528ab2694c118180aa3bd10dd19ff400b909ab4dcf58fc69925b2c7b12a6
        messages: []
```

## Contributing

We welcome contributions to the project! If you notice any issues or errors, feel free to open an issue or submit a pull request.

## License

Licensed under GPL-3.0 license ([LICENSE](/LICENSE) or https://opensource.org/license/gpl-3-0/).
