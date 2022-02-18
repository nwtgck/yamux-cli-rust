# yamux
[![CI](https://github.com/nwtgck/yamux-cli-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/nwtgck/yamux-cli-rust/actions/workflows/ci.yml)

CLI of multiplexing TCP and UDP using [yamux](https://github.com/hashicorp/yamux/blob/master/spec.md)

### Linux

```bash
curl -L https://github.com/nwtgck/yamux-cli-rust/releases/download/v0.3.0/yamux-x86_64-unknown-linux-musl.tar.gz | tar xzf -
./yamux-x86_64-unknown-linux-musl/yamux --help
```

### macOS (Intel)

```bash
curl -L https://github.com/nwtgck/yamux-cli-rust/releases/download/v0.3.0/yamux-x86_64-apple-darwin.tar.gz | tar xzf -
./yamux-x86_64-apple-darwin/yamux --help
```

### macOS (Apple Silicon)

```bash
curl -L https://github.com/nwtgck/yamux-cli-rust/releases/download/v0.3.0/yamux-aarch64-apple-darwin.tar.gz | tar xzf -
./yamux-aarch64-apple-darwin/yamux --help
```

Other binaries are found in <https://github.com/nwtgck/yamux-cli-rust/releases>.

## Usage

### TCP

```bash
... | yamux localhost 80 | ...
```

```bash
... | yamux -l 8080 | ...
```

### Unix domain socket

```bash
... | yamux -U /unix/domain/socket/path | ...
```

```bash
... | yamux -U -l /unix/domain/socket/path | ...
```

### UDP

```bash
... | yamux -u 1.1.1.1 53 | ...
```

```bash
... | yamux -ul 1053 | ...
```

## Complete example

Here is a complete simple example, but not useful. This is forwarding local 80 port to local 8080 port.

```bash
mkfifo my_pipe
cat my_pipe | yamux localhost 80 | yamux -l 8080 > ./my_pipe 
```

An expected usage of this CLI is to combine network tools and transport a remote port.

## Port forwarding over NAT example

Here is more practical usage. The commands below forward 22 port in machine A to 2222 port in another machine B.

```bash
# Machine A
curl -sSN https://ppng.io/aaa | yamux localhost 22 | curl -sSNT - https://ppng.io/bbb
```

```bash
# Machine B
curl -sSN https://ppng.io/bbb | yamux -l 2222 | curl -sSNT - https://ppng.io/aaa
```

`https://ppng.io` is [Piping Server](https://github.com/nwtgck/piping-server), which streams data over pure HTTP. 

## Go implementation
<https://github.com/nwtgck/yamux-cli>
