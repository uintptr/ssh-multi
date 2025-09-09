# sshmulti

Use multiple IP/Host addresses to reach the same SSH server. Useful when using
an optional VPN server depending on location ( home vs office )

## Install

```
cargo install --git https://github.com/uintptr/ssh-multi
```

## Use

```
Host vps
    ProxyCommand    ssh-multi 10.0.0.2 10.0.2.2
    User            joe
```
