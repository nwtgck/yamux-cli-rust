# yamux
Multiplexing TCP connection CLI using [yamux](https://github.com/hashicorp/yamux/blob/master/spec.md)

## Usage

### TCP

```bash
... | yamux localhost 80 | ...
```

```bash
... | yamux -l 8080 | ...
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
