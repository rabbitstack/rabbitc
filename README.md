# rabbitc

**rabbitc** is the micro container runtime meant for learning purposes. For more information, read the blog [post](http://rabbitstack.github.io/operating%20systems/containers/linux-container-internals-part-ii/).

## Building

Rust toolchain is required to build `rabbitc`. Clone this repository and run `cargo build --release`.

`rabbitc --help` prints all availalbe options.

```bash
OPTIONS:
    -c, --cmd <cmd>                      Command that is run inside container [default: /bin/sh]
    -t, --container-ip <container-ip>    The default IP address for container in CIDR notation [default: 172.19.0.2/16]
    -h, --hostname <hostname>            Container host name [default: rabbitc]
    -i, --network-ip <network-ip>        The default IP address for the bridge device in CIDR notation [default:
                                         172.19.0.1/16]
    -n, --network-name <network-name>    The name of the bridge device where containers are connected [default:
                                         rabbitc0]
    -r, --rootfs <rootfs>                Root file system path for the container
```
