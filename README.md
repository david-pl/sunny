## sunny

Simple app to gather power data for my photovoltaic cells


## Running:

```bash
$ ./sunny --help
Usage: sunny [OPTIONS] --granularity <GRANULARITY> --url <URL> --db-path <DB_PATH>

Options:
  -g, --granularity <GRANULARITY>        
  -b, --bind <BIND>                      [default: 0.0.0.0:3000]
      --url <URL>                        
      --db-path <DB_PATH>                
      --segment-size <SEGMENT_SIZE>      [default: 100]
      --loss-threshold <LOSS_THRESHOLD>  [default: 10]
  -h, --help                             Print help
```

```bash
./sunny -g 60 --db-path db-test --url <local-network-address-of-inverter> 
```
