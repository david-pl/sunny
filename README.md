## sunny

Simple app to gather power data for my photovoltaic cells


## Running:

```bash
$ ./sunny --help
Usage: sunny [OPTIONS] --granularity <GRANULARITY> --url <URL> --sunny-home <SUNNY_HOME>

Options:
  -g, --granularity <GRANULARITY>        
  -b, --bind <BIND>                      [default: 0.0.0.0:3000]
      --url <URL>                        
      --sunny-home <SUNNY_HOME>          
      --segment-size <SEGMENT_SIZE>      [default: 100]
      --loss-threshold <LOSS_THRESHOLD>  [default: 10]
  -h, --help                             Print help
```

```bash
./sunny -g 60 --sunny-path /home/ubuntu/sunny/ --url <local-network-address-of-inverter> 
```
