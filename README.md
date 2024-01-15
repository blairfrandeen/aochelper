# aochelper

Utility to download Advent of Code puzzle inputs.

## Installation
```sh
cargo install aochelper
```

## Setup
Run 
```sh
aochelper set year <year>
```
in the directory in which you'll be working to set the puzzle year.

The authentication is automatic if you log into Advent of Code with Snap-installed Firefox on Linux. Otherwise, get your Advent of Code session cookie from your browser of choice, and run

```sh
aochelper set session_key <your key here>
```

## Usage
To download a puzzle for a given day:
```sh
aochelper get <day>
```
This will download the puzzle inputs to _inputs/year.day_.

## Troubleshooting
Works on my machine!
